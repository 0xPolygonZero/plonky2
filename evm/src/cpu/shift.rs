use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_shift = lv.op.shl + lv.op.shr;
    let val = lv.mem_channels[0];
    let displacement = lv.mem_channels[1]; // holds the shift displacement d
    let two_exp = lv.mem_channels[2]; // holds 2^d
    let output = lv.mem_channels[NUM_GP_CHANNELS - 1]; // should hold val * 2^d (mod 2^256)

    let shift_table_segment = P::Scalar::from_canonical_u64(Segment::ShiftTable as u64);

    // Value, displacement, 2^disp, and output channels must be used and read-only.
    yield_constr.constraint(is_shift * (val.used - P::ONES));
    yield_constr.constraint(is_shift * (val.is_read - P::ONES));
    yield_constr.constraint(is_shift * (displacement.used - P::ONES));
    yield_constr.constraint(is_shift * (displacement.is_read - P::ONES));
    yield_constr.constraint(is_shift * (output.used - P::ONES));
    yield_constr.constraint(is_shift * (output.is_read - P::ONES));
    yield_constr.constraint(is_shift * (two_exp.used - P::ONES));
    yield_constr.constraint(is_shift * (two_exp.is_read - P::ONES));

    let high_limbs_are_zero = lv.general.shift().displacement_high_limbs_are_zero;
    yield_constr
        .constraint(is_shift * (high_limbs_are_zero - high_limbs_are_zero * high_limbs_are_zero));

    let high_limbs_sum: P = displacement.value[1..].iter().copied().sum();
    yield_constr.constraint(is_shift * high_limbs_sum * high_limbs_are_zero);

    // When the shift displacement is < 2^32, constrain the two_exp
    // mem_channel to be the entry corresponding to `displacement` in
    // the shift table lookup (will be zero if displacement >= 256).
    let small_disp_filter = is_shift * high_limbs_are_zero;
    yield_constr.constraint(small_disp_filter * two_exp.addr_context); // kernel mode only
    yield_constr.constraint(small_disp_filter * (two_exp.addr_segment - shift_table_segment));
    yield_constr.constraint(small_disp_filter * (two_exp.addr_virtual - displacement.value[0]));

    // When the shift displacement is >= 2^32, constrain two_exp.value
    // to be zero.
    let large_disp_filter = is_shift * (P::ONES - high_limbs_are_zero);
    for &limb in &two_exp.value {
        yield_constr.constraint(large_disp_filter * limb);
    }

    // Other channels must be unused
    for chan in &lv.mem_channels[3..NUM_GP_CHANNELS - 1] {
        yield_constr.constraint(is_shift * chan.used); // channel is not used
    }

    // Cross-table lookup must connect the memory channels here to MUL
    // (in the case of left shift) or DIV (in the case of right shift)
    // in the arithmetic table. Specifically, the mapping is
    //
    // 0 -> 0  (value to be shifted is the same)
    // 2 -> 1  (two_exp becomes the multiplicand (resp. divisor))
    // last -> last  (output is the same)
}

pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_shift = builder.add_extension(lv.op.shl, lv.op.shr);
    let val = lv.mem_channels[0];
    let displacement = lv.mem_channels[1];
    let two_exp = lv.mem_channels[2];
    let output = lv.mem_channels[NUM_GP_CHANNELS - 1];

    let shift_table_segment = F::from_canonical_u64(Segment::ShiftTable as u64);

    let t = builder.arithmetic_extension(F::ONE, -F::ONE, is_shift, val.used, is_shift);
    yield_constr.constraint(builder, t);
    let t = builder.arithmetic_extension(F::ONE, -F::ONE, is_shift, val.is_read, is_shift);
    yield_constr.constraint(builder, t);
    let t = builder.arithmetic_extension(F::ONE, -F::ONE, is_shift, displacement.used, is_shift);
    yield_constr.constraint(builder, t);
    let t = builder.arithmetic_extension(F::ONE, -F::ONE, is_shift, displacement.is_read, is_shift);
    yield_constr.constraint(builder, t);
    let t = builder.arithmetic_extension(F::ONE, -F::ONE, is_shift, output.used, is_shift);
    yield_constr.constraint(builder, t);
    let t = builder.arithmetic_extension(F::ONE, -F::ONE, is_shift, output.is_read, is_shift);
    yield_constr.constraint(builder, t);
    let t = builder.arithmetic_extension(F::ONE, -F::ONE, is_shift, two_exp.used, is_shift);
    yield_constr.constraint(builder, t);
    let t = builder.arithmetic_extension(F::ONE, -F::ONE, is_shift, two_exp.is_read, is_shift);
    yield_constr.constraint(builder, t);

    let high_limbs_are_zero = lv.general.shift().displacement_high_limbs_are_zero;
    let t = builder.mul_sub_extension(
        high_limbs_are_zero,
        high_limbs_are_zero,
        high_limbs_are_zero,
    );
    let t = builder.mul_extension(t, is_shift);
    yield_constr.constraint(builder, t);

    let high_limbs_sum = builder.add_many_extension(&displacement.value[1..]);
    let t = builder.mul_many_extension(&[is_shift, high_limbs_sum, high_limbs_are_zero]);
    yield_constr.constraint(builder, t);

    let small_disp_filter = builder.mul_extension(is_shift, high_limbs_are_zero);
    let t = builder.mul_extension(small_disp_filter, two_exp.addr_context);
    yield_constr.constraint(builder, t);
    let t = builder.arithmetic_extension(
        F::ONE,
        -shift_table_segment,
        small_disp_filter,
        two_exp.addr_segment,
        small_disp_filter,
    );
    yield_constr.constraint(builder, t);
    let t = builder.sub_extension(two_exp.addr_virtual, displacement.value[0]);
    let t = builder.mul_extension(small_disp_filter, t);
    yield_constr.constraint(builder, t);

    let large_disp_filter =
        builder.arithmetic_extension(-F::ONE, F::ONE, is_shift, high_limbs_are_zero, is_shift);
    for &limb in &two_exp.value {
        let t = builder.mul_extension(large_disp_filter, limb);
        yield_constr.constraint(builder, t);
    }

    for chan in &lv.mem_channels[3..NUM_GP_CHANNELS - 1] {
        let t = builder.mul_extension(is_shift, chan.used);
        yield_constr.constraint(builder, t);
    }
}
