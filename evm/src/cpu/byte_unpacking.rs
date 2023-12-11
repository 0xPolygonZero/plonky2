use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;

pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // The MSTORE_32BYTES opcodes are differentiated from MLOAD_32BYTES
    // by the 5th bit set to 0.
    let filter = lv.op.m_op_32bytes * (lv.opcode_bits[5] - P::ONES);
    let new_offset = nv.mem_channels[0].value[0];
    let virt = lv.mem_channels[2].value[0];
    // Read len from opcode bits and constrain the pushed new offset.
    let len_bits: P = lv.opcode_bits[..5]
        .iter()
        .enumerate()
        .map(|(i, &bit)| bit * P::Scalar::from_canonical_u64(1 << i))
        .sum();
    let len = len_bits + P::ONES;
    yield_constr.constraint(filter * (new_offset - virt - len));
}

pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // The MSTORE_32BYTES opcodes are differentiated from MLOAD_32BYTES
    // by the 5th bit set to 0.
    let filter =
        builder.mul_sub_extension(lv.op.m_op_32bytes, lv.opcode_bits[5], lv.op.m_op_32bytes);
    let new_offset = nv.mem_channels[0].value[0];
    let virt = lv.mem_channels[2].value[0];
    // Read len from opcode bits and constrain the pushed new offset.
    let len_bits = lv.opcode_bits[..5].iter().enumerate().fold(
        builder.zero_extension(),
        |cumul, (i, &bit)| {
            builder.mul_const_add_extension(F::from_canonical_u64(1 << i), bit, cumul)
        },
    );
    let diff = builder.sub_extension(new_offset, virt);
    let diff = builder.sub_extension(diff, len_bits);
    let constr = builder.mul_sub_extension(filter, diff, filter);
    yield_constr.constraint(builder, constr);
}
