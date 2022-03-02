use num::BigUint;
use plonky2_field::extension_field::Extendable;

use crate::curve::curve_types::{Curve, CurveScalar};
use crate::field::field_types::Field;
use crate::gadgets::biguint::BigUintTarget;
use crate::gadgets::curve::AffinePointTarget;
use crate::hash::hash_types::RichField;
use crate::hash::keccak::KeccakHash;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::{GenericHashOut, Hasher};

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Computes `n*p + m*q`.
    pub fn curve_msm<C: Curve>(
        &mut self,
        p: &AffinePointTarget<C>,
        q: &AffinePointTarget<C>,
        n: &BigUintTarget,
        m: &BigUintTarget,
    ) -> AffinePointTarget<C> {
        let limbs_n = self.split_biguint_to_2_bit_limbs(n);
        let limbs_m = self.split_biguint_to_2_bit_limbs(m);
        assert_eq!(limbs_n.len(), limbs_m.len());
        let num_limbs = limbs_n.len();

        let hash_0 = KeccakHash::<32>::hash_no_pad(&[F::ZERO]);
        let hash_0_scalar = C::ScalarField::from_biguint(BigUint::from_bytes_le(
            &GenericHashOut::<F>::to_bytes(&hash_0),
        ));
        let rando = (CurveScalar(hash_0_scalar) * C::GENERATOR_PROJECTIVE).to_affine();
        let rando_t = self.constant_affine_point(rando);
        let neg_rando = self.constant_affine_point(-rando);

        let mut precomputation = vec![p.clone(); 16];
        let mut cur_p = rando_t.clone();
        let mut cur_q = rando_t.clone();
        for i in 0..4 {
            precomputation[i] = cur_p.clone();
            precomputation[4 * i] = cur_q.clone();
            cur_p = self.curve_add(&cur_p, p);
            cur_q = self.curve_add(&cur_q, q);
        }
        for i in 1..4 {
            precomputation[i] = self.curve_add(&precomputation[i], &neg_rando);
            precomputation[4 * i] = self.curve_add(&precomputation[4 * i], &neg_rando);
        }
        for i in 1..4 {
            for j in 1..4 {
                precomputation[i + 4 * j] =
                    self.curve_add(&precomputation[i], &precomputation[4 * j]);
            }
        }

        let four = self.constant(F::from_canonical_usize(4));

        let zero = self.zero();
        let mut result = rando_t;
        for (limb_n, limb_m) in limbs_n.into_iter().zip(limbs_m).rev() {
            result = self.curve_repeated_double(&result, 2);
            let index = self.mul_add(four, limb_m, limb_n);
            let r = self.random_access_curve_points(index, precomputation.clone());
            let is_zero = self.is_equal(index, zero);
            let should_add = self.not(is_zero);
            result = self.curve_conditional_add(&result, &r, should_add);
        }
        let starting_point_multiplied = (0..2 * num_limbs).fold(rando, |acc, _| acc.double());
        let to_add = self.constant_affine_point(-starting_point_multiplied);
        result = self.curve_add(&result, &to_add);

        result
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use anyhow::Result;
    use num::BigUint;
    use plonky2_field::secp256k1_base::Secp256K1Base;
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

    use crate::curve::curve_types::{AffinePoint, Curve, CurveScalar};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_curve_msm() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let p =
            (CurveScalar(Secp256K1Scalar::rand()) * Secp256K1::GENERATOR_PROJECTIVE).to_affine();
        let q =
            (CurveScalar(Secp256K1Scalar::rand()) * Secp256K1::GENERATOR_PROJECTIVE).to_affine();
        let n = Secp256K1Scalar::rand();
        let m = Secp256K1Scalar::rand();

        let res =
            (CurveScalar(n) * p.to_projective() + CurveScalar(m) * q.to_projective()).to_affine();
        let res_expected = builder.constant_affine_point(res);
        builder.curve_assert_valid(&res_expected);

        let p_target = builder.constant_affine_point(p);
        let q_target = builder.constant_affine_point(q);
        let n_target = builder.constant_nonnative(n);
        let m_target = builder.constant_nonnative(m);

        let res_target = builder.curve_msm(&p_target, &q_target, &n_target.value, &m_target.value);
        builder.curve_assert_valid(&res_target);

        builder.connect_affine_point(&res_target, &res_expected);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_naive_msm() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let p =
            (CurveScalar(Secp256K1Scalar::rand()) * Secp256K1::GENERATOR_PROJECTIVE).to_affine();
        let q =
            (CurveScalar(Secp256K1Scalar::rand()) * Secp256K1::GENERATOR_PROJECTIVE).to_affine();
        let n = Secp256K1Scalar::rand();
        let m = Secp256K1Scalar::rand();

        let res =
            (CurveScalar(n) * p.to_projective() + CurveScalar(m) * q.to_projective()).to_affine();
        let res_expected = builder.constant_affine_point(res);
        builder.curve_assert_valid(&res_expected);

        let p_target = builder.constant_affine_point(p);
        let q_target = builder.constant_affine_point(q);
        let n_target = builder.constant_nonnative(n);
        let m_target = builder.constant_nonnative(m);

        let res0_target = builder.curve_scalar_mul_windowed(&p_target, &n_target);
        let res1_target = builder.curve_scalar_mul_windowed(&q_target, &m_target);
        let res_target = builder.curve_add(&res0_target, &res1_target);
        builder.curve_assert_valid(&res_target);

        builder.connect_affine_point(&res_target, &res_expected);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_curve_lul() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let p = AffinePoint::<Secp256K1> {
            x: Secp256K1Base::from_biguint(
                BigUint::from_str(
                    "95702873347299649035220040874584348285675823985309557645567012532974768144045",
                )
                .unwrap(),
            ),
            y: Secp256K1Base::from_biguint(
                BigUint::from_str(
                    "34849299245821426255020320369755722155634282348110887335812955146294938249053",
                )
                .unwrap(),
            ),
            zero: false,
        };
        let q = AffinePoint::<Secp256K1> {
            x: Secp256K1Base::from_biguint(
                BigUint::from_str(
                    "66037057977021147605301350925941983227524093291368248236634649161657340356645",
                )
                .unwrap(),
            ),
            y: Secp256K1Base::from_biguint(
                BigUint::from_str(
                    "80942789991494769168550664638932185697635702317529676703644628861613896422610",
                )
                .unwrap(),
            ),
            zero: false,
        };

        let n = BigUint::from_str("89874493710619023150462632713212469930").unwrap();
        let m = BigUint::from_str("76073901947022186525975758425319149118").unwrap();

        let res = (CurveScalar(Secp256K1Scalar::from_biguint(n.clone())) * p.to_projective()
            + CurveScalar(Secp256K1Scalar::from_biguint(m.clone())) * q.to_projective())
        .to_affine();
        let res_expected = builder.constant_affine_point(res);
        builder.curve_assert_valid(&res_expected);

        let p_target = builder.constant_affine_point(p);
        let q_target = builder.constant_affine_point(q);
        let n_target = builder.constant_biguint(&n);
        let m_target = builder.constant_biguint(&m);

        let res_target = builder.curve_msm(&p_target, &q_target, &n_target, &m_target);
        builder.curve_assert_valid(&res_target);

        builder.connect_affine_point(&res_target, &res_expected);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }
}
