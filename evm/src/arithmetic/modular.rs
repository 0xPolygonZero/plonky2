//! Support for the EVM modular instructions ADDMOD, MULMOD and MOD,
//! as well as DIV.
//!
//! This crate verifies an EVM modular instruction, which takes three
//! 256-bit inputs A, B and M, and produces a 256-bit output C satisfying
//!
//!    C = operation(A, B) (mod M).
//!
//! where operation can be addition, multiplication, or just return
//! the first argument (for MOD).  Inputs A, B and M, and output C,
//! are given as arrays of 16-bit limbs. For example, if the limbs of
//! A are a[0]...a[15], then
//!
//!    A = \sum_{i=0}^15 a[i] β^i,
//!
//! where β = 2^16 = 2^LIMB_BITS. To verify that A, B, M and C satisfy
//! the equation we proceed as follows. Define
//!
//!    a(x) = \sum_{i=0}^15 a[i] x^i
//!
//! (so A = a(β)) and similarly for b(x), m(x) and c(x). Then
//! operation(A,B) = C (mod M) if and only if there exists q such that
//! the polynomial
//!
//!    operation(a(x), b(x)) - c(x) - m(x) * q(x)
//!
//! is zero when evaluated at x = β, i.e. it is divisible by (x - β);
//! equivalently, there exists a polynomial s such that
//!
//!    operation(a(x), b(x)) - c(x) - m(x) * q(x) - (x - β) * s(x) == 0
//!
//! if and only if operation(A,B) = C (mod M). In the code below, this
//! "constraint polynomial" is constructed in the variable
//! `constr_poly`. It must be identically zero for the modular
//! operation to be verified, or, equivalently, each of its
//! coefficients must be zero. The variable names of the constituent
//! polynomials are (writing N for N_LIMBS=16):
//!
//!   a(x) = \sum_{i=0}^{N-1} input0[i] * x^i
//!   b(x) = \sum_{i=0}^{N-1} input1[i] * x^i
//!   c(x) = \sum_{i=0}^{N-1} output[i] * x^i
//!   m(x) = \sum_{i=0}^{N-1} modulus[i] * x^i
//!   q(x) = \sum_{i=0}^{2N-1} quot[i] * x^i
//!   s(x) = \sum_i^{2N-2} aux[i] * x^i
//!
//! Because A, B, M and C are 256-bit numbers, the degrees of a, b, m
//! and c are (at most) N-1 = 15. If m = 1, then Q would be A*B which
//! can be up to 2^512 - ε, so deg(q) can be up to 2*N-1 = 31. Note
//! that, although for arbitrary m and q we might have deg(m*q) = 3*N-2,
//! because the magnitude of M*Q must match that of operation(A,B), we
//! always have deg(m*q) <= 2*N-1. Finally, in order for all the degrees
//! to match, we have deg(s) <= 2*N-2 = 30.
//!
//! -*-
//!
//! To verify that the output is reduced, that is, output < modulus,
//! the prover supplies the value `out_aux_red` which must satisfy
//!
//!    output - modulus = out_aux_red + 2^256
//!
//! and these values are passed to the "less than" operation.
//!
//! -*-
//!
//! The EVM defines division by zero as zero. We handle this as
//! follows:
//!
//! The prover supplies a binary value `mod_is_zero` which is one if
//! the modulus is zero and zero otherwise. This is verified, then
//! added to the modulus (this can't overflow, as modulus[0] was
//! range-checked and mod_is_zero is 0 or 1). The rest of the
//! calculation proceeds as if modulus was actually 1; this correctly
//! verifies that the output is zero, as required by the standard.
//! To summarise:
//!
//! - mod_is_zero is 0 or 1
//! - if mod_is_zero is 1, then
//!    - given modulus is 0
//!    - updated modulus is 1, which forces the correct output of 0
//! - if mod_is_zero is 0, then
//!    - given modulus can be 0 or non-zero
//!    - updated modulus is same as given
//!    - if modulus is non-zero, correct output is obtained
//!    - if modulus is 0, then the test output < modulus, checking that
//!      the output is reduced, will fail, because output is non-negative.
//!
//! In the case of DIV, we do something similar, except that we "replace"
//! the modulus with "2^256" to force the quotient to be zero.

