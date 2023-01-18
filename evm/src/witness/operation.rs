use ethereum_types::{BigEndianHash, U256};
use itertools::Itertools;
use keccak_hash::keccak;
use plonky2::field::types::Field;

use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::assembler::BYTES_PER_OFFSET;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::cpu::simple_logic::eq_iszero::generate_pinv_diff;
use crate::generation::state::GenerationState;
use crate::memory::segments::Segment;
use crate::witness::errors::ProgramError;
use crate::witness::memory::MemoryAddress;
use crate::witness::util::{
    keccak_sponge_log, mem_read_code_with_log_and_fill, mem_read_gp_with_log_and_fill,
    mem_write_gp_log_and_fill, stack_pop_with_log_and_fill, stack_push_log_and_fill,
};
use crate::{arithmetic, logic};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operation {
    Iszero,
    Not,
    Byte,
    Syscall(u8),
    Eq,
    BinaryLogic(logic::Op),
    BinaryArithmetic(arithmetic::BinaryOperator),
    TernaryArithmetic(arithmetic::TernaryOperator),
    KeccakGeneral,
    ProverInput,
    Pop,
    Jump,
    Jumpi,
    Pc,
    Gas,
    Jumpdest,
    Push(u8),
    Dup(u8),
    Swap(u8),
    GetContext,
    SetContext,
    ConsumeGas,
    ExitKernel,
    MloadGeneral,
    MstoreGeneral,
}

pub(crate) fn generate_binary_logic_op<F: Field>(
    op: logic::Op,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(in0, log_in0), (in1, log_in1)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;
    let operation = logic::Operation::new(op, in0, in1);
    let log_out = stack_push_log_and_fill(state, &mut row, operation.result)?;

    state.traces.push_logic(operation);
    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_binary_arithmetic_op<F: Field>(
    operator: arithmetic::BinaryOperator,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(input0, log_in0), (input1, log_in1)] =
        stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;
    let operation = arithmetic::Operation::binary(operator, input0, input1);

    let log_out = stack_push_log_and_fill(state, &mut row, operation.result())?;

    if operator == arithmetic::BinaryOperator::Shl || operator == arithmetic::BinaryOperator::Shr {
        const LOOKUP_CHANNEL: usize = 2;
        let lookup_addr = MemoryAddress::new(0, Segment::ShiftTable, input0.low_u32() as usize);
        if input0.bits() <= 32 {
            let (_, read) =
                mem_read_gp_with_log_and_fill(LOOKUP_CHANNEL, lookup_addr, state, &mut row);
            state.traces.push_memory(read);
        } else {
            // The shift constraints still expect the address to be set, even though no read will occur.
            let mut channel = &mut row.mem_channels[LOOKUP_CHANNEL];
            channel.addr_context = F::from_canonical_usize(lookup_addr.context);
            channel.addr_segment = F::from_canonical_usize(lookup_addr.segment);
            channel.addr_virtual = F::from_canonical_usize(lookup_addr.virt);
        }
    }

    state.traces.push_arithmetic(operation);
    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_ternary_arithmetic_op<F: Field>(
    operator: arithmetic::TernaryOperator,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(input0, log_in0), (input1, log_in1), (input2, log_in2)] =
        stack_pop_with_log_and_fill::<3, _>(state, &mut row)?;
    let operation = arithmetic::Operation::ternary(operator, input0, input1, input2);
    let log_out = stack_push_log_and_fill(state, &mut row, operation.result())?;

    state.traces.push_arithmetic(operation);
    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_keccak_general<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    row.is_keccak_sponge = F::ONE;
    let [(context, log_in0), (segment, log_in1), (base_virt, log_in2), (len, log_in3)] =
        stack_pop_with_log_and_fill::<4, _>(state, &mut row)?;
    let len = len.as_usize();

    let base_address = MemoryAddress::new_u256s(context, segment, base_virt);
    let input = (0..len)
        .map(|i| {
            let address = MemoryAddress {
                virt: base_address.virt.saturating_add(i),
                ..base_address
            };
            let val = state.memory.get(address);
            val.as_u32() as u8
        })
        .collect_vec();
    log::debug!("Hashing {:?}", input);

    let hash = keccak(&input);
    let log_push = stack_push_log_and_fill(state, &mut row, hash.into_uint())?;

    keccak_sponge_log(state, base_address, input);

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_in3);
    state.traces.push_memory(log_push);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_prover_input<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let pc = state.registers.program_counter;
    let input_fn = &KERNEL.prover_inputs[&pc];
    let input = state.prover_input(input_fn);
    let write = stack_push_log_and_fill(state, &mut row, input)?;

    state.traces.push_memory(write);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_pop<F: Field>(
    state: &mut GenerationState<F>,
    row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    if state.registers.stack_len == 0 {
        return Err(ProgramError::StackUnderflow);
    }

    state.registers.stack_len -= 1;
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_jump<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(dst, log_in0)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let dst: u32 = dst
        .try_into()
        .map_err(|_| ProgramError::InvalidJumpDestination)?;

    let (jumpdest_bit, jumpdest_bit_log) = mem_read_gp_with_log_and_fill(
        NUM_GP_CHANNELS - 1,
        MemoryAddress::new(state.registers.context, Segment::JumpdestBits, dst as usize),
        state,
        &mut row,
    );
    if state.registers.is_kernel {
        // Don't actually do the read, just set the address, etc.
        let mut channel = &mut row.mem_channels[NUM_GP_CHANNELS - 1];
        channel.used = F::ZERO;
        channel.value[0] = F::ONE;

        row.mem_channels[1].value[0] = F::ONE;
    } else {
        if jumpdest_bit != U256::one() {
            return Err(ProgramError::InvalidJumpDestination);
        }
        state.traces.push_memory(jumpdest_bit_log);
    }

    // Extra fields required by the constraints.
    row.general.jumps_mut().should_jump = F::ONE;
    row.general.jumps_mut().cond_sum_pinv = F::ONE;

    state.traces.push_memory(log_in0);
    state.traces.push_cpu(row);
    state.registers.program_counter = dst as usize;
    Ok(())
}

