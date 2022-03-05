use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

/// Division instruction of a u32 divisor N into a u32 dividend D,
/// with u32 quotient Q and u32 remainder R. If D is not zero, then
/// the values will satisfy N = Q*D + R with 0 <= R < D.  If D is
/// zero, then the remainder is set to the special value u32::MAX =
/// 2^32 - 1 (which is not a valid remainder for any nonzero D) and
/// the quotient is set to zero.  In particular, no overflow is
/// possible.
///
/// FIXME: Should the quotient be set to zero on division-by-zero, or
/// something else?

pub(crate) fn generate_division<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    let dividend = values[COL_DIV_INPUT_DIVIDEND].to_canonical_u64() as u32;
    let divisor = values[COL_DIV_INPUT_DIVISOR].to_canonical_u64() as u32;

    let (quo, rem) = if divisor == 0 {
        (0u32, u32::MAX)
    } else {
        (dividend / divisor, dividend % divisor)
    };

    values[COL_DIV_OUTPUT_QUOT_0] = F::from_canonical_u16(quo as u16);
    values[COL_DIV_OUTPUT_QUOT_1] = F::from_canonical_u16((quo >> 16) as u16);
    values[COL_DIV_OUTPUT_REM_0] = F::from_canonical_u16(rem as u16);
    values[COL_DIV_OUTPUT_REM_1] = F::from_canonical_u16((rem >> 16) as u16);
}

pub(crate) fn eval_division<F: Field, P: PackedField<Scalar = F>>(
    local_values: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_div = local_values[IS_DIV];
    let dividend = local_values[COL_DIV_INPUT_DIVIDEND];
    let divisor = local_values[COL_DIV_INPUT_DIVISOR];
    let quotient_0 = local_values[COL_DIV_OUTPUT_QUOT_0];
    let quotient_1 = local_values[COL_DIV_OUTPUT_QUOT_1];
    let remainder_0 = local_values[COL_DIV_OUTPUT_REM_0];
    let remainder_1 = local_values[COL_DIV_OUTPUT_REM_1];
    let divisor_inv = local_values[COL_DIV_DIVISOR_INV];
    let divisor_rem_diff_m1_0 = local_values[COL_DIV_DIVISOR_REM_DIFF_M1_0];
    let divisor_rem_diff_m1_1 = local_values[COL_DIV_DIVISOR_REM_DIFF_M1_1];

    let base = F::from_canonical_u64(1 << 16);

    let quotient = quotient_0 + quotient_1 * base;
    let remainder = remainder_0 + remainder_1 * base;
    let divisor_rem_diff_m1 = divisor_rem_diff_m1_0 + divisor_rem_diff_m1_1 * base;

    // If dividend is nonzero, the constraint is
    // dividend = divisor * quotient + remainder
    let nonzero_divisor_constr = (divisor * quotient + remainder) - dividend;
    // If dividend is zero, the constraint is
    // quotient = 0 and remainder = u32::MAX.
    let u32_max = P::from(F::from_canonical_u32(u32::MAX));
    let zero_divisor_constr = (remainder - quotient) - u32_max;

    // Selector variable
    let divisor_is_nonzero = divisor * divisor_inv - P::ONES;
    let divisor_is_zero = divisor;
    yield_constr.constraint(is_div * divisor_is_nonzero * nonzero_divisor_constr);
    yield_constr.constraint(is_div * divisor_is_zero * zero_divisor_constr);

    // Finally, ensure that `remainder < quotient`. We know that `divisor_rem_diff_m1` fits in a
    // `u32` because we've range-checked it. Now assert that either the divisor is zero or `divisor
    // - remainder - 1 == divisor_rem_diff_m1`.
    let divisor_rem_diff_constr = divisor - remainder - P::ONES - divisor_rem_diff_m1;
    yield_constr.constraint(is_div * divisor_is_zero * divisor_rem_diff_constr);
}

pub(crate) fn eval_division_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    local_values: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_div = local_values[IS_DIV];
    let dividend = local_values[COL_DIV_INPUT_DIVIDEND];
    let divisor = local_values[COL_DIV_INPUT_DIVISOR];
    let quotient_0 = local_values[COL_DIV_OUTPUT_QUOT_0];
    let quotient_1 = local_values[COL_DIV_OUTPUT_QUOT_1];
    let remainder_0 = local_values[COL_DIV_OUTPUT_REM_0];
    let remainder_1 = local_values[COL_DIV_OUTPUT_REM_1];

    // TODO
}
