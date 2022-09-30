//! Support for the EVM ADDMOD instruction.
//!
//! This crate verifies an EVM ADDMOD instruction, which takes three
//! 256-bit inputs A, B and M, and produces a 256-bit output C satisfying
//!
//!    C = operation(A, B) (mod M).
//!
//! where operation can be addition, multiplication, or just return
//! the first argument.  Inputs A, B and M, and output C, are given as
//! arrays of 16-bit limbs. For example, if the limbs of A are
//! a[0]...a[15], then
//!
//!    A = \sum_{i=0}^15 a[i] β^i,
//!
//! where β = 2^16. To verify that A, B, M and C satisfy the equation we
//! proceed as follows. Define a(x) = \sum_{i=0}^15 a[i] x^i (so A = a(β))
//! and similarly for b(x), m(x) and c(x). Then operation(A,B) = C (mod M)
//! if and only if there exist polynomials q and s such that
//!
//!    operation(a(x), b(x)) - c(x) - m(x)*s(x) - (x - β)*q(x) == 0.
//!
//! Because A, B, M and C are 256-bit numbers, the degrees of a, b, m
//! and c are (at most) 15. On the other hand, the deg(m) can be 0, in
//! which case deg(s) may need to be 15, so in general we need to
//! accommodate deg(m*s) <= 30. Hence deg(q) can be up to 29.
//!
//! TODO: Write up analysis of degrees of the polynomials and the
//! bounds on their coefficients.

use num::BigUint;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use super::columns;
use crate::arithmetic::columns::*;
use crate::arithmetic::utils::{
    pol_add, pol_add_assign, pol_add_circuit, pol_adjoin_root, pol_extend, pol_extend_circuit,
    pol_mul_wide, pol_mul_wide2, pol_mul_wide_circuit, pol_remove_root_2exp, pol_sub_assign,
    pol_sub_assign_circuit,
};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

/// Convert the base-2^16 representation into a base-2^32 representation
fn columns_to_biguint<const N: usize>(limbs: &[i64; N]) -> BigUint {
    const BASE: i64 = 1i64 << LIMB_BITS;

    // Although the input type is i64, the values must always be in
    // [0, 2^16 + ε) because of the caller's range check on the inputs
    // (the ε allows us to convert calculated output, which can be
    // bigger than 2^16).
    debug_assert!(limbs.iter().all(|&x| x >= 0));

    let mut limbs_u32 = Vec::with_capacity(N / 2 + 1);
    let mut cy = 0i64; // cy is necessary to handle ε > 0
    for i in 0..(N / 2) {
        let t = cy + limbs[2 * i] + BASE * limbs[2 * i + 1];
        limbs_u32.push(t as u32);
        cy = t >> 32;
    }
    if N & 1 != 0 {
        // If N is odd we need to add the last limb on its own
        let t = cy + limbs[N - 1];
        limbs_u32.push(t as u32);
        cy = t >> 32;
    }
    limbs_u32.push(cy as u32);

    BigUint::from_slice(&limbs_u32)
}

/// Convert the base-2^32 representation into a base-2^16 representation
fn biguint_to_columns<const N: usize>(num: &BigUint) -> [i64; N] {
    assert!(num.bits() <= 16 * N as u64);
    let mut output = [0i64; N];
    for (i, limb) in num.iter_u32_digits().enumerate() {
        output[2 * i] = limb as u16 as i64;
        output[2 * i + 1] = (limb >> LIMB_BITS) as i64;
    }
    output
}