use num::{bigint::Sign, BigInt, One, Zero};
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use super::columns;
use crate::arithmetic::columns::*;
use crate::arithmetic::compare::{eval_ext_circuit_lt, eval_packed_generic_lt};
use crate::arithmetic::utils::*;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

/// Convert the base-2^16 representation of a number into a BigInt.
///
/// Given `N` signed (16 + ε)-bit values in `limbs`, return the BigInt
///
///   \sum_{i=0}^{N-1} limbs[i] * β^i.
///
/// This is basically "evaluate the given polynomial at β". Although
/// the input type is i64, the values must always be in (-2^16 - ε,
/// 2^16 + ε) because of the caller's range check on the inputs (the ε
/// allows us to convert calculated output, which can be bigger than
/// 2^16).
fn columns_to_bigint<const N: usize>(limbs: &[i64; N]) -> BigInt {
    const BASE: i64 = 1i64 << LIMB_BITS;

    let mut pos_limbs_u32 = Vec::with_capacity(N / 2 + 1);
    let mut neg_limbs_u32 = Vec::with_capacity(N / 2 + 1);
    let mut cy = 0i64; // cy is necessary to handle ε > 0
    for i in 0..(N / 2) {
        let t = cy + limbs[2 * i] + BASE * limbs[2 * i + 1];
        pos_limbs_u32.push(if t > 0 { t as u32 } else { 0u32 });
        neg_limbs_u32.push(if t < 0 { -t as u32 } else { 0u32 });
        cy = t / (1i64 << 32);
    }
    if N & 1 != 0 {
        // If N is odd we need to add the last limb on its own
        let t = cy + limbs[N - 1];
        pos_limbs_u32.push(if t > 0 { t as u32 } else { 0u32 });
        neg_limbs_u32.push(if t < 0 { -t as u32 } else { 0u32 });
        cy = t / (1i64 << 32);
    }
    pos_limbs_u32.push(if cy > 0 { cy as u32 } else { 0u32 });
    neg_limbs_u32.push(if cy < 0 { -cy as u32 } else { 0u32 });

    let pos = BigInt::from_slice(Sign::Plus, &pos_limbs_u32);
    let neg = BigInt::from_slice(Sign::Plus, &neg_limbs_u32);
    pos - neg
}

/// Convert a BigInt into a base-2^16 representation.
///
/// Given a BigInt `num`, return an array of `N` signed 16-bit
/// values, say `limbs`, such that
///
///   num = \sum_{i=0}^{N-1} limbs[i] * β^i.
///
/// Note that `N` must be at least ceil(log2(num)/16) in order to be
/// big enough to hold `num`.
fn bigint_to_columns<const N: usize>(num: &BigInt) -> [i64; N] {
    assert!(num.bits() <= 16 * N as u64);
    let mut output = [0i64; N];
    for (i, limb) in num.iter_u32_digits().enumerate() {
        output[2 * i] = limb as u16 as i64;
        output[2 * i + 1] = (limb >> LIMB_BITS) as i64;
    }
    if num.sign() == Sign::Minus {
        for c in output.iter_mut() {
            *c = -*c;
        }
    }
    output
}

