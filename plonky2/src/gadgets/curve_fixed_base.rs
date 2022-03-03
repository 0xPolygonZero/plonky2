use num::BigUint;
use plonky2_field::extension_field::Extendable;

use crate::curve::curve_types::{AffinePoint, Curve, CurveScalar};
use crate::field::field_types::Field;
use crate::gadgets::curve::AffinePointTarget;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::hash::hash_types::RichField;
use crate::hash::keccak::KeccakHash;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::{GenericHashOut, Hasher};

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Do windowed fixed-base scalar multiplication, using a 4-bit window.
    pub fn fixed_base_curve_mul<C: Curve>(
        &mut self,
        base: AffinePoint<C>,
        scalar: &NonNativeTarget<C::ScalarField>,
    ) -> AffinePointTarget<C> {
        // Holds `(16^i) * base` for `i=0..scalar.value.limbs.len() * 8`.
        let scaled_base = (0..scalar.value.limbs.len() * 8).scan(base, |acc, _| {
            let tmp = *acc;
            for _ in 0..4 {
                *acc = acc.double();
            }
            Some(tmp)
        });

        let limbs = self.split_nonnative_to_4_bit_limbs(scalar);

        let hash_0 = KeccakHash::<32>::hash_no_pad(&[F::ZERO]);
        let hash_0_scalar = C::ScalarField::from_biguint(BigUint::from_bytes_le(
            &GenericHashOut::<F>::to_bytes(&hash_0),
        ));
        let rando = (CurveScalar(hash_0_scalar) * C::GENERATOR_PROJECTIVE).to_affine();

        let zero = self.zero();
        let mut result = self.constant_affine_point(rando);
        // `s * P = sum s_i * P_i` with `P_i = (16^i) * P` and `s = sum s_i * (16^i)`.
        for (limb, point) in limbs.into_iter().zip(scaled_base) {
            // Holds `t * P_i` for `p=0..16`.
            let muls_point = (0..16)
                .scan(AffinePoint::ZERO, |acc, _| {
                    let tmp = *acc;
                    *acc = (point + *acc).to_affine();
                    Some(tmp)
                })
                .map(|p| self.constant_affine_point(p))
                .collect::<Vec<_>>();
            let is_zero = self.is_equal(limb, zero);
            let should_add = self.not(is_zero);
            // `r = s_i * P_i`
            let r = self.random_access_curve_points(limb, muls_point);
            result = self.curve_conditional_add(&result, &r, should_add);
        }

        let to_add = self.constant_affine_point(-rando);
        self.curve_add(&result, &to_add)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::field_types::PrimeField;
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::secp256k1::Secp256K1;
    use crate::field::field_types::Field;
    use crate::iop::witness::{PartialWitness, Witness};
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_fixed_base() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_ecc_config();

        let mut pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = Secp256K1::GENERATOR_AFFINE;
        let n = Secp256K1Scalar::rand();

        let res = (CurveScalar(n) * g.to_projective()).to_affine();
        let res_expected = builder.constant_affine_point(res);
        builder.curve_assert_valid(&res_expected);

        let n_target = builder.add_virtual_nonnative_target::<Secp256K1Scalar>();
        pw.set_biguint_target(&n_target.value, &n.to_canonical_biguint());

        let res_target = builder.fixed_base_curve_mul(g, &n_target);
        builder.curve_assert_valid(&res_target);

        builder.connect_affine_point(&res_target, &res_expected);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }
}
