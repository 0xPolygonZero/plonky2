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

use ethereum_types::U256;
use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::arithmetic::columns::*;
use crate::arithmetic::utils::u256_to_array;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

/// Generate row for ADD, SUB, GT and LT operations.
pub(crate) fn generate<F: PrimeField64>(
    lv: &mut [F],
    filter: usize,
    left_in: U256,
    right_in: U256,
) {
    u256_to_array(&mut lv[INPUT_REGISTER_0], left_in);
    u256_to_array(&mut lv[INPUT_REGISTER_1], right_in);
    u256_to_array(&mut lv[INPUT_REGISTER_2], U256::zero());

    match filter {
        IS_ADD => {
            let (result, cy) = left_in.overflowing_add(right_in);
            u256_to_array(&mut lv[AUX_INPUT_REGISTER_0], U256::from(cy as u32));
            u256_to_array(&mut lv[OUTPUT_REGISTER], result);
        }
        IS_SUB => {
            let (diff, cy) = left_in.overflowing_sub(right_in);
            u256_to_array(&mut lv[AUX_INPUT_REGISTER_0], U256::from(cy as u32));
            u256_to_array(&mut lv[OUTPUT_REGISTER], diff);
        }
        IS_LT => {
            let (diff, cy) = left_in.overflowing_sub(right_in);
            u256_to_array(&mut lv[AUX_INPUT_REGISTER_0], diff);
            u256_to_array(&mut lv[OUTPUT_REGISTER], U256::from(cy as u32));
        }
        IS_GT => {
            let (diff, cy) = right_in.overflowing_sub(left_in);
            u256_to_array(&mut lv[AUX_INPUT_REGISTER_0], diff);
            u256_to_array(&mut lv[OUTPUT_REGISTER], U256::from(cy as u32));
        }
        _ => panic!("unexpected operation filter"),
    };
}

/// 2^-16 mod (2^64 - 2^32 + 1)
const GOLDILOCKS_INVERSE_65536: u64 = 18446462594437939201;

/// Constrains x + y == z + cy*2^256, assuming filter != 0.
///
/// Set `is_two_row_op=true` to allow the code to be called from the
/// two-row `modular` code (for checking that the modular output is
/// reduced).
///
/// NB: This function ONLY verifies that cy is 0 or 1 when
/// is_two_row_op=false; when is_two_row_op=true the caller must
/// verify for itself.
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
pub(crate) fn eval_packed_generic_addcy<P: PackedField>(
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    x: &[P],
    y: &[P],
    z: &[P],
    given_cy: &[P],
    is_two_row_op: bool,
) {
    debug_assert!(
        x.len() == N_LIMBS && y.len() == N_LIMBS && z.len() == N_LIMBS && given_cy.len() == N_LIMBS
    );

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
        // NB: Mild hack: We don't check that given_cy[0] is 0 or 1
        // when is_two_row_op is true because that's only the case
        // when this function is called from
        // modular::modular_constr_poly(), in which case (1) this
        // condition has already been checked and (2) it exceeds the
        // degree budget because given_cy[0] is already degree 2.
        yield_constr.constraint_transition(filter * (cy - given_cy[0]));
        for i in 1..N_LIMBS {
            yield_constr.constraint_transition(filter * given_cy[i]);
        }
    } else {
        yield_constr.constraint(filter * given_cy[0] * (given_cy[0] - P::ONES));
        yield_constr.constraint(filter * (cy - given_cy[0]));
        for i in 1..N_LIMBS {
            yield_constr.constraint(filter * given_cy[i]);
        }
    }
}

pub(crate) fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_add = lv[IS_ADD];
    let is_sub = lv[IS_SUB];
    let is_lt = lv[IS_LT];
    let is_gt = lv[IS_GT];

    let in0 = &lv[INPUT_REGISTER_0];
    let in1 = &lv[INPUT_REGISTER_1];
    let out = &lv[OUTPUT_REGISTER];
    let aux = &lv[AUX_INPUT_REGISTER_0];

    // x + y = z + w*2^256
    eval_packed_generic_addcy(yield_constr, is_add, in0, in1, out, aux, false);
    eval_packed_generic_addcy(yield_constr, is_sub, in1, out, in0, aux, false);
    eval_packed_generic_addcy(yield_constr, is_lt, in1, aux, in0, out, false);
    eval_packed_generic_addcy(yield_constr, is_gt, in0, aux, in1, out, false);
}