/// Generate the output and auxiliary values for given `operation`.
///
/// NB: `operation` can set the higher order elements in its result to
/// zero if they are not used.
fn generate_modular_op<F: RichField>(
    lv: &mut [F; NUM_ARITH_COLUMNS],
    filter: usize,
    operation: fn([i64; N_LIMBS], [i64; N_LIMBS]) -> [i64; 2 * N_LIMBS - 1],
) {
    // Inputs are all range-checked in [0, 2^16), so the "as i64"
    // conversion is safe.
    let input0_limbs = read_value_i64_limbs(lv, MODULAR_INPUT_0);
    let input1_limbs = read_value_i64_limbs(lv, MODULAR_INPUT_1);
    let mut modulus_limbs = read_value_i64_limbs(lv, MODULAR_MODULUS);

    // BigInts are just used to avoid having to implement modular
    // reduction.
    let mut modulus = columns_to_bigint(&modulus_limbs);

    // constr_poly is initialised to the calculated input, and is
    // used as such for the BigInt reduction; later, other values are
    // added/subtracted, which is where its meaning as the "constraint
    // polynomial" comes in.
    let mut constr_poly = [0i64; 2 * N_LIMBS];
    constr_poly[..2 * N_LIMBS - 1].copy_from_slice(&operation(input0_limbs, input1_limbs));

    // two_exp_256 == 2^256
    let two_exp_256 = {
        let mut t = BigInt::zero();
        t.set_bit(256, true);
        t
    };

    let mut mod_is_zero = F::ZERO;
    if modulus.is_zero() {
        if filter == columns::IS_DIV {
            // set modulus = 2^256
            modulus = two_exp_256.clone();
            // modulus_limbs don't play a role below
        } else {
            // set modulus = 1
            modulus = BigInt::one();
            modulus_limbs[0] = 1i64;
        }
        mod_is_zero = F::ONE;
    }

    let input = columns_to_bigint(&constr_poly);

    // modulus != 0 here, because, if the given modulus was zero, then
    // it was set to 1 or 2^256 above
    let mut output = &input % &modulus;
    // output will be -ve (but > -modulus) if input was -ve, so we can
    // add modulus to obtain a "canonical" +ve output.
    if output.sign() == Sign::Minus {
        output += &modulus;
    }
    let output_limbs = bigint_to_columns::<N_LIMBS>(&output);
    let quot = (&input - &output) / &modulus; // exact division; can be -ve
    let quot_limbs = bigint_to_columns::<{ 2 * N_LIMBS }>(&quot);

    // output < modulus here, so the proof requires (output - modulus) % 2^256:
    let out_aux_red = bigint_to_columns::<N_LIMBS>(&(two_exp_256 + output - modulus));

    // constr_poly is the array of coefficients of the polynomial
    //
    //   operation(a(x), b(x)) - c(x) - s(x)*m(x).
    //
    pol_sub_assign(&mut constr_poly, &output_limbs);
    let prod = pol_mul_wide2(quot_limbs, modulus_limbs);
    pol_sub_assign(&mut constr_poly, &prod[0..2 * N_LIMBS]);

    // Higher order terms of the product must be zero for valid quot and modulus:
    debug_assert!(&prod[2 * N_LIMBS..].iter().all(|&x| x == 0i64));

    // constr_poly must be zero when evaluated at x = β :=
    // 2^LIMB_BITS, hence it's divisible by (x - β). `aux_limbs` is
    // the result of removing that root.
    let aux_limbs = pol_remove_root_2exp::<LIMB_BITS, _, { 2 * N_LIMBS }>(constr_poly);

    lv[MODULAR_OUTPUT].copy_from_slice(&output_limbs.map(|c| F::from_canonical_i64(c)));
    lv[MODULAR_OUT_AUX_RED].copy_from_slice(&out_aux_red.map(|c| F::from_canonical_i64(c)));
    lv[MODULAR_QUO_INPUT].copy_from_slice(&quot_limbs.map(|c| F::from_noncanonical_i64(c)));
    lv[MODULAR_AUX_INPUT].copy_from_slice(&aux_limbs.map(|c| F::from_noncanonical_i64(c)));
    lv[MODULAR_MOD_IS_ZERO] = mod_is_zero;
}

/// Generate the output and auxiliary values for modular operations.
///
/// `filter` must be one of `columns::IS_{ADDMOD,MULMOD,MOD}`.
pub(crate) fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS], filter: usize) {
    match filter {
        columns::IS_ADDMOD => generate_modular_op(lv, filter, pol_add),
        columns::IS_SUBMOD => generate_modular_op(lv, filter, pol_sub),
        columns::IS_MULMOD => generate_modular_op(lv, filter, pol_mul_wide),
        columns::IS_MOD | columns::IS_DIV => generate_modular_op(lv, filter, |a, _| pol_extend(a)),
        _ => panic!("generate modular operation called with unknown opcode"),
    }
}