fn generate_modular_op<F: RichField>(
    lv: &mut [F; NUM_ARITH_COLUMNS],
    operation: fn([i64; N_LIMBS], [i64; N_LIMBS]) -> [i64; 2 * N_LIMBS - 1],
) {
    // Inputs are all range-checked in [0, 2^16), so the "as i64"
    // conversion is safe.
    let input0_limbs = ADDMOD_INPUT_0.map(|c| F::to_canonical_u64(&lv[c]) as i64);
    let input1_limbs = ADDMOD_INPUT_1.map(|c| F::to_canonical_u64(&lv[c]) as i64);
    let modulus_limbs = ADDMOD_MODULUS.map(|c| F::to_canonical_u64(&lv[c]) as i64);

    // The use of BigUints is entirely to avoid having to implement
    // modular reduction.
    let modulus = columns_to_biguint(&modulus_limbs);

    // constr_poly is initialised to the calculated input, and is
    // used as such for the BigUint reduction; later, other values are
    // added/subtracted, which is where its meaning as the "constraint
    // polynomial" comes in.

    let mut constr_poly = [0i64; 2 * N_LIMBS];
    constr_poly[..2 * N_LIMBS - 1].copy_from_slice(&operation(input0_limbs, input1_limbs));

    let input = columns_to_biguint(&constr_poly);

    // Assumes modulus != 0
    let res = &input % &modulus;
    let output_limbs = biguint_to_columns::<N_LIMBS>(&res);
    let lambda = (input - &res) / &modulus; // exact division
    let quot_limbs = biguint_to_columns::<{ 2 * N_LIMBS }>(&lambda);

    // TODO: explain the mapping between a, b, c, etc. and the
    // variable names used!

    // constr_poly is the array of coefficients of the polynomial
    //
    //   operation(a(x), b(x)) - c(x) - s(x)*m(x).
    //
    pol_sub_assign(&mut constr_poly, &output_limbs);
    let prod = pol_mul_wide2(quot_limbs, modulus_limbs);
    pol_sub_assign(&mut constr_poly, &prod[0..2 * N_LIMBS]);

    // Higher order terms must be zero for valid quot and modulus:
    debug_assert!(&prod[2 * N_LIMBS..].iter().all(|&x| x == 0i64));

    // constr_poly must be zero when evaluated at x = β := 2^LIMB_BITS,
    // hence it's divisible by (x - β). If we write it as
    //
    //   operation(a(x), b(x)) - c(x) - s(x)*m(x)
    //       = \sum_{i=0}^n p_i x^i
    //       = (x - β) \sum_{i=0}^{n-1} q_i x^i
    //
    // then by comparing coefficients it is easy to see that
    //
    //   q_0 = -p_0 / β  and  q_i = (q_{i-1} - p_i) / β
    //
    // for 0 < i <= n-1 (and the divisions are exact).

    let aux_limbs = pol_remove_root_2exp::<LIMB_BITS, _>(constr_poly);

    for deg in 0..N_LIMBS {
        lv[ADDMOD_OUTPUT[deg]] = F::from_canonical_i64(output_limbs[deg]);
        lv[ADDMOD_QUO_INPUT[deg]] = F::from_canonical_i64(quot_limbs[deg]);
        lv[ADDMOD_QUO_INPUT[deg + N_LIMBS]] = F::from_canonical_i64(quot_limbs[deg + N_LIMBS]);
        lv[ADDMOD_AUX_INPUT[deg]] = F::from_canonical_i64(aux_limbs[deg]);
        lv[ADDMOD_AUX_INPUT[deg + N_LIMBS]] = F::from_canonical_i64(aux_limbs[deg + N_LIMBS]);
    }
}

pub(crate) fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS], filter: usize) {
    match filter {
        columns::IS_ADDMOD => generate_modular_op(lv, pol_add),
        columns::IS_MULMOD => generate_modular_op(lv, pol_mul_wide),
        columns::IS_MOD => generate_modular_op(lv, |a, _| pol_extend(a)),
        _ => panic!("generate modular operation called with unknown opcode"),
    }
}

#[allow(clippy::needless_range_loop)]
fn modular_constr_poly<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
) -> [P; 2 * N_LIMBS] {
    range_check_error!(ADDMOD_INPUT_0, 16);
    range_check_error!(ADDMOD_INPUT_1, 16);
    range_check_error!(ADDMOD_MODULUS, 16);
    range_check_error!(ADDMOD_QUO_INPUT, 16);
    range_check_error!(ADDMOD_AUX_INPUT, 20, signed);
    range_check_error!(ADDMOD_OUTPUT, 16);

    // FIXME: This code assumes that the modulus is greater than zero, and
    // that the output is less than the modulus.

    let modulus = ADDMOD_MODULUS.map(|c| lv[c]);
    let output = ADDMOD_OUTPUT.map(|c| lv[c]);
    let quot = ADDMOD_QUO_INPUT.map(|c| lv[c]);
    let aux = ADDMOD_AUX_INPUT.map(|c| lv[c]);

    let prod = pol_mul_wide2(quot, modulus);
    for &x in prod[2 * N_LIMBS..].iter() {
        yield_constr.constraint(filter * x);
    }

    // Constraint poly holds the coefficients of the polynomial that
    // must be identically zero for this modular addition to be
    // verified.
    //
    // Set constr_poly[deg] to be the degree deg coefficient of the
    // polynomial operation(a(x), b(x)) - c(x) - q(x) * m(x) where
    //
    //   a(x) = \sum_i input0_limbs[i] * 2^LIMB_BITS
    //   b(x) = \sum_i input1_limbs[i] * 2^LIMB_BITS
    //   c(x) = \sum_i output_limbs[i] * 2^LIMB_BITS
    //   q(x) = \sum_i quot_limbs[i] * 2^(2*LIMB_BITS)
    //   m(x) = \sum_i modulus_limbs[i] * 2^LIMB_BITS
    //
    // This polynomial should equal (x - 2^LIMB_BITS) * s(x) where s is
    //
    //   s(x) = \sum_i aux_limbs[i] * 2^(2*LIMB_BITS - 1)
    //

    let mut constr_poly: [_; 2 * N_LIMBS] = prod[0..2 * N_LIMBS].try_into().unwrap();
    pol_add_assign(&mut constr_poly, &output);

    // Add (x - 2^LIMB_BITS) * s(x) to constr_poly.
    let base = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    pol_add_assign(&mut constr_poly, &pol_adjoin_root(aux, base));

    constr_poly
}

