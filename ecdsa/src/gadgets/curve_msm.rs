use num::BigUint;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::keccak::KeccakHash;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{GenericHashOut, Hasher};
use plonky2_field::extension::Extendable;
use plonky2_field::types::Field;

use crate::curve::curve_types::{Curve, CurveScalar};
use crate::gadgets::curve::{AffinePointTarget, CircuitBuilderCurve};
use crate::gadgets::curve_windowed_mul::CircuitBuilderWindowedMul;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::gadgets::split_nonnative::CircuitBuilderSplit;

/// Computes `n*p + m*q` using windowed MSM, with a 2-bit window.
/// See Algorithm 9.23 in Handbook of Elliptic and Hyperelliptic Curve Cryptography for a
/// description.
/// Note: Doesn't work if `p == q`.
pub fn curve_msm_circuit<C: Curve, F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    p: &AffinePointTarget<C>,
    q: &AffinePointTarget<C>,
    n: &NonNativeTarget<C::ScalarField>,
    m: &NonNativeTarget<C::ScalarField>,
) -> AffinePointTarget<C> {
    let limbs_n = builder.split_nonnative_to_2_bit_limbs(n);
    let limbs_m = builder.split_nonnative_to_2_bit_limbs(m);
    assert_eq!(limbs_n.len(), limbs_m.len());
    let num_limbs = limbs_n.len();

    let hash_0 = KeccakHash::<32>::hash_no_pad(&[F::ZERO]);
    let hash_0_scalar = C::ScalarField::from_noncanonical_biguint(BigUint::from_bytes_le(
        &GenericHashOut::<F>::to_bytes(&hash_0),
    ));
    let rando = (CurveScalar(hash_0_scalar) * C::GENERATOR_PROJECTIVE).to_affine();
    let rando_t = builder.constant_affine_point(rando);
    let neg_rando = builder.constant_affine_point(-rando);

    // Precomputes `precomputation[i + 4*j] = i*p + j*q` for `i,j=0..4`.
    let mut precomputation = vec![p.clone(); 16];
    let mut cur_p = rando_t.clone();
    let mut cur_q = rando_t.clone();
    for i in 0..4 {
        precomputation[i] = cur_p.clone();
        precomputation[4 * i] = cur_q.clone();
        cur_p = builder.curve_add(&cur_p, p);
        cur_q = builder.curve_add(&cur_q, q);
    }
    for i in 1..4 {
        precomputation[i] = builder.curve_add(&precomputation[i], &neg_rando);
        precomputation[4 * i] = builder.curve_add(&precomputation[4 * i], &neg_rando);
    }
    for i in 1..4 {
        for j in 1..4 {
            precomputation[i + 4 * j] =
                builder.curve_add(&precomputation[i], &precomputation[4 * j]);
        }
    }

    let four = builder.constant(F::from_canonical_usize(4));

    let zero = builder.zero();
    let mut result = rando_t;
    for (limb_n, limb_m) in limbs_n.into_iter().zip(limbs_m).rev() {
        result = builder.curve_repeated_double(&result, 2);
        let index = builder.mul_add(four, limb_m, limb_n);
        let r = builder.random_access_curve_points(index, precomputation.clone());
        let is_zero = builder.is_equal(index, zero);
        let should_add = builder.not(is_zero);
        result = builder.curve_conditional_add(&result, &r, should_add);
    }
    let starting_point_multiplied = (0..2 * num_limbs).fold(rando, |acc, _| acc.double());
    let to_add = builder.constant_affine_point(-starting_point_multiplied);
    result = builder.curve_add(&result, &to_add);

    result
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

    use crate::curve::curve_types::{Curve, CurveScalar};
    use crate::curve::secp256k1::Secp256K1;
    use crate::gadgets::curve::CircuitBuilderCurve;
    use crate::gadgets::curve_msm::curve_msm_circuit;
    use crate::gadgets::nonnative::CircuitBuilderNonNative;

    #[test]
    #[ignore]
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

        let res_target =
            curve_msm_circuit(&mut builder, &p_target, &q_target, &n_target, &m_target);
        builder.curve_assert_valid(&res_target);

        builder.connect_affine_point(&res_target, &res_expected);

        dbg!(builder.num_gates());
        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }
}
