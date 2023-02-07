//! Support for EVM instructions ADD, SUB, LT and GT
//!
//! This crate verifies EVM instructions ADD, SUB, LT and GT (i.e. for
//! unsigned inputs). Each of these instructions can be verified using
//! the "add with carry out" equation
//!
//!   X + Y = Z + CY * 2^256
//!
//! by an appropriate assignment of "inputs" and "outputs" to the
//! variables X, Y, Z and CY. Specifically,
//!
//! ADD: X + Y, inputs X, Y, output Z, ignore CY
//! SUB: Z - X, inputs X, Z, output Y, ignore CY
//!  GT: X > Z, inputs X, Z, output CY, auxiliary output Y
//!  LT: Z < X, inputs Z, X, output CY, auxiliary output Y

use itertools::{izip, Itertools};
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::arithmetic::columns::*;
use crate::arithmetic::utils::read_value_u64_limbs;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

fn u256_add_cc(input0: [u64; N_LIMBS], input1: [u64; N_LIMBS]) -> ([u64; N_LIMBS], u64) {
    // Input and output have 16-bit limbs
    let mut output = [0u64; N_LIMBS];

    const MASK: u64 = (1u64 << LIMB_BITS) - 1u64;
    let mut cy = 0u64;
    for (i, a, b) in izip!(0.., input0, input1) {
        let s = a + b + cy;
        cy = s >> LIMB_BITS;
        assert!(cy <= 1u64, "input limbs were larger than 16 bits");
        output[i] = s & MASK;
    }
    (output, cy)
}

fn u256_sub_br(input0: [u64; N_LIMBS], input1: [u64; N_LIMBS]) -> ([u64; N_LIMBS], u64) {
    const LIMB_BOUNDARY: u64 = 1 << LIMB_BITS;
    const MASK: u64 = LIMB_BOUNDARY - 1u64;

    let mut output = [0u64; N_LIMBS];
    let mut br = 0u64;
    for (i, a, b) in izip!(0.., input0, input1) {
        let d = LIMB_BOUNDARY + a - b - br;
        // if a < b, then d < 2^16 so br = 1
        // if a >= b, then d >= 2^16 so br = 0
        br = 1u64 - (d >> LIMB_BITS);
        assert!(br <= 1u64, "input limbs were larger than 16 bits");
        output[i] = d & MASK;
    }

    (output, br)
}

/// Generate row for ADD, SUB, GT and LT operations.
///
/// A row consists of four values, GENERAL_REGISTER_[012] and
/// GENERAL_REGISTER_BIT. The interpretation of these values for each
/// operation is as follows:
///
/// ADD: REGISTER_0 + REGISTER_1, output in REGISTER_2, ignore REGISTER_BIT
/// SUB: REGISTER_2 - REGISTER_0, output in REGISTER_1, ignore REGISTER_BIT
///  GT: REGISTER_0 > REGISTER_2, output in REGISTER_BIT, auxiliary output in REGISTER_1
///  LT: REGISTER_2 < REGISTER_0, output in REGISTER_BIT, auxiliary output in REGISTER_1
pub(crate) fn generate<F: RichField>(lv: &mut [F], filter: usize) {
    match filter {
        IS_ADD => {
            let x = read_value_u64_limbs(lv, GENERAL_REGISTER_0);
            let y = read_value_u64_limbs(lv, GENERAL_REGISTER_1);

            // x + y == z + cy*2^256
            let (z, cy) = u256_add_cc(x, y);

            lv[GENERAL_REGISTER_2].copy_from_slice(&z.map(F::from_canonical_u64));
            lv[GENERAL_REGISTER_BIT] = F::from_canonical_u64(cy);
        }
        IS_SUB | IS_GT | IS_LT => {
            let x = read_value_u64_limbs(lv, GENERAL_REGISTER_0);
            let z = read_value_u64_limbs(lv, GENERAL_REGISTER_2);

            // y == z - x + cy*2^256
            let (y, cy) = u256_sub_br(z, x);

            lv[GENERAL_REGISTER_1].copy_from_slice(&y.map(F::from_canonical_u64));
            lv[GENERAL_REGISTER_BIT] = F::from_canonical_u64(cy);
        }
        _ => panic!("unexpected operation filter"),
    };
}

fn eval_packed_generic_check_is_one_bit<P: PackedField>(
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    x: P,
) {
    yield_constr.constraint(filter * x * (x - P::ONES));
}

