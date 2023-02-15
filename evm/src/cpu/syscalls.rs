//! Handle instructions that are implemented in terms of system calls.
//!
//! These are usually the ones that are too complicated to implement in one CPU table row.

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use static_assertions::const_assert;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

// Copy the constant but make it `usize`.
const BYTES_PER_OFFSET: usize = crate::cpu::kernel::assembler::BYTES_PER_OFFSET as usize;
const_assert!(BYTES_PER_OFFSET < NUM_GP_CHANNELS); // Reserve one channel for stack push

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.syscall;

    // Look up the handler in memory
    let code_segment = P::Scalar::from_canonical_usize(Segment::Code as usize);
    let syscall_jumptable_start =
        P::Scalar::from_canonical_usize(KERNEL.global_labels["syscall_jumptable"]);
    let opcode: P = lv
        .opcode_bits
        .into_iter()
        .enumerate()
        .map(|(i, bit)| bit * P::Scalar::from_canonical_u64(1 << i))
        .sum();
    let opcode_handler_addr_start =
        syscall_jumptable_start + opcode * P::Scalar::from_canonical_usize(BYTES_PER_OFFSET);
    for (i, channel) in lv.mem_channels[0..BYTES_PER_OFFSET].iter().enumerate() {
        yield_constr.constraint(filter * (channel.used - P::ONES));
        yield_constr.constraint(filter * (channel.is_read - P::ONES));

        // Set kernel context and code segment
        yield_constr.constraint(filter * channel.addr_context);
        yield_constr.constraint(filter * (channel.addr_segment - code_segment));

        // Set address, using a separate channel for each of the `BYTES_PER_OFFSET` limbs.
        let limb_address = opcode_handler_addr_start + P::Scalar::from_canonical_usize(i);
        yield_constr.constraint(filter * (channel.addr_virtual - limb_address));
    }

    // Disable unused channels (the last channel is used to push to the stack)
    for channel in &lv.mem_channels[BYTES_PER_OFFSET..NUM_GP_CHANNELS - 1] {
        yield_constr.constraint(filter * channel.used);
    }

    // Set program counter to the handler address
    // The addresses are big-endian in memory
    let target = lv.mem_channels[0..BYTES_PER_OFFSET]
        .iter()
        .map(|channel| channel.value[0])
        .fold(P::ZEROS, |cumul, limb| {
            cumul * P::Scalar::from_canonical_u64(256) + limb
        });
    yield_constr.constraint_transition(filter * (nv.program_counter - target));
    // Set kernel mode
    yield_constr.constraint_transition(filter * (nv.is_kernel_mode - P::ONES));
    // Maintain current context
    yield_constr.constraint_transition(filter * (nv.context - lv.context));
    // Reset gas counter to zero.
    yield_constr.constraint_transition(filter * nv.gas);

    // This memory channel is constrained in `stack.rs`.
    let output = lv.mem_channels[NUM_GP_CHANNELS - 1].value;
    // Push to stack: current PC + 1 (limb 0), kernel flag (limb 1), gas counter (limbs 6 and 7).
    yield_constr.constraint(filter * (output[0] - (lv.program_counter + P::ONES)));
    yield_constr.constraint(filter * (output[1] - lv.is_kernel_mode));
    yield_constr.constraint(filter * (output[6] - lv.gas));
    // TODO: Range check `output[6]`.
    yield_constr.constraint(filter * output[7]); // High limb of gas is zero.

    // Zero the rest of that register
    for &limb in &output[2..6] {
        yield_constr.constraint(filter * limb);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = lv.op.syscall;

    // Look up the handler in memory
    let code_segment = F::from_canonical_usize(Segment::Code as usize);
    let syscall_jumptable_start = builder.constant_extension(
        F::from_canonical_usize(KERNEL.global_labels["syscall_jumptable"]).into(),
    );
    let opcode = lv
        .opcode_bits
        .into_iter()
        .rev()
        .fold(builder.zero_extension(), |cumul, bit| {
            builder.mul_const_add_extension(F::TWO, cumul, bit)
        });
    let opcode_handler_addr_start = builder.mul_const_add_extension(
        F::from_canonical_usize(BYTES_PER_OFFSET),
        opcode,
        syscall_jumptable_start,
    );
    for (i, channel) in lv.mem_channels[0..BYTES_PER_OFFSET].iter().enumerate() {
        {
            let constr = builder.mul_sub_extension(filter, channel.used, filter);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.mul_sub_extension(filter, channel.is_read, filter);
            yield_constr.constraint(builder, constr);
        }

        // Set kernel context and code segment
        {
            let constr = builder.mul_extension(filter, channel.addr_context);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.arithmetic_extension(
                F::ONE,
                -code_segment,
                filter,
                channel.addr_segment,
                filter,
            );
            yield_constr.constraint(builder, constr);
        }

        // Set address, using a separate channel for each of the `BYTES_PER_OFFSET` limbs.
        {
            let diff = builder.sub_extension(channel.addr_virtual, opcode_handler_addr_start);
            let constr = builder.arithmetic_extension(
                F::ONE,
                -F::from_canonical_usize(i),
                filter,
                diff,
                filter,
            );
            yield_constr.constraint(builder, constr);
        }
    }

    // Disable unused channels (the last channel is used to push to the stack)
    for channel in &lv.mem_channels[BYTES_PER_OFFSET..NUM_GP_CHANNELS - 1] {
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }

    // Set program counter to the handler address
    // The addresses are big-endian in memory
    {
        let target = lv.mem_channels[0..BYTES_PER_OFFSET]
            .iter()
            .map(|channel| channel.value[0])
            .fold(builder.zero_extension(), |cumul, limb| {
                builder.mul_const_add_extension(F::from_canonical_u64(256), cumul, limb)
            });
        let diff = builder.sub_extension(nv.program_counter, target);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint_transition(builder, constr);
    }
    // Set kernel mode
    {
        let constr = builder.mul_sub_extension(filter, nv.is_kernel_mode, filter);
        yield_constr.constraint_transition(builder, constr);
    }
    // Maintain current context
    {
        let diff = builder.sub_extension(nv.context, lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint_transition(builder, constr);
    }
    // Reset gas counter to zero.
    {
        let constr = builder.mul_extension(filter, nv.gas);
        yield_constr.constraint_transition(builder, constr);
    }

    // This memory channel is constrained in `stack.rs`.
    let output = lv.mem_channels[NUM_GP_CHANNELS - 1].value;
    // Push to stack: current PC + 1 (limb 0), kernel flag (limb 1), gas counter (limbs 6 and 7).
    {
        let pc_plus_1 = builder.add_const_extension(lv.program_counter, F::ONE);
        let diff = builder.sub_extension(output[0], pc_plus_1);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(output[1], lv.is_kernel_mode);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(output[6], lv.gas);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    // TODO: Range check `output[6]`.
    {
        // High limb of gas is zero.
        let constr = builder.mul_extension(filter, output[7]);
        yield_constr.constraint(builder, constr);
    }

    // Zero the rest of that register
    for &limb in &output[2..6] {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }
}