/// Build the part of the constraint polynomial that's common to all
/// modular operations, and perform the common verifications.
///
/// Specifically, with the notation above, build the polynomial
///
///   c(x) + q(x) * m(x) + (x - β) * s(x)
///
/// and check consistency when m = 0, and that c is reduced.
fn modular_constr_poly<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
) -> [P; 2 * N_LIMBS] {
    range_check_error!(MODULAR_INPUT_0, 16);
    range_check_error!(MODULAR_INPUT_1, 16);
    range_check_error!(MODULAR_MODULUS, 16);
    range_check_error!(MODULAR_QUO_INPUT, 16);
    range_check_error!(MODULAR_AUX_INPUT, 20, signed);
    range_check_error!(MODULAR_OUTPUT, 16);

    let mut modulus = read_value::<N_LIMBS, _>(lv, MODULAR_MODULUS);
    let mod_is_zero = lv[MODULAR_MOD_IS_ZERO];

    // Check that mod_is_zero is zero or one
    yield_constr.constraint(filter * (mod_is_zero * mod_is_zero - mod_is_zero));

    // Check that mod_is_zero is zero if modulus is not zero (they
    // could both be zero)
    let limb_sum = modulus.into_iter().sum::<P>();
    yield_constr.constraint(filter * limb_sum * mod_is_zero);

    // See the file documentation for why this suffices to handle
    // modulus = 0.
    modulus[0] += mod_is_zero;

    let mut output = read_value::<N_LIMBS, _>(lv, MODULAR_OUTPUT);

    // Needed to compensate for adding mod_is_zero to modulus above,
    // since the call eval_packed_generic_lt() below subtracts modulus
    // verify in the case of a DIV.
    output[0] += mod_is_zero * lv[IS_DIV];

    // Verify that the output is reduced, i.e. output < modulus.
    let out_aux_red = &lv[MODULAR_OUT_AUX_RED];
    // this sets is_less_than to 1 unless we get mod_is_zero when
    // doing a DIV; in that case, we need is_less_than=0, since the
    // function checks
    //
    //   output - modulus == out_aux_red + is_less_than*2^256
    //
    // and we were given output = out_aux_red
    let is_less_than = P::ONES - mod_is_zero * lv[IS_DIV];
    eval_packed_generic_lt(
        yield_constr,
        filter,
        &output,
        &modulus,
        out_aux_red,
        is_less_than,
    );
    // restore output[0]
    output[0] -= mod_is_zero * lv[IS_DIV];

    // prod = q(x) * m(x)
    let quot = read_value::<{ 2 * N_LIMBS }, _>(lv, MODULAR_QUO_INPUT);
    let prod = pol_mul_wide2(quot, modulus);
    // higher order terms must be zero
    for &x in prod[2 * N_LIMBS..].iter() {
        yield_constr.constraint(filter * x);
    }

    // constr_poly = c(x) + q(x) * m(x)
    let mut constr_poly: [_; 2 * N_LIMBS] = prod[0..2 * N_LIMBS].try_into().unwrap();
    pol_add_assign(&mut constr_poly, &output);

    // constr_poly = c(x) + q(x) * m(x) + (x - β) * s(x)
    let mut aux = read_value::<{ 2 * N_LIMBS }, _>(lv, MODULAR_AUX_INPUT);
    aux[2 * N_LIMBS - 1] = P::ZEROS; // zero out the MOD_IS_ZERO flag
    let base = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    pol_add_assign(&mut constr_poly, &pol_adjoin_root(aux, base));

    constr_poly
}

/// Add constraints for modular operations.
pub(crate) fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // NB: The CTL code guarantees that filter is 0 or 1, i.e. that
    // only one of the operations below is "live".
    let filter = lv[columns::IS_ADDMOD]
        + lv[columns::IS_MULMOD]
        + lv[columns::IS_MOD]
        + lv[columns::IS_SUBMOD]
        + lv[columns::IS_DIV];

    // constr_poly has 2*N_LIMBS limbs
    let constr_poly = modular_constr_poly(lv, yield_constr, filter);

    let input0 = read_value(lv, MODULAR_INPUT_0);
    let input1 = read_value(lv, MODULAR_INPUT_1);

    let add_input = pol_add(input0, input1);
    let sub_input = pol_sub(input0, input1);
    let mul_input = pol_mul_wide(input0, input1);
    let mod_input = pol_extend(input0);

    for (input, &filter) in [
        (&add_input, &lv[columns::IS_ADDMOD]),
        (&sub_input, &lv[columns::IS_SUBMOD]),
        (&mul_input, &lv[columns::IS_MULMOD]),
        (&mod_input, &(lv[columns::IS_MOD] + lv[columns::IS_DIV])),
    ] {
        // Need constr_poly_copy to be the first argument to
        // pol_sub_assign, since it is the longer of the two
        // arguments.
        let mut constr_poly_copy = constr_poly;
        pol_sub_assign(&mut constr_poly_copy, input);

        // At this point constr_poly_copy holds the coefficients of
        // the polynomial
        //
        //   operation(a(x), b(x)) - c(x) - q(x) * m(x) - (x - β) * s(x)
        //
        // where operation is add, mul or |a,b|->a.  The modular
        // operation is valid if and only if all of those coefficients
        // are zero.
        for &c in constr_poly_copy.iter() {
            yield_constr.constraint(filter * c);
        }
    }
}

