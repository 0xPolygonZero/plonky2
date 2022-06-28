//! Helper methods for checking that a value is canonical, i.e. is less than `|F|`.
//!
//! See https://hackmd.io/NC-yRmmtRQSvToTHb96e8Q#Checking-element-validity

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

/// Computes the helper value used in the is-canonical check.
pub(crate) fn compute_canonical_inv<F: Field>(value_to_check: u64) -> F {
    let value_hi_32 = (value_to_check >> 32) as u32;

    if value_hi_32 == u32::MAX {
        debug_assert_eq!(value_to_check as u32, 0, "Value was not canonical.");
        // In this case it doesn't matter what we put for the purported inverse value. The
        // constraint containing this value will get multiplied by the low u32 limb, which will be
        // zero, satisfying the constraint regardless of what we put here.
        F::ZERO
    } else {
        F::from_canonical_u32(u32::MAX - value_hi_32).inverse()
    }
}

/// Adds constraints to require that a list of four `u16`s, in little-endian order, represent a
/// canonical field element, i.e. that their combined value is less than `|F|`. Returns their
/// combined value.
pub(crate) fn combine_u16s_check_canonical<F: Field, P: PackedField<Scalar = F>>(
    limb_0_u16: P,
    limb_1_u16: P,
    limb_2_u16: P,
    limb_3_u16: P,
    inverse: P,
    yield_constr: &mut ConstraintConsumer<P>,
) -> P {
    let base = F::from_canonical_u32(1 << 16);
    let limb_0_u32 = limb_0_u16 + limb_1_u16 * base;
    let limb_1_u32 = limb_2_u16 + limb_3_u16 * base;
    combine_u32s_check_canonical(limb_0_u32, limb_1_u32, inverse, yield_constr)
}

/// Adds constraints to require that a list of four `u16`s, in little-endian order, represent a
/// canonical field element, i.e. that their combined value is less than `|F|`. Returns their
/// combined value.
pub(crate) fn combine_u16s_check_canonical_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    limb_0_u16: ExtensionTarget<D>,
    limb_1_u16: ExtensionTarget<D>,
    limb_2_u16: ExtensionTarget<D>,
    limb_3_u16: ExtensionTarget<D>,
    inverse: ExtensionTarget<D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) -> ExtensionTarget<D> {
    let base = F::from_canonical_u32(1 << 16);
    let limb_0_u32 = builder.mul_const_add_extension(base, limb_1_u16, limb_0_u16);
    let limb_1_u32 = builder.mul_const_add_extension(base, limb_3_u16, limb_2_u16);
    combine_u32s_check_canonical_circuit(builder, limb_0_u32, limb_1_u32, inverse, yield_constr)
}

/// Adds constraints to require that a pair of `u32`s, in little-endian order, represent a canonical
/// field element, i.e. that their combined value is less than `|F|`. Returns their combined value.
pub(crate) fn combine_u32s_check_canonical<F: Field, P: PackedField<Scalar = F>>(
    limb_0_u32: P,
    limb_1_u32: P,
    inverse: P,
    yield_constr: &mut ConstraintConsumer<P>,
) -> P {
    let u32_max = P::from(F::from_canonical_u32(u32::MAX));

    // This is zero if and only if the high limb is `u32::MAX`.
    let diff = u32_max - limb_1_u32;
    // If this is zero, the diff is invertible, so the high limb is not `u32::MAX`.
    let hi_not_max = inverse * diff - F::ONE;
    // If this is zero, either the high limb is not `u32::MAX`, or the low limb is zero.
    let hi_not_max_or_lo_zero = hi_not_max * limb_0_u32;

    yield_constr.constraint(hi_not_max_or_lo_zero);

    // Return the combined value.
    limb_0_u32 + limb_1_u32 * F::from_canonical_u64(1 << 32)
}

/// Adds constraints to require that a pair of `u32`s, in little-endian order, represent a canonical
/// field element, i.e. that their combined value is less than `|F|`. Returns their combined value.
pub(crate) fn combine_u32s_check_canonical_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    limb_0_u32: ExtensionTarget<D>,
    limb_1_u32: ExtensionTarget<D>,
    inverse: ExtensionTarget<D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) -> ExtensionTarget<D> {
    let one = builder.one_extension();
    let u32_max = builder.constant_extension(F::Extension::from_canonical_u32(u32::MAX));

    // This is zero if and only if the high limb is `u32::MAX`.
    let diff = builder.sub_extension(u32_max, limb_1_u32);
    // If this is zero, the diff is invertible, so the high limb is not `u32::MAX`.
    let hi_not_max = builder.mul_sub_extension(inverse, diff, one);
    // If this is zero, either the high limb is not `u32::MAX`, or the low limb is zero.
    let hi_not_max_or_lo_zero = builder.mul_extension(hi_not_max, limb_0_u32);

    yield_constr.constraint(builder, hi_not_max_or_lo_zero);

    // Return the combined value.
    builder.mul_const_add_extension(F::from_canonical_u64(1 << 32), limb_1_u32, limb_0_u32)
}
