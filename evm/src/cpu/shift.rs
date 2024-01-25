use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

/// Evaluates constraints for shift operations on the CPU side:
/// the shifting factor is read from memory when displacement < 2^32.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_shift = lv.op.shift;
    let displacement = lv.mem_channels[0]; // holds the shift displacement d
    let two_exp = lv.mem_channels[2]; // holds 2^d

    // Not needed here; val is the input and we're verifying that output is
    // val * 2^d (mod 2^256)
    // let val = lv.mem_channels[0];
    // let output = lv.mem_channels[NUM_GP_CHANNELS - 1];

    let shift_table_segment = P::Scalar::from_canonical_usize(Segment::ShiftTable.unscale());

    // Only lookup the shifting factor when displacement is < 2^32.
    // two_exp.used is true (1) if the high limbs of the displacement are
    // zero and false (0) otherwise.
    let high_limbs_are_zero = two_exp.used;
    yield_constr.constraint(is_shift * high_limbs_are_zero * (two_exp.is_read - P::ONES));

    let high_limbs_sum: P = displacement.value[1..].iter().copied().sum();
    let high_limbs_sum_inv = lv.general.shift().high_limb_sum_inv;
    // Verify that high_limbs_are_zero = 0 implies high_limbs_sum != 0 and
    // high_limbs_are_zero = 1 implies high_limbs_sum = 0.
    let t = high_limbs_sum * high_limbs_sum_inv - (P::ONES - high_limbs_are_zero);
    yield_constr.constraint(is_shift * t);
    yield_constr.constraint(is_shift * high_limbs_sum * high_limbs_are_zero);

    // When the shift displacement is < 2^32, constrain the two_exp
    // mem_channel to be the entry corresponding to `displacement` in
    // the shift table lookup (will be zero if displacement >= 256).
    yield_constr.constraint(is_shift * two_exp.addr_context); // read from kernel memory
    yield_constr.constraint(is_shift * (two_exp.addr_segment - shift_table_segment));
    yield_constr.constraint(is_shift * (two_exp.addr_virtual - displacement.value[0]));

    // Other channels must be unused
    for chan in &lv.mem_channels[3..NUM_GP_CHANNELS] {
        yield_constr.constraint(is_shift * chan.used); // channel is not used
    }

    // Cross-table lookup must connect the memory channels here to MUL
    // (in the case of left shift) or DIV (in the case of right shift)
    // in the arithmetic table. Specifically, the mapping is
    //
    // 1 -> 0  (value to be shifted is the same)
    // 2 -> 1  (two_exp becomes the multiplicand (resp. divisor))
    // next_0 -> next_0  (output is the same)
}

/// Circuit version.
/// Evaluates constraints for shift operations on the CPU side:
/// the shifting factor is read from memory when displacement < 2^32.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_shift = lv.op.shift;
    let displacement = lv.mem_channels[0];
    let two_exp = lv.mem_channels[2];

    let shift_table_segment = F::from_canonical_usize(Segment::ShiftTable.unscale());

    // Only lookup the shifting factor when displacement is < 2^32.
    // two_exp.used is true (1) if the high limbs of the displacement are
    // zero and false (0) otherwise.
    let high_limbs_are_zero = two_exp.used;
    let one = builder.one_extension();
    let t = builder.sub_extension(two_exp.is_read, one);
    let t = builder.mul_extension(high_limbs_are_zero, t);
    let t = builder.mul_extension(is_shift, t);
    yield_constr.constraint(builder, t);

    let high_limbs_sum = builder.add_many_extension(&displacement.value[1..]);
    let high_limbs_sum_inv = lv.general.shift().high_limb_sum_inv;
    // Verify that high_limbs_are_zero = 0 implies high_limbs_sum != 0 and
    // high_limbs_are_zero = 1 implies high_limbs_sum = 0.
    let t = builder.one_extension();
    let t = builder.sub_extension(t, high_limbs_are_zero);
    let t = builder.mul_sub_extension(high_limbs_sum, high_limbs_sum_inv, t);
    let t = builder.mul_extension(is_shift, t);
    yield_constr.constraint(builder, t);

    let t = builder.mul_many_extension([is_shift, high_limbs_sum, high_limbs_are_zero]);
    yield_constr.constraint(builder, t);

    // When the shift displacement is < 2^32, constrain the two_exp
    // mem_channel to be the entry corresponding to `displacement` in
    // the shift table lookup (will be zero if displacement >= 256).
    let t = builder.mul_extension(is_shift, two_exp.addr_context);
    yield_constr.constraint(builder, t);
    let t = builder.arithmetic_extension(
        F::ONE,
        -shift_table_segment,
        is_shift,
        two_exp.addr_segment,
        is_shift,
    );
    yield_constr.constraint(builder, t);
    let t = builder.sub_extension(two_exp.addr_virtual, displacement.value[0]);
    let t = builder.mul_extension(is_shift, t);
    yield_constr.constraint(builder, t);

    // Other channels must be unused
    for chan in &lv.mem_channels[3..NUM_GP_CHANNELS] {
        let t = builder.mul_extension(is_shift, chan.used);
        yield_constr.constraint(builder, t);
    }
}
