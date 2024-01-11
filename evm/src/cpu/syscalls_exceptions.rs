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

/// Evaluates constraints for syscalls and exceptions.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter_syscall = lv.op.syscall;
    let filter_exception = lv.op.exception;
    let total_filter = filter_syscall + filter_exception;

    // First, constrain filters to be boolean.
    // Ensuring they are mutually exclusive is done in other modules
    // through the `is_cpu_cycle` variable.
    yield_constr.constraint(filter_syscall * (filter_syscall - P::ONES));
    yield_constr.constraint(filter_exception * (filter_exception - P::ONES));

    // If exception, ensure we are not in kernel mode
    yield_constr.constraint(filter_exception * lv.is_kernel_mode);

    // Get the exception code as an value in {0, ..., 7}.
    let exc_code_bits = lv.general.exception().exc_code_bits;
    let exc_code: P = exc_code_bits
        .into_iter()
        .enumerate()
        .map(|(i, bit)| bit * P::Scalar::from_canonical_u64(1 << i))
        .sum();
    // Ensure that all bits are either 0 or 1.
    for bit in exc_code_bits {
        yield_constr.constraint(filter_exception * bit * (bit - P::ONES));
    }

    // Look up the handler in memory
    let code_segment = P::Scalar::from_canonical_usize(Segment::Code.unscale());

    let opcode: P = lv
        .opcode_bits
        .into_iter()
        .enumerate()
        .map(|(i, bit)| bit * P::Scalar::from_canonical_u64(1 << i))
        .sum();

    // Syscall handler
    let syscall_jumptable_start =
        P::Scalar::from_canonical_usize(KERNEL.global_labels["syscall_jumptable"]);
    let opcode_handler_addr_start =
        syscall_jumptable_start + opcode * P::Scalar::from_canonical_usize(BYTES_PER_OFFSET);
    // Exceptions handler
    let exc_jumptable_start =
        P::Scalar::from_canonical_usize(KERNEL.global_labels["exception_jumptable"]);
    let exc_handler_addr_start =
        exc_jumptable_start + exc_code * P::Scalar::from_canonical_usize(BYTES_PER_OFFSET);

    let jumpdest_channel = lv.mem_channels[1];

    // Set `used` and `is_read`.
    // The channel is not used: the reads will be done with the byte packing CTL.
    yield_constr.constraint(total_filter * (jumpdest_channel.used));
    yield_constr.constraint(total_filter * (jumpdest_channel.is_read - P::ONES));

    // Set kernel context and code segment
    yield_constr.constraint(total_filter * jumpdest_channel.addr_context);
    yield_constr.constraint(total_filter * (jumpdest_channel.addr_segment - code_segment));

    // Set address.
    yield_constr
        .constraint(filter_syscall * (jumpdest_channel.addr_virtual - opcode_handler_addr_start));
    yield_constr
        .constraint(filter_exception * (jumpdest_channel.addr_virtual - exc_handler_addr_start));

    // Set higher limbs to zero.
    for &limb in &jumpdest_channel.value[1..] {
        yield_constr.constraint(total_filter * limb);
    }

    // Disable unused channels
    for channel in &lv.mem_channels[2..NUM_GP_CHANNELS] {
        yield_constr.constraint(total_filter * channel.used);
    }

    // Set program counter to the handler address
    yield_constr
        .constraint_transition(total_filter * (nv.program_counter - jumpdest_channel.value[0]));
    // Set kernel mode
    yield_constr.constraint_transition(total_filter * (nv.is_kernel_mode - P::ONES));
    // Reset gas counter to zero.
    yield_constr.constraint_transition(total_filter * nv.gas);

    let output = nv.mem_channels[0].value;
    // New top of the stack: current PC + 1 (limb 0), kernel flag (limb 1), gas counter (limbs 6 and 7).
    yield_constr.constraint(filter_syscall * (output[0] - (lv.program_counter + P::ONES)));
    yield_constr.constraint(filter_exception * (output[0] - lv.program_counter));
    // Check the kernel mode, for syscalls only
    yield_constr.constraint(filter_syscall * (output[1] - lv.is_kernel_mode));
    yield_constr.constraint(total_filter * (output[6] - lv.gas));
    yield_constr.constraint(total_filter * output[7]); // High limb of gas is zero.

    // Zero the rest of that register
    // output[1] is 0 for exceptions, but not for syscalls
    yield_constr.constraint(filter_exception * output[1]);
    for &limb in &output[2..6] {
        yield_constr.constraint(total_filter * limb);
    }
}

