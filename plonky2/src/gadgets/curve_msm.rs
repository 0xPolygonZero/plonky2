use num::BigUint;
use plonky2_field::extension_field::Extendable;

use crate::curve::curve_types::{Curve, CurveScalar};
use crate::field::field_types::Field;
use crate::gadgets::curve::AffinePointTarget;
use crate::gadgets::nonnative::NonNativeTarget;
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
        n: &NonNativeTarget<C::ScalarField>,
        m: &NonNativeTarget<C::ScalarField>,
    ) -> AffinePointTarget<C> {
        let bits_n = self.split_nonnative_to_bits(n);
        let bits_m = self.split_nonnative_to_bits(m);
        assert_eq!(bits_n.len(), bits_m.len());

        let sum = self.curve_add(p, q);
        let precomputation = vec![p.clone(), p.clone(), q.clone(), sum];

        let two = self.two();
        let hash_0 = KeccakHash::<32>::hash_no_pad(&[F::ZERO]);
        let hash_0_scalar = C::ScalarField::from_biguint(BigUint::from_bytes_le(
            &GenericHashOut::<F>::to_bytes(&hash_0),
        ));
        let starting_point = CurveScalar(hash_0_scalar) * C::GENERATOR_PROJECTIVE;
        let starting_point_multiplied =
            (0..C::ScalarField::BITS).fold(starting_point, |acc, _| acc.double());

        let zero = self.zero();
        let mut result = self.constant_affine_point(starting_point.to_affine());
        for (b_n, b_m) in bits_n.into_iter().zip(bits_m).rev() {
            result = self.curve_double(&result);
            let index = self.mul_add(two, b_m.target, b_n.target);
            let r = self.random_access_curve_points(index, precomputation.clone());
            let is_zero = self.is_equal(index, zero);
            let should_add = self.not(is_zero);
            result = self.curve_conditional_add(&result, &r, should_add);
        }
        let to_subtract = self.constant_affine_point(starting_point_multiplied.to_affine());
        let to_add = self.curve_neg(&to_subtract);
        result = self.curve_add(&result, &to_add);

        result
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Neg;

    use anyhow::Result;
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_yo() -> Result<()> {
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

        let res_target = builder.curve_msm(&p_target, &q_target, &n_target, &m_target);
        builder.curve_assert_valid(&res_target);

        builder.connect_affine_point(&res_target, &res_expected);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_ya() -> Result<()> {
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

        // let res0_target = builder.curve_scalar_mul_windowed(&p_target, &n_target);
        // let res1_target = builder.curve_scalar_mul_windowed(&q_target, &m_target);
        let res0_target = builder.curve_scalar_mul(&p_target, &n_target);
        let res1_target = builder.curve_scalar_mul(&q_target, &m_target);
        let res_target = builder.curve_add(&res0_target, &res1_target);
        builder.curve_assert_valid(&res_target);

        builder.connect_affine_point(&res_target, &res_expected);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }
}