fn modular_constr_poly_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    builder: &mut CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
) -> [ExtensionTarget<D>; 2 * N_LIMBS] {
    let mut modulus = read_value::<N_LIMBS, _>(lv, MODULAR_MODULUS);
    let mod_is_zero = lv[MODULAR_MOD_IS_ZERO];

    let t = builder.mul_sub_extension(mod_is_zero, mod_is_zero, mod_is_zero);
    let t = builder.mul_extension(filter, t);
    yield_constr.constraint(builder, t);

    let limb_sum = builder.add_many_extension(modulus);
    let t = builder.mul_extension(limb_sum, mod_is_zero);
    let t = builder.mul_extension(filter, t);
    yield_constr.constraint(builder, t);

    modulus[0] = builder.add_extension(modulus[0], mod_is_zero);

    let mut output = read_value::<N_LIMBS, _>(lv, MODULAR_OUTPUT);
    output[0] = builder.mul_add_extension(mod_is_zero, lv[IS_DIV], output[0]);

    let out_aux_red = &lv[MODULAR_OUT_AUX_RED];
    let one = builder.one_extension();
    let is_less_than =
        builder.arithmetic_extension(F::NEG_ONE, F::ONE, mod_is_zero, lv[IS_DIV], one);

    eval_ext_circuit_lt(
        builder,
        yield_constr,
        filter,
        &output,
        &modulus,
        out_aux_red,
        is_less_than,
    );
    output[0] =
        builder.arithmetic_extension(F::NEG_ONE, F::ONE, mod_is_zero, lv[IS_DIV], output[0]);

    let quot = read_value::<{ 2 * N_LIMBS }, _>(lv, MODULAR_QUO_INPUT);
    let prod = pol_mul_wide2_ext_circuit(builder, quot, modulus);
    for &x in prod[2 * N_LIMBS..].iter() {
        let t = builder.mul_extension(filter, x);
        yield_constr.constraint(builder, t);
    }

    let mut constr_poly: [_; 2 * N_LIMBS] = prod[0..2 * N_LIMBS].try_into().unwrap();
    pol_add_assign_ext_circuit(builder, &mut constr_poly, &output);

    let mut aux = read_value::<{ 2 * N_LIMBS }, _>(lv, MODULAR_AUX_INPUT);
    aux[2 * N_LIMBS - 1] = builder.zero_extension();
    let base = builder.constant_extension(F::Extension::from_canonical_u64(1u64 << LIMB_BITS));
    let t = pol_adjoin_root_ext_circuit(builder, aux, base);
    pol_add_assign_ext_circuit(builder, &mut constr_poly, &t);

    constr_poly
}

pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = builder.add_many_extension([
        lv[columns::IS_ADDMOD],
        lv[columns::IS_SUBMOD],
        lv[columns::IS_MULMOD],
        lv[columns::IS_MOD],
        lv[columns::IS_DIV],
    ]);

    let constr_poly = modular_constr_poly_ext_circuit(lv, builder, yield_constr, filter);

    let input0 = read_value(lv, MODULAR_INPUT_0);
    let input1 = read_value(lv, MODULAR_INPUT_1);

    let add_input = pol_add_ext_circuit(builder, input0, input1);
    let sub_input = pol_sub_ext_circuit(builder, input0, input1);
    let mul_input = pol_mul_wide_ext_circuit(builder, input0, input1);
    let mod_input = pol_extend_ext_circuit(builder, input0);

    let mod_div_filter = builder.add_extension(lv[columns::IS_MOD], lv[columns::IS_DIV]);
    for (input, &filter) in [
        (&add_input, &lv[columns::IS_ADDMOD]),
        (&sub_input, &lv[columns::IS_SUBMOD]),
        (&mul_input, &lv[columns::IS_MULMOD]),
        (&mod_input, &mod_div_filter),
    ] {
        let mut constr_poly_copy = constr_poly;
        pol_sub_assign_ext_circuit(builder, &mut constr_poly_copy, input);
        for &c in constr_poly_copy.iter() {
            let t = builder.mul_extension(filter, c);
            yield_constr.constraint(builder, t);
        }
    }
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
    fn generate_eval_consistency_not_modular() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // if `IS_ADDMOD == 0`, then the constraints should be met even
        // if all values are garbage.
        lv[IS_ADDMOD] = F::ZERO;
        lv[IS_SUBMOD] = F::ZERO;
        lv[IS_MULMOD] = F::ZERO;
        lv[IS_MOD] = F::ZERO;
        lv[IS_DIV] = F::ZERO;

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
    fn generate_eval_consistency() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        for op_filter in [IS_ADDMOD, IS_DIV, IS_SUBMOD, IS_MOD, IS_MULMOD] {
            // Reset operation columns, then select one
            lv[IS_ADDMOD] = F::ZERO;
            lv[IS_SUBMOD] = F::ZERO;
            lv[IS_MULMOD] = F::ZERO;
            lv[IS_MOD] = F::ZERO;
            lv[IS_DIV] = F::ZERO;
            lv[op_filter] = F::ONE;

            for i in 0..N_RND_TESTS {
                // set inputs to random values
                for (ai, bi, mi) in izip!(MODULAR_INPUT_0, MODULAR_INPUT_1, MODULAR_MODULUS) {
                    lv[ai] = F::from_canonical_u16(rng.gen());
                    lv[bi] = F::from_canonical_u16(rng.gen());
                    lv[mi] = F::from_canonical_u16(rng.gen());
                }

                // For the second half of the tests, set the top 16 -
                // start digits of the modulus to zero so it is much
                // smaller than the inputs.
                if i > N_RND_TESTS / 2 {
                    // 1 <= start < N_LIMBS
                    let start = (rng.gen::<usize>() % (N_LIMBS - 1)) + 1;
                    for mi in MODULAR_MODULUS.skip(start) {
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
    fn zero_modulus() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        for op_filter in [IS_ADDMOD, IS_SUBMOD, IS_DIV, IS_MOD, IS_MULMOD] {
            // Reset operation columns, then select one
            lv[IS_ADDMOD] = F::ZERO;
            lv[IS_SUBMOD] = F::ZERO;
            lv[IS_MULMOD] = F::ZERO;
            lv[IS_MOD] = F::ZERO;
            lv[IS_DIV] = F::ZERO;
            lv[op_filter] = F::ONE;

            for _i in 0..N_RND_TESTS {
                // set inputs to random values and the modulus to zero;
                // the output is defined to be zero when modulus is zero.
                for (ai, bi, mi) in izip!(MODULAR_INPUT_0, MODULAR_INPUT_1, MODULAR_MODULUS) {
                    lv[ai] = F::from_canonical_u16(rng.gen());
                    lv[bi] = F::from_canonical_u16(rng.gen());
                    lv[mi] = F::ZERO;
                }

                generate(&mut lv, op_filter);

                // check that the correct output was generated
                if op_filter == IS_DIV {
                    assert!(lv[DIV_OUTPUT].iter().all(|&c| c == F::ZERO));
                } else {
                    assert!(lv[MODULAR_OUTPUT].iter().all(|&c| c == F::ZERO));
                }

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
                let random_oi = if op_filter == IS_DIV {
                    DIV_OUTPUT.start + rng.gen::<usize>() % N_LIMBS
                } else {
                    MODULAR_OUTPUT.start + rng.gen::<usize>() % N_LIMBS
                };
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
}