/// Circuit version of `eval_packed`.
/// Evaluates constraints for syscalls and exceptions.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter_syscall = lv.op.syscall;
    let filter_exception = lv.op.exception;
    let total_filter = builder.add_extension(filter_syscall, filter_exception);

    // First, constrain filters to be boolean.
    // Ensuring they are mutually exclusive is done in other modules
    // through the `is_cpu_cycle` variable.
    let constr = builder.mul_sub_extension(filter_syscall, filter_syscall, filter_syscall);
    yield_constr.constraint(builder, constr);
    let constr = builder.mul_sub_extension(filter_exception, filter_exception, filter_exception);
    yield_constr.constraint(builder, constr);

    // Ensure that, if exception, we are not in kernel mode
    let constr = builder.mul_extension(filter_exception, lv.is_kernel_mode);
    yield_constr.constraint(builder, constr);

    let exc_code_bits = lv.general.exception().exc_code_bits;
    let exc_code =
        exc_code_bits
            .into_iter()
            .enumerate()
            .fold(builder.zero_extension(), |cumul, (i, bit)| {
                builder.mul_const_add_extension(F::from_canonical_u64(1 << i), bit, cumul)
            });

    // Ensure that all bits are either 0 or 1.
    for bit in exc_code_bits {
        let constr = builder.mul_sub_extension(bit, bit, bit);
        let constr = builder.mul_extension(filter_exception, constr);
        yield_constr.constraint(builder, constr);
    }

    // Look up the handler in memory
    let code_segment = F::from_canonical_usize(Segment::Code.unscale());

    let opcode = lv
        .opcode_bits
        .into_iter()
        .rev()
        .fold(builder.zero_extension(), |cumul, bit| {
            builder.mul_const_add_extension(F::TWO, cumul, bit)
        });

    // Syscall handler
    let syscall_jumptable_start = builder.constant_extension(
        F::from_canonical_usize(KERNEL.global_labels["syscall_jumptable"]).into(),
    );
    let opcode_handler_addr_start = builder.mul_const_add_extension(
        F::from_canonical_usize(BYTES_PER_OFFSET),
        opcode,
        syscall_jumptable_start,
    );

    // Exceptions handler
    let exc_jumptable_start = builder.constant_extension(
        F::from_canonical_usize(KERNEL.global_labels["exception_jumptable"]).into(),
    );
    let exc_handler_addr_start = builder.mul_const_add_extension(
        F::from_canonical_usize(BYTES_PER_OFFSET),
        exc_code,
        exc_jumptable_start,
    );

    let jumpdest_channel = lv.mem_channels[1];

    // Set `used` and `is_read`.
    // The channel is not used: the reads will be done with the byte packing CTL.
    {
        let constr = builder.mul_extension(total_filter, jumpdest_channel.used);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr =
            builder.mul_sub_extension(total_filter, jumpdest_channel.is_read, total_filter);
        yield_constr.constraint(builder, constr);
    }

    // Set kernel context and code segment
    {
        let constr = builder.mul_extension(total_filter, jumpdest_channel.addr_context);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.arithmetic_extension(
            F::ONE,
            -code_segment,
            total_filter,
            jumpdest_channel.addr_segment,
            total_filter,
        );
        yield_constr.constraint(builder, constr);
    }

    // Set address.
    {
        let diff_syscall =
            builder.sub_extension(jumpdest_channel.addr_virtual, opcode_handler_addr_start);
        let constr = builder.mul_extension((filter_syscall), diff_syscall);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff_exception =
            builder.sub_extension(jumpdest_channel.addr_virtual, exc_handler_addr_start);
        let constr = builder.mul_extension(filter_exception, diff_exception);
        yield_constr.constraint(builder, constr);
    }

    // Set higher limbs to zero.
    for &limb in &jumpdest_channel.value[1..] {
        let constr = builder.mul_extension(total_filter, limb);
        yield_constr.constraint(builder, constr);
    }

    // Disable unused channels
    for channel in &lv.mem_channels[2..NUM_GP_CHANNELS] {
        let constr = builder.mul_extension(total_filter, channel.used);
        yield_constr.constraint(builder, constr);
    }

    // Set program counter to the handler address
    // The addresses are big-endian in memory
    {
        let diff = builder.sub_extension(nv.program_counter, jumpdest_channel.value[0]);
        let constr = builder.mul_extension(total_filter, diff);
        yield_constr.constraint_transition(builder, constr);
    }
    // Set kernel mode
    {
        let constr = builder.mul_sub_extension(total_filter, nv.is_kernel_mode, total_filter);
        yield_constr.constraint_transition(builder, constr);
    }
    // Reset gas counter to zero.
    {
        let constr = builder.mul_extension(total_filter, nv.gas);
        yield_constr.constraint_transition(builder, constr);
    }

    // New top of the stack.
    let output = nv.mem_channels[0].value;
    // Push to stack (syscall): current PC + 1 (limb 0), kernel flag (limb 1), gas counter (limbs 6 and 7).
    {
        let pc_plus_1 = builder.add_const_extension(lv.program_counter, F::ONE);
        let diff = builder.sub_extension(output[0], pc_plus_1);
        let constr = builder.mul_extension(filter_syscall, diff);
        yield_constr.constraint(builder, constr);
    }
    // Push to stack (exception): current PC (limb 0), kernel flag (limb 1), gas counter (limbs 6 and 7).
    {
        let diff = builder.sub_extension(output[0], lv.program_counter);
        let constr = builder.mul_extension(filter_exception, diff);
        yield_constr.constraint(builder, constr);
    }
    // Push to stack(exception): current PC (limb 0), gas counter (limbs 6 and 7).
    {
        let diff = builder.sub_extension(output[1], lv.is_kernel_mode);
        let constr = builder.mul_extension(filter_syscall, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        let diff = builder.sub_extension(output[6], lv.gas);
        let constr = builder.mul_extension(total_filter, diff);
        yield_constr.constraint(builder, constr);
    }
    {
        // High limb of gas is zero.
        let constr = builder.mul_extension(total_filter, output[7]);
        yield_constr.constraint(builder, constr);
    }

    // Zero the rest of that register
    let constr = builder.mul_extension(filter_exception, output[1]);
    yield_constr.constraint(builder, constr);
    for &limb in &output[2..6] {
        let constr = builder.mul_extension(total_filter, limb);
        yield_constr.constraint(builder, constr);
    }
}
