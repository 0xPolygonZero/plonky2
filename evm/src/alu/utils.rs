use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::alu::columns;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

/// NB: Tests for equality, but only on the assumption that the limbs
/// in `larger` are all at least as big as those in `smaller`, and
/// that the limbs in `larger are at most (LIMB_BITS + 1) bits.
pub fn eval_packed_generic_are_equal<P: PackedField>(
    yield_constr: &mut ConstraintConsumer<P>,
    is_op: P,
    larger: &[P; columns::N_LIMBS_32],
    smaller: &[P; columns::N_LIMBS_32],
) {
    let overflow = P::Scalar::from_canonical_u64(1 << 32);
    let overflow_inv = overflow.inverse();
    let mut cy = P::ZEROS;
    for &(a, b) in larger.zip(*smaller).iter() {
        // t should be either 0 or 2^LIMB_BITS
        let t = cy + a - b;
        yield_constr.constraint(is_op * t * (overflow - t));
        // cy <-- 0 or 1
        cy = t * overflow_inv;
    }
}

/// NB: Tests for equality, but only on the assumption that the limbs
/// in `larger` are all at least as big as those in `smaller`, and
/// that the limbs in `larger are at most (LIMB_BITS + 1) bits.
pub fn eval_ext_circuit_are_equal<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    is_op: ExtensionTarget<D>,
    larger: &[ExtensionTarget<D>; columns::N_LIMBS_32],
    smaller: &[ExtensionTarget<D>; columns::N_LIMBS_32],
) {
    // 2^32 in the base field
    let overflow_base = F::from_canonical_u64(1 << 32);
    // 2^32 in the extension field as an ExtensionTarget
    let overflow = builder.constant_extension(F::Extension::from(overflow_base));
    // 2^-32 in the base field.
    let overflow_inv = F::inverse_2exp(32);

    let mut cy = builder.zero_extension();
    for &(a, b) in larger.zip(*smaller).iter() {
        // t0 = cy + a
        let t0 = builder.add_extension(cy, a);
        // t  = t0 - b
        let t = builder.sub_extension(t0, b);
        // t1 = overflow - t
        let t1 = builder.sub_extension(overflow, t);
        // t2 = t * t1
        let t2 = builder.mul_extension(t, t1);

        let filtered_limb_constraint = builder.mul_extension(is_op, t2);
        yield_constr.constraint(builder, filtered_limb_constraint);

        cy = builder.mul_const_extension(overflow_inv, t);
    }
}