fn eval_ext_circuit_check_is_one_bit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    x: ExtensionTarget<D>,
) {
    let constr = builder.mul_sub_extension(x, x, x);
    let filtered_constr = builder.mul_extension(filter, constr);
    yield_constr.constraint(builder, filtered_constr);
}

/// 2^-16 mod (2^64 - 2^32 + 1)
const GOLDILOCKS_INVERSE_65536: u64 = 18446462594437939201;

/// Constrains x + y == z + cy*2^256, assuming filter != 0.
///
/// NB: This function DOES NOT verify that cy is 0 or 1; the caller
/// must do that.
///
/// Set `is_two_row_op=true` to allow the code to be called from the
/// two-row `modular` code (for checking that the modular output is
/// reduced).
///
/// Note that the digits of `x + y` are in `[0, 2*(2^16-1)]`
/// (i.e. they are the sums of two 16-bit numbers), whereas the digits
/// of `z` can only be in `[0, 2^16-1]`. In the function we check that:
///
/// \sum_i (x_i + y_i) * 2^(16*i) = \sum_i z_i * 2^(16*i) + given_cy*2^256.
///
/// If `N_LIMBS = 1`, then this amounts to verifying that either `x_0
/// + y_0 = z_0` or `x_0 + y_0 == z_0 + cy*2^16` (this is `t` on line
/// 127ff). Ok. Now assume the constraints are valid for `N_LIMBS =
/// n-1`. Then by induction,
///
/// \sum_{i=0}^{n-1} (x_i + y_i) * 2^(16*i) + (x_n + y_n)*2^(16*n) ==
/// \sum_{i=0}^{n-1} z_i * 2^(16*i) + cy_{n-1}*2^(16*n) + z_n*2^(16*n)
/// + cy_n*2^(16*n)
///
/// is true if `(x_n + y_n)*2^(16*n) == cy_{n-1}*2^(16*n) +
/// z_n*2^(16*n) + cy_n*2^(16*n)` (again, this is `t` on line 127ff)
/// with the last `cy_n` checked against the `given_cy` given as input.
pub(crate) fn eval_packed_generic_add_cc<P: PackedField>(
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    x: &[P],
    y: &[P],
    z: &[P],
    given_cy: P,
    is_two_row_op: bool,
) {
    debug_assert!(x.len() == N_LIMBS && y.len() == N_LIMBS && z.len() == N_LIMBS);

    let overflow = P::Scalar::from_canonical_u64(1u64 << LIMB_BITS);
    let overflow_inv = P::Scalar::from_canonical_u64(GOLDILOCKS_INVERSE_65536);
    debug_assert!(
        overflow * overflow_inv == P::Scalar::ONE,
        "only works with LIMB_BITS=16 and F=Goldilocks"
    );

    let mut cy = P::ZEROS;
    for ((&xi, &yi), &zi) in x.iter().zip_eq(y).zip_eq(z) {
        // Verify that (xi + yi) - zi is either 0 or 2^LIMB_BITS
        let t = cy + xi + yi - zi;
        if is_two_row_op {
            yield_constr.constraint_transition(filter * t * (overflow - t));
        } else {
            yield_constr.constraint(filter * t * (overflow - t));
        }
        // cy <-- 0 or 1
        // NB: this is multiplication by a constant, so doesn't
        // increase the degree of the constraint.
        cy = t * overflow_inv;
    }

    if is_two_row_op {
        yield_constr.constraint_transition(filter * (cy - given_cy));
    } else {
        yield_constr.constraint(filter * (cy - given_cy));
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_add = lv[IS_ADD];
    let is_sub = lv[IS_SUB];
    let is_lt = lv[IS_LT];
    let is_gt = lv[IS_GT];

    let x = &lv[GENERAL_REGISTER_0];
    let y = &lv[GENERAL_REGISTER_1];
    let z = &lv[GENERAL_REGISTER_2];
    let cy = lv[GENERAL_REGISTER_BIT];

    let op_filter = is_add + is_sub + is_lt + is_gt;
    eval_packed_generic_check_is_one_bit(yield_constr, op_filter, cy);

    // x + y = z + cy*2^256
    eval_packed_generic_add_cc(yield_constr, op_filter, x, y, z, cy, false);
}

#[allow(clippy::needless_collect)]
pub(crate) fn eval_ext_circuit_add_cc<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    x: &[ExtensionTarget<D>],
    y: &[ExtensionTarget<D>],
    z: &[ExtensionTarget<D>],
    given_cy: ExtensionTarget<D>,
    is_two_row_op: bool,
) {
    debug_assert!(x.len() == N_LIMBS && y.len() == N_LIMBS && z.len() == N_LIMBS);

    // 2^LIMB_BITS in the base field
    let overflow_base = F::from_canonical_u64(1 << LIMB_BITS);
    // 2^LIMB_BITS in the extension field as an ExtensionTarget
    let overflow = builder.constant_extension(F::Extension::from(overflow_base));
    // 2^-LIMB_BITS in the base field.
    let overflow_inv = F::from_canonical_u64(GOLDILOCKS_INVERSE_65536);

    let mut cy = builder.zero_extension();
    for ((&xi, &yi), &zi) in x.iter().zip_eq(y).zip_eq(z) {
        // t0 = cy + xi + yi
        let t0 = builder.add_many_extension([cy, xi, yi]);
        // t  = t0 - zi
        let t = builder.sub_extension(t0, zi);
        // t1 = overflow - t
        let t1 = builder.sub_extension(overflow, t);
        // t2 = t * t1
        let t2 = builder.mul_extension(t, t1);

        let filtered_limb_constraint = builder.mul_extension(filter, t2);
        if is_two_row_op {
            yield_constr.constraint_transition(builder, filtered_limb_constraint);
        } else {
            yield_constr.constraint(builder, filtered_limb_constraint);
        }

        cy = builder.mul_const_extension(overflow_inv, t);
    }

    let good_cy = builder.sub_extension(cy, given_cy);
    let filter = builder.mul_extension(filter, good_cy);
    if is_two_row_op {
        yield_constr.constraint_transition(builder, filter);
    } else {
        yield_constr.constraint(builder, filter);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_add = lv[IS_ADD];
    let is_sub = lv[IS_SUB];
    let is_lt = lv[IS_LT];
    let is_gt = lv[IS_GT];

    let x = &lv[GENERAL_REGISTER_0];
    let y = &lv[GENERAL_REGISTER_1];
    let z = &lv[GENERAL_REGISTER_2];
    let cy = lv[GENERAL_REGISTER_BIT];

    let op_filter = builder.add_many_extension([is_add, is_sub, is_lt, is_gt]);
    eval_ext_circuit_check_is_one_bit(builder, yield_constr, op_filter, cy);
    eval_ext_circuit_add_cc(builder, yield_constr, op_filter, x, y, z, cy, false);
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::{Field, Sample};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::*;
    use crate::arithmetic::columns::NUM_ARITH_COLUMNS;
    use crate::constraint_consumer::ConstraintConsumer;

    // TODO: Should be able to refactor this test to apply to all operations.
    #[test]
    fn generate_eval_consistency_not_addcc() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));

        // if the operation filters are all zero, then the constraints
        // should be met even if all values are
        // garbage.
        lv[IS_ADD] = F::ZERO;
        lv[IS_SUB] = F::ZERO;
        lv[IS_LT] = F::ZERO;
        lv[IS_GT] = F::ZERO;

        let mut constrant_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            F::ONE,
            F::ONE,
            F::ONE,
        );
        eval_packed_generic(&lv, &mut constrant_consumer);
        for &acc in &constrant_consumer.constraint_accs {
            assert_eq!(acc, F::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency_addcc() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        const N_ITERS: usize = 1000;

        for _ in 0..N_ITERS {
            for op_filter in [IS_ADD, IS_SUB, IS_LT, IS_GT] {
                // set entire row to random 16-bit values
                let mut lv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));

                // set operation filter and ensure all constraints are
                // satisfied.  we have to explicitly set the other
                // operation filters to zero since all are treated by
                // the call.
                lv[IS_ADD] = F::ZERO;
                lv[IS_SUB] = F::ZERO;
                lv[IS_LT] = F::ZERO;
                lv[IS_GT] = F::ZERO;
                lv[op_filter] = F::ONE;

                generate(&mut lv, op_filter);

                let mut constrant_consumer = ConstraintConsumer::new(
                    vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                    F::ONE,
                    F::ONE,
                    F::ONE,
                );
                eval_packed_generic(&lv, &mut constrant_consumer);
                for &acc in &constrant_consumer.constraint_accs {
                    assert_eq!(acc, F::ZERO);
                }
            }
        }
    }
}
