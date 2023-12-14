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

    // The address to write to is stored in the first memory channel.
    // It contains virt, segment, ctx in its first 3 limbs, and 0 otherwise.
    // The new address is identical, except for its `virtual` limb that is increased by the corresponding `len` offset.
    let new_addr = nv.mem_channels[0].value;
    let written_addr = lv.mem_channels[0].value;

    // Read len from opcode bits and constrain the pushed new offset.
    let len_bits: P = lv.opcode_bits[..5]
        .iter()
        .enumerate()
        .map(|(i, &bit)| bit * P::Scalar::from_canonical_u64(1 << i))
        .sum();
    let len = len_bits + P::ONES;

    // Check that `virt` is increased properly.
    yield_constr.constraint(filter * (new_addr[0] - written_addr[0] - len));

    // Check that `segment` and `ctx` do not change.
    yield_constr.constraint(filter * (new_addr[1] - written_addr[1]));
    yield_constr.constraint(filter * (new_addr[2] - written_addr[2]));

    // Check that the rest of the returned address is null.
    for &limb in &new_addr[3..] {
        yield_constr.constraint(filter * limb);
    }
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

    // The address to write to is stored in the first memory channel.
    // It contains virt, segment, ctx in its first 3 limbs, and 0 otherwise.
    // The new address is identical, except for its `virtual` limb that is increased by the corresponding `len` offset.
    let new_addr = nv.mem_channels[0].value;
    let written_addr = lv.mem_channels[0].value;

    // Read len from opcode bits and constrain the pushed new offset.
    let len_bits = lv.opcode_bits[..5].iter().enumerate().fold(
        builder.zero_extension(),
        |cumul, (i, &bit)| {
            builder.mul_const_add_extension(F::from_canonical_u64(1 << i), bit, cumul)
        },
    );

    // Check that `virt` is increased properly.
    let diff = builder.sub_extension(new_addr[0], written_addr[0]);
    let diff = builder.sub_extension(diff, len_bits);
    let constr = builder.mul_sub_extension(filter, diff, filter);
    yield_constr.constraint(builder, constr);

    // Check that `segment` and `ctx` do not change.
    {
        let diff = builder.sub_extension(new_addr[1], written_addr[1]);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);

        let diff = builder.sub_extension(new_addr[2], written_addr[2]);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // Check that the rest of the returned address is null.
    for &limb in &new_addr[3..] {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }
}