pub(crate) fn generate_jumpi<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(dst, log_in0), (cond, log_in1)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;

    let should_jump = !cond.is_zero();
    if should_jump {
        row.general.jumps_mut().should_jump = F::ONE;
        let cond_sum_u64 = cond
            .0
            .into_iter()
            .map(|limb| ((limb as u32) as u64) + (limb >> 32))
            .sum();
        let cond_sum = F::from_canonical_u64(cond_sum_u64);
        row.general.jumps_mut().cond_sum_pinv = cond_sum.inverse();

        let dst: u32 = dst
            .try_into()
            .map_err(|_| ProgramError::InvalidJumpiDestination)?;
        state.registers.program_counter = dst as usize;
    } else {
        row.general.jumps_mut().should_jump = F::ZERO;
        row.general.jumps_mut().cond_sum_pinv = F::ZERO;
        state.registers.program_counter += 1;
    }

    let (jumpdest_bit, jumpdest_bit_log) = mem_read_gp_with_log_and_fill(
        NUM_GP_CHANNELS - 1,
        MemoryAddress::new(
            state.registers.context,
            Segment::JumpdestBits,
            dst.low_u32() as usize,
        ),
        state,
        &mut row,
    );
    if !should_jump || state.registers.is_kernel {
        // Don't actually do the read, just set the address, etc.
        let mut channel = &mut row.mem_channels[NUM_GP_CHANNELS - 1];
        channel.used = F::ZERO;
        channel.value[0] = F::ONE;
    } else {
        if jumpdest_bit != U256::one() {
            return Err(ProgramError::InvalidJumpiDestination);
        }
        state.traces.push_memory(jumpdest_bit_log);
    }

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_pc<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let write = stack_push_log_and_fill(state, &mut row, state.registers.program_counter.into())?;
    state.traces.push_memory(write);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_jumpdest<F: Field>(
    state: &mut GenerationState<F>,
    row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_get_context<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let ctx = state.registers.context.into();
    let write = stack_push_log_and_fill(state, &mut row, ctx)?;
    state.traces.push_memory(write);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_set_context<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(ctx, log_in)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let sp_to_save = state.registers.stack_len.into();
    let old_ctx = state.registers.context;
    let new_ctx = ctx.as_usize();

    let sp_field = ContextMetadata::StackSize as usize;
    let old_sp_addr = MemoryAddress::new(old_ctx, Segment::ContextMetadata, sp_field);
    let new_sp_addr = MemoryAddress::new(new_ctx, Segment::ContextMetadata, sp_field);

    let log_write_old_sp = mem_write_gp_log_and_fill(1, old_sp_addr, state, &mut row, sp_to_save);
    let (new_sp, log_read_new_sp) = mem_read_gp_with_log_and_fill(2, new_sp_addr, state, &mut row);

    state.registers.context = new_ctx;
    state.registers.stack_len = new_sp.as_usize();
    state.traces.push_memory(log_in);
    state.traces.push_memory(log_write_old_sp);
    state.traces.push_memory(log_read_new_sp);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_push<F: Field>(
    n: u8,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let code_context = state.registers.code_context();
    let num_bytes = n as usize + 1;
    let initial_offset = state.registers.program_counter + 1;
    let offsets = initial_offset..initial_offset + num_bytes;
    let mut addrs = offsets.map(|offset| MemoryAddress::new(code_context, Segment::Code, offset));

    // First read val without going through `mem_read_with_log` type methods, so we can pass it
    // to stack_push_log_and_fill.
    let bytes = (0..num_bytes)
        .map(|i| {
            state
                .memory
                .get(MemoryAddress::new(
                    code_context,
                    Segment::Code,
                    initial_offset + i,
                ))
                .as_u32() as u8
        })
        .collect_vec();

    let val = U256::from_big_endian(&bytes);
    let write = stack_push_log_and_fill(state, &mut row, val)?;

    // In the first cycle, we read up to NUM_GP_CHANNELS - 1 bytes, leaving the last GP channel
    // to push the result.
    for (i, addr) in (&mut addrs).take(NUM_GP_CHANNELS - 1).enumerate() {
        let (_, read) = mem_read_gp_with_log_and_fill(i, addr, state, &mut row);
        state.traces.push_memory(read);
    }
    state.traces.push_memory(write);
    state.traces.push_cpu(row);

    // In any subsequent cycles, we read up to 1 + NUM_GP_CHANNELS bytes.
    for mut addrs_chunk in &addrs.chunks(1 + NUM_GP_CHANNELS) {
        let mut row = CpuColumnsView::default();
        row.is_cpu_cycle = F::ONE;
        row.op.push = F::ONE;

        let first_addr = addrs_chunk.next().unwrap();
        let (_, first_read) = mem_read_code_with_log_and_fill(first_addr, state, &mut row);
        state.traces.push_memory(first_read);

        for (i, addr) in addrs_chunk.enumerate() {
            let (_, read) = mem_read_gp_with_log_and_fill(i, addr, state, &mut row);
            state.traces.push_memory(read);
        }

        state.traces.push_cpu(row);
    }

    Ok(())
}

pub(crate) fn generate_dup<F: Field>(
    n: u8,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let other_addr_lo = state
        .registers
        .stack_len
        .checked_sub(1 + (n as usize))
        .ok_or(ProgramError::StackUnderflow)?;
    let other_addr = MemoryAddress::new(state.registers.context, Segment::Stack, other_addr_lo);

    let (val, log_in) = mem_read_gp_with_log_and_fill(0, other_addr, state, &mut row);
    let log_out = stack_push_log_and_fill(state, &mut row, val)?;

    state.traces.push_memory(log_in);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_swap<F: Field>(
    n: u8,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let other_addr_lo = state
        .registers
        .stack_len
        .checked_sub(2 + (n as usize))
        .ok_or(ProgramError::StackUnderflow)?;
    let other_addr = MemoryAddress::new(state.registers.context, Segment::Stack, other_addr_lo);

    let [(in0, log_in0)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let (in1, log_in1) = mem_read_gp_with_log_and_fill(1, other_addr, state, &mut row);
    let log_out0 = mem_write_gp_log_and_fill(NUM_GP_CHANNELS - 2, other_addr, state, &mut row, in0);
    let log_out1 = stack_push_log_and_fill(state, &mut row, in1)?;

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_out0);
    state.traces.push_memory(log_out1);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_not<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(x, log_in)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let result = !x;
    let log_out = stack_push_log_and_fill(state, &mut row, result)?;

    state.traces.push_memory(log_in);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_byte<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(i, log_in0), (x, log_in1)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;

    let byte = if i < 32.into() {
        // byte(i) is the i'th little-endian byte; we want the i'th big-endian byte.
        x.byte(31 - i.as_usize())
    } else {
        0
    };
    let log_out = stack_push_log_and_fill(state, &mut row, byte.into())?;

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_iszero<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(x, log_in)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let is_zero = x.is_zero();
    let result = {
        let t: u64 = is_zero.into();
        t.into()
    };
    let log_out = stack_push_log_and_fill(state, &mut row, result)?;

    generate_pinv_diff(x, U256::zero(), &mut row);

    state.traces.push_memory(log_in);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_syscall<F: Field>(
    opcode: u8,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let handler_jumptable_addr = KERNEL.global_labels["syscall_jumptable"];
    let handler_addr_addr =
        handler_jumptable_addr + (opcode as usize) * (BYTES_PER_OFFSET as usize);
    assert_eq!(BYTES_PER_OFFSET, 3, "Code below assumes 3 bytes per offset");
    let (handler_addr0, log_in0) = mem_read_gp_with_log_and_fill(
        0,
        MemoryAddress::new(0, Segment::Code, handler_addr_addr),
        state,
        &mut row,
    );
    let (handler_addr1, log_in1) = mem_read_gp_with_log_and_fill(
        1,
        MemoryAddress::new(0, Segment::Code, handler_addr_addr + 1),
        state,
        &mut row,
    );
    let (handler_addr2, log_in2) = mem_read_gp_with_log_and_fill(
        2,
        MemoryAddress::new(0, Segment::Code, handler_addr_addr + 2),
        state,
        &mut row,
    );

    let handler_addr = (handler_addr0 << 16) + (handler_addr1 << 8) + handler_addr2;
    let new_program_counter = handler_addr.as_usize();

    let syscall_info = U256::from(state.registers.program_counter + 1)
        + (U256::from(u64::from(state.registers.is_kernel)) << 32);
    let log_out = stack_push_log_and_fill(state, &mut row, syscall_info)?;

    state.registers.program_counter = new_program_counter;
    log::debug!("Syscall to {}", KERNEL.offset_name(new_program_counter));
    state.registers.is_kernel = true;

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);

    Ok(())
}

pub(crate) fn generate_eq<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(in0, log_in0), (in1, log_in1)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;
    let eq = in0 == in1;
    let result = U256::from(u64::from(eq));
    let log_out = stack_push_log_and_fill(state, &mut row, result)?;

    generate_pinv_diff(in0, in1, &mut row);

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_exit_kernel<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(kexit_info, log_in)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let kexit_info_u64 = kexit_info.0[0];
    let program_counter = kexit_info_u64 as u32 as usize;
    let is_kernel_mode_val = (kexit_info_u64 >> 32) as u32;
    assert!(is_kernel_mode_val == 0 || is_kernel_mode_val == 1);
    let is_kernel_mode = is_kernel_mode_val != 0;

    state.registers.program_counter = program_counter;
    state.registers.is_kernel = is_kernel_mode;
    log::debug!(
        "Exiting to {}, is_kernel={}",
        KERNEL.offset_name(program_counter),
        is_kernel_mode
    );

    state.traces.push_memory(log_in);
    state.traces.push_cpu(row);

    Ok(())
}

pub(crate) fn generate_mload_general<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(context, log_in0), (segment, log_in1), (virt, log_in2)] =
        stack_pop_with_log_and_fill::<3, _>(state, &mut row)?;

    let (val, log_read) = mem_read_gp_with_log_and_fill(
        3,
        MemoryAddress::new_u256s(context, segment, virt),
        state,
        &mut row,
    );

    let log_out = stack_push_log_and_fill(state, &mut row, val)?;

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_read);
    state.traces.push_memory(log_out);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_mstore_general<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(context, log_in0), (segment, log_in1), (virt, log_in2), (val, log_in3)] =
        stack_pop_with_log_and_fill::<4, _>(state, &mut row)?;

    let address = MemoryAddress {
        context: context.as_usize(),
        segment: segment.as_usize(),
        virt: virt.as_usize(),
    };
    let log_write = mem_write_gp_log_and_fill(4, address, state, &mut row, val);

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_in3);
    state.traces.push_memory(log_write);
    state.traces.push_cpu(row);
    Ok(())
}