pub(crate) fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv[columns::IS_ADDMOD] + lv[columns::IS_MULMOD] + lv[columns::IS_MOD];
    /*
    // TODO: Do I need to check this here?
    yield_constr.constraint(filter * filter - filter);
    eval_packed_generic_lt();
    */

    // constr_poly has 2*N_LIMBS
    let constr_poly = modular_constr_poly(lv, yield_constr, filter);

    let input0 = ADDMOD_INPUT_0.map(|c| lv[c]);
    let input1 = ADDMOD_INPUT_1.map(|c| lv[c]);

    let add_input: [_; 2 * N_LIMBS - 1] = pol_add(input0, input1);
    let mul_input = pol_mul_wide(input0, input1);
    let mod_input = pol_extend(input0);

    for (input, &filter) in [
        (&add_input, &lv[columns::IS_ADDMOD]),
        (&mul_input, &lv[columns::IS_MULMOD]),
        (&mod_input, &lv[columns::IS_MOD]),
    ] {
        // At this point input holds the coefficients of the polynomial
        // operation(a(x), b(x)) - c(x) - s(x)*m(x) - (x - 2^LIMB_BITS)*q(x).
        // The modular operation is valid if and only if all of those
        // coefficients are zero.

        // Need constr_poly_copy to be the first argument to
        // pol_sub_assign, since it is the longer of the two
        // arguments.
        let mut constr_poly_copy = constr_poly;
        pol_sub_assign(&mut constr_poly_copy, input);
        for &c in constr_poly_copy.iter() {
            yield_constr.constraint(filter * c);
        }
    }
}

fn eval_ext_circuit_modular_op<F: RichField + Extendable<D>, const D: usize>(
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    builder: &mut CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    operation: fn(
        &mut CircuitBuilder<F, D>,
        [ExtensionTarget<D>; N_LIMBS],
        [ExtensionTarget<D>; N_LIMBS],
    ) -> [ExtensionTarget<D>; 2 * N_LIMBS - 1],
    filter: ExtensionTarget<D>,
) {
    let input0 = ADDMOD_INPUT_0.map(|c| lv[c]);
    let input1 = ADDMOD_INPUT_1.map(|c| lv[c]);
    let modulus = ADDMOD_MODULUS.map(|c| lv[c]);
    let output = ADDMOD_OUTPUT.map(|c| lv[c]);
    let quot = ADDMOD_QUO_INPUT.map(|c| lv[c]);
    let aux = ADDMOD_AUX_INPUT.map(|c| lv[c]);

    let mut constr_poly = operation(builder, input0, input1);

    pol_sub_assign_circuit(builder, &mut constr_poly, output);
    let tmp: [ExtensionTarget<D>; 3 * N_LIMBS - 1] = pol_mul_wide_circuit(builder, quot, modulus);
    pol_sub_assign_circuit(builder, &mut constr_poly, tmp);
    for &x in tmp[2 * N_LIMBS..].iter() {
        let t = builder.mul_extension(filter, x);
        yield_constr.constraint(builder, t);
    }

    let base = F::from_canonical_u64(1 << LIMB_BITS);
    let t = builder.mul_const_extension(base, aux[0]);
    constr_poly[0] = builder.sub_extension(constr_poly[0], t);
    for deg in 1..(2 * N_LIMBS - 1) {
        let t0 = builder.mul_const_extension(base, aux[deg]);
        let t1 = builder.sub_extension(t0, aux[deg - 1]);
        constr_poly[deg] = builder.sub_extension(constr_poly[deg], t1);
    }

    for &c in &constr_poly {
        let t = builder.mul_extension(filter, c);
        yield_constr.constraint(builder, t);
    }
}

pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_modular_op(
        lv,
        builder,
        yield_constr,
        pol_add_circuit,
        lv[columns::IS_ADDMOD],
    );
    eval_ext_circuit_modular_op(
        lv,
        builder,
        yield_constr,
        pol_mul_wide_circuit,
        lv[columns::IS_MULMOD],
    );
    eval_ext_circuit_modular_op(
        lv,
        builder,
        yield_constr,
        |bldr, a, _| pol_extend_circuit(bldr, a),
        lv[columns::IS_MOD],
    );
}

#[cfg(test)]
mod tests {
    use itertools::izip;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Field;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::*;
    use crate::arithmetic::columns::NUM_ARITH_COLUMNS;
    use crate::constraint_consumer::ConstraintConsumer;

    const N_RND_TESTS: usize = 1000;

    // TODO: Should be able to refactor this test to apply to all operations.
    #[test]
    fn generate_eval_consistency_not_addmod() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // if `IS_ADDMOD == 0`, then the constraints should be met even
        // if all values are garbage.
        lv[IS_ADDMOD] = F::ZERO;
        lv[IS_MULMOD] = F::ZERO;
        lv[IS_MOD] = F::ZERO;

        let mut constraint_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_packed_generic(&lv, &mut constraint_consumer);
        for &acc in &constraint_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency_addmod() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        for op_filter in [IS_ADDMOD, IS_MOD, IS_MULMOD] {
            // Reset operation columns, then select one
            lv[IS_ADDMOD] = F::ZERO;
            lv[IS_MULMOD] = F::ZERO;
            lv[IS_MOD] = F::ZERO;
            lv[op_filter] = F::ONE;

            for i in 0..N_RND_TESTS {
                // set inputs to random values
                for (&ai, &bi, &mi) in izip!(
                    ADDMOD_INPUT_0.iter(),
                    ADDMOD_INPUT_1.iter(),
                    ADDMOD_MODULUS.iter()
                ) {
                    lv[ai] = F::from_canonical_u16(rng.gen());
                    lv[bi] = F::from_canonical_u16(rng.gen());
                    lv[mi] = F::from_canonical_u16(rng.gen());
                }

                // For the second half of the tests, set the top 16 - start
                // digits to zero, so the modulus is much smaller than the
                // inputs.
                if i > N_RND_TESTS / 2 {
                    // 1 <= start < N_LIMBS
                    let start = (rng.gen::<usize>() % (N_LIMBS - 1)) + 1;
                    for &mi in &ADDMOD_MODULUS[start..N_LIMBS] {
                        lv[mi] = F::ZERO;
                    }
                }

                generate(&mut lv, op_filter);

                let mut constraint_consumer = ConstraintConsumer::new(
                    vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                    GoldilocksField::ONE,
                    GoldilocksField::ONE,
                    GoldilocksField::ONE,
                );
                eval_packed_generic(&lv, &mut constraint_consumer);
                for &acc in &constraint_consumer.constraint_accs {
                    assert_eq!(acc, GoldilocksField::ZERO);
                }
            }
        }
    }

    #[test]
    fn addmod_zero_modulus() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // set `IS_ADDMOD == 1` and ensure all constraints are satisfied.
        lv[IS_ADDMOD] = F::ONE;

        for _i in 0..N_RND_TESTS {
            // set inputs to random values and the modulus to zero;
            // the output is defined to be zero when modulus is zero.
            for (&ai, &bi, &mi) in izip!(
                ADDMOD_INPUT_0.iter(),
                ADDMOD_INPUT_1.iter(),
                ADDMOD_MODULUS.iter()
            ) {
                lv[ai] = F::from_canonical_u16(rng.gen());
                lv[bi] = F::from_canonical_u16(rng.gen());
                lv[mi] = F::ZERO;
            }

            generate(&mut lv, columns::IS_ADDMOD);

            // check that the correct output was generated
            assert!(ADDMOD_OUTPUT.iter().all(|&oi| lv[oi] == F::ZERO));

            let mut constraint_consumer = ConstraintConsumer::new(
                vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                GoldilocksField::ONE,
                GoldilocksField::ONE,
                GoldilocksField::ONE,
            );
            eval_packed_generic(&lv, &mut constraint_consumer);
            assert!(constraint_consumer
                .constraint_accs
                .iter()
                .all(|&acc| acc == F::ZERO));

            // Corrupt one output limb by setting it to a non-zero value
            let random_oi = ADDMOD_OUTPUT[rng.gen::<usize>() % N_LIMBS];
            lv[random_oi] = F::from_canonical_u16(rng.gen_range(1..u16::MAX));

            eval_packed_generic(&lv, &mut constraint_consumer);

            // Check that at least one of the constraints was non-zero
            assert!(constraint_consumer
                .constraint_accs
                .iter()
                .any(|&acc| acc != F::ZERO));
        }
    }
}