#[allow(clippy::needless_collect)]
pub(crate) fn eval_ext_circuit_addcy<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    x: &[ExtensionTarget<D>],
    y: &[ExtensionTarget<D>],
    z: &[ExtensionTarget<D>],
    given_cy: &[ExtensionTarget<D>],
    is_two_row_op: bool,
) {
    debug_assert!(
        x.len() == N_LIMBS && y.len() == N_LIMBS && z.len() == N_LIMBS && given_cy.len() == N_LIMBS
    );

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

    let good_cy = builder.sub_extension(cy, given_cy[0]);
    let cy_filter = builder.mul_extension(filter, good_cy);

    // Check given carry is one bit
    let bit_constr = builder.mul_sub_extension(given_cy[0], given_cy[0], given_cy[0]);
    let bit_filter = builder.mul_extension(filter, bit_constr);

    if is_two_row_op {
        yield_constr.constraint_transition(builder, cy_filter);
        for i in 1..N_LIMBS {
            let t = builder.mul_extension(filter, given_cy[i]);
            yield_constr.constraint_transition(builder, t);
        }
    } else {
        yield_constr.constraint(builder, bit_filter);
        yield_constr.constraint(builder, cy_filter);
        for i in 1..N_LIMBS {
            let t = builder.mul_extension(filter, given_cy[i]);
            yield_constr.constraint(builder, t);
        }
    }
}

pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_add = lv[IS_ADD];
    let is_sub = lv[IS_SUB];
    let is_lt = lv[IS_LT];
    let is_gt = lv[IS_GT];

    let in0 = &lv[INPUT_REGISTER_0];
    let in1 = &lv[INPUT_REGISTER_1];
    let out = &lv[OUTPUT_REGISTER];
    let aux = &lv[AUX_INPUT_REGISTER_0];

    eval_ext_circuit_addcy(builder, yield_constr, is_add, in0, in1, out, aux, false);
    eval_ext_circuit_addcy(builder, yield_constr, is_sub, in1, out, in0, aux, false);
    eval_ext_circuit_addcy(builder, yield_constr, is_lt, in1, aux, in0, out, false);
    eval_ext_circuit_addcy(builder, yield_constr, is_gt, in0, aux, in1, out, false);
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
    fn generate_eval_consistency_not_addcy() {
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
    fn generate_eval_consistency_addcy() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        const N_ITERS: usize = 1000;

        for _ in 0..N_ITERS {
            for op_filter in [IS_ADD, IS_SUB, IS_LT, IS_GT] {
                // set entire row to random 16-bit values
                let mut lv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));

                // set operation filter and ensure all constraints are
                // satisfied. We have to explicitly set the other
                // operation filters to zero since all are treated by
                // the call.
                lv[IS_ADD] = F::ZERO;
                lv[IS_SUB] = F::ZERO;
                lv[IS_LT] = F::ZERO;
                lv[IS_GT] = F::ZERO;
                lv[op_filter] = F::ONE;

                let left_in = U256::from(rng.gen::<[u8; 32]>());
                let right_in = U256::from(rng.gen::<[u8; 32]>());

                generate(&mut lv, op_filter, left_in, right_in);

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

                let expected = match op_filter {
                    IS_ADD => left_in.overflowing_add(right_in).0,
                    IS_SUB => left_in.overflowing_sub(right_in).0,
                    IS_LT => U256::from((left_in < right_in) as u8),
                    IS_GT => U256::from((left_in > right_in) as u8),
                    _ => panic!("unrecognised operation"),
                };

                let mut expected_limbs = [F::ZERO; N_LIMBS];
                u256_to_array(&mut expected_limbs, expected);
                assert!(expected_limbs
                    .iter()
                    .zip(&lv[OUTPUT_REGISTER])
                    .all(|(x, y)| x == y));
            }
        }
    }
}
