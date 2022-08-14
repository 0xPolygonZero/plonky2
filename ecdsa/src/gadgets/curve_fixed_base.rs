use num::BigUint;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::keccak::KeccakHash;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{GenericHashOut, Hasher};
use plonky2_field::extension::Extendable;
use plonky2_field::types::Field;

use crate::curve::curve_types::{AffinePoint, Curve, CurveScalar};
use crate::gadgets::curve::{AffinePointTarget, CircuitBuilderCurve};
use crate::gadgets::curve_windowed_mul::CircuitBuilderWindowedMul;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::gadgets::split_nonnative::CircuitBuilderSplit;

/// Compute windowed fixed-base scalar multiplication, using a 4-bit window.
pub fn fixed_base_curve_mul_circuit<C: Curve, F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
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

    let limbs = builder.split_nonnative_to_4_bit_limbs(scalar);

    let hash_0 = KeccakHash::<32>::hash_no_pad(&[F::ZERO]);
    let hash_0_scalar = C::ScalarField::from_biguint(BigUint::from_bytes_le(
        &GenericHashOut::<F>::to_bytes(&hash_0),
    ));
    let rando = (CurveScalar(hash_0_scalar) * C::GENERATOR_PROJECTIVE).to_affine();

    let zero = builder.zero();
    let mut result = builder.constant_affine_point(rando);
    // `s * P = sum s_i * P_i` with `P_i = (16^i) * P` and `s = sum s_i * (16^i)`.
    for (limb, point) in limbs.into_iter().zip(scaled_base) {
        // `muls_point[t] = t * P_i` for `t=0..16`.
        let mut muls_point = (0..16)
            .scan(AffinePoint::ZERO, |acc, _| {
                let tmp = *acc;
                *acc = (point + *acc).to_affine();
                Some(tmp)
            })
            // First element if zero, so we skip it since `constant_affine_point` takes non-zero input.
            .skip(1)
            .map(|p| builder.constant_affine_point(p))
            .collect::<Vec<_>>();
        // We add back a point in position 0. `limb == zero` is checked below, so this point can be arbitrary.
        muls_point.insert(0, muls_point[0].clone());
        let is_zero = builder.is_equal(limb, zero);
        let should_add = builder.not(is_zero);
        // `r = s_i * P_i`
        let r = builder.random_access_curve_points(limb, muls_point);
        result = builder.curve_conditional_add(&result, &r, should_add);
    }

    let to_add = builder.constant_affine_point(-rando);
    builder.curve_add(&result, &to_add)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;
    use plonky2_field::types::Field;
    use plonky2_field::types::PrimeField;

    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::secp256k1::Secp256K1;
    use crate::gadgets::biguint::witness_set_biguint_target;
    use crate::gadgets::curve::CircuitBuilderCurve;
    use crate::gadgets::curve_fixed_base::fixed_base_curve_mul_circuit;
    use crate::gadgets::nonnative::CircuitBuilderNonNative;

    #[test]
    #[ignore]
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
        witness_set_biguint_target(&mut pw, &n_target.value, &n.to_canonical_biguint());

        let res_target = fixed_base_curve_mul_circuit(&mut builder, g, &n_target);
        builder.curve_assert_valid(&res_target);

        builder.connect_affine_point(&res_target, &res_expected);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }
}
