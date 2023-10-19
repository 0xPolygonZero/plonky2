use ethereum_types::{BigEndianHash, U256};
use itertools::Itertools;
use keccak_hash::keccak;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use super::util::{byte_packing_log, byte_unpacking_log, push_no_write, push_with_write};
use crate::arithmetic::BinaryOperator;
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::assembler::BYTES_PER_OFFSET;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::cpu::simple_logic::eq_iszero::generate_pinv_diff;
use crate::cpu::stack_bounds::MAX_USER_STACK_SIZE;
use crate::extension_tower::BN_BASE;
use crate::generation::state::GenerationState;
use crate::memory::segments::Segment;
use crate::poseidon::columns::POSEIDON_SPONGE_RATE;
use crate::util::u256_to_usize;
use crate::witness::errors::MemoryError::{ContextTooLarge, SegmentTooLarge, VirtTooLarge};
use crate::witness::errors::ProgramError;
use crate::witness::errors::ProgramError::MemoryError;
use crate::witness::memory::{MemoryAddress, MemoryChannel, MemoryOp, MemoryOpKind};
use crate::witness::operation::MemoryChannel::GeneralPurpose;
use crate::witness::util::{
    compute_poseidon, keccak_sponge_log, mem_read_gp_with_log_and_fill, mem_write_gp_log_and_fill,
    poseidon_sponge_log, stack_pop_with_log_and_fill,
};
use crate::{arithmetic, logic};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operation {
    Iszero,
    Not,
    Syscall(u8, usize, bool), // (syscall number, minimum stack length, increases stack length)
    Eq,
    BinaryLogic(logic::Op),
    BinaryArithmetic(arithmetic::BinaryOperator),
    TernaryArithmetic(arithmetic::TernaryOperator),
    KeccakGeneral,
    PoseidonGeneral,
    ProverInput,
    Pop,
    Jump,
    Jumpi,
    Pc,
    Jumpdest,
    Push(u8),
    Dup(u8),
    Swap(u8),
    GetContext,
    SetContext,
    Mload32Bytes,
    Mstore32Bytes,
    ExitKernel,
    MloadGeneral,
    MstoreGeneral,
}

pub(crate) fn generate_binary_logic_op<F: Field>(
    op: logic::Op,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(in0, _), (in1, log_in1)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;
    let operation = logic::Operation::new(op, in0, in1);

    push_no_write(state, &mut row, operation.result, Some(NUM_GP_CHANNELS - 1));

    state.traces.push_logic(operation);
    state.traces.push_memory(log_in1);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_binary_arithmetic_op<F: Field>(
    operator: arithmetic::BinaryOperator,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(input0, _), (input1, log_in1)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;
    let operation = arithmetic::Operation::binary(operator, input0, input1);

    if operator == arithmetic::BinaryOperator::AddFp254
        || operator == arithmetic::BinaryOperator::MulFp254
        || operator == arithmetic::BinaryOperator::SubFp254
    {
        let channel = &mut row.mem_channels[2];

        let val_limbs: [u64; 4] = BN_BASE.0;
        for (i, limb) in val_limbs.into_iter().enumerate() {
            channel.value[2 * i] = F::from_canonical_u32(limb as u32);
            channel.value[2 * i + 1] = F::from_canonical_u32((limb >> 32) as u32);
        }
    }

    push_no_write(
        state,
        &mut row,
        operation.result(),
        Some(NUM_GP_CHANNELS - 1),
    );

    state.traces.push_arithmetic(operation);
    state.traces.push_memory(log_in1);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_ternary_arithmetic_op<F: Field>(
    operator: arithmetic::TernaryOperator,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(input0, _), (input1, log_in1), (input2, log_in2)] =
        stack_pop_with_log_and_fill::<3, _>(state, &mut row)?;
    let operation = arithmetic::Operation::ternary(operator, input0, input1, input2);

    push_no_write(
        state,
        &mut row,
        operation.result(),
        Some(NUM_GP_CHANNELS - 1),
    );

    state.traces.push_arithmetic(operation);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_keccak_general<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    row.is_keccak_sponge = F::ONE;
    let [(context, _), (segment, log_in1), (base_virt, log_in2), (len, log_in3)] =
        stack_pop_with_log_and_fill::<4, _>(state, &mut row)?;
    let len = u256_to_usize(len)?;

    let base_address = MemoryAddress::new_u256s(context, segment, base_virt)?;
    let input = (0..len)
        .map(|i| {
            let address = MemoryAddress {
                virt: base_address.virt.saturating_add(i),
                ..base_address
            };
            let val = state.memory.get(address);
            val.low_u32() as u8
        })
        .collect_vec();
    log::debug!("Hashing {:?}", input);

    let hash = keccak(&input);
    push_no_write(state, &mut row, hash.into_uint(), Some(NUM_GP_CHANNELS - 1));

    keccak_sponge_log(state, base_address, input);

    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_in3);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_poseidon_general<F: RichField>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    row.op.poseidon_general = F::ONE;
    let [(context, _), (segment, log_in1), (base_virt, log_in2), (len, log_in3)] =
        stack_pop_with_log_and_fill::<4, _>(state, &mut row)?;
    let len = u256_to_usize(len)?;

    let base_address = MemoryAddress::new_u256s(context, segment, base_virt)?;
    let input = (0..len)
        .map(|i| {
            let address = MemoryAddress {
                virt: base_address.virt.saturating_add(i),
                ..base_address
            };
            let val = state.memory.get(address);

            val.0[0]
        })
        .collect_vec();
    log::debug!("Hashing {:?}", input);

    let mut padded_input = input.clone();
    if padded_input.len() % POSEIDON_SPONGE_RATE == POSEIDON_SPONGE_RATE - 1 {
        padded_input.push(1);
    } else {
        padded_input.push(1);
        while (padded_input.len() + 1) % plonky2::hash::poseidon::SPONGE_RATE != 0 {
            padded_input.push(0);
        }
        padded_input.push(1);
    }

    poseidon_sponge_log(state, base_address, input.clone(), padded_input.clone());

    let padded_input = padded_input
        .iter()
        .map(|&elt| F::from_canonical_u64(elt))
        .collect::<Vec<F>>();
    let hash: [u64; 4] = compute_poseidon(padded_input)
        .iter()
        .map(|&elt| elt.to_canonical_u64())
        .collect::<Vec<u64>>()
        .try_into()
        .unwrap();
    push_no_write(state, &mut row, U256(hash), Some(NUM_GP_CHANNELS - 1));

    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_in3);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_prover_input<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let pc = state.registers.program_counter;
    let input_fn = &KERNEL.prover_inputs[&pc];
    let input = state.prover_input(input_fn)?;
    push_with_write(state, &mut row, input)?;
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_pop<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(_, _)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;

    state.traces.push_cpu(row);

    Ok(())
}

pub(crate) fn generate_jump<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(dst, _)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;

    let dst: u32 = dst
        .try_into()
        .map_err(|_| ProgramError::InvalidJumpDestination)?;

    let (jumpdest_bit, jumpdest_bit_log) = mem_read_gp_with_log_and_fill(
        NUM_GP_CHANNELS - 1,
        MemoryAddress::new(state.registers.context, Segment::JumpdestBits, dst as usize),
        state,
        &mut row,
    );

    row.mem_channels[1].value[0] = F::ONE;

    if state.registers.is_kernel {
        // Don't actually do the read, just set the address, etc.
        let channel = &mut row.mem_channels[NUM_GP_CHANNELS - 1];
        channel.used = F::ZERO;
        channel.value[0] = F::ONE;
    } else {
        if jumpdest_bit != U256::one() {
            return Err(ProgramError::InvalidJumpDestination);
        }
        state.traces.push_memory(jumpdest_bit_log);
    }

    // Extra fields required by the constraints.
    row.general.jumps_mut().should_jump = F::ONE;
    row.general.jumps_mut().cond_sum_pinv = F::ONE;

    let diff = row.stack_len - F::ONE;
    if let Some(inv) = diff.try_inverse() {
        row.general.stack_mut().stack_inv = inv;
        row.general.stack_mut().stack_inv_aux = F::ONE;
    } else {
        row.general.stack_mut().stack_inv = F::ZERO;
        row.general.stack_mut().stack_inv_aux = F::ZERO;
    }

    state.traces.push_cpu(row);
    state.jump_to(dst as usize)?;
    Ok(())
}

pub(crate) fn generate_jumpi<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(dst, _), (cond, log_cond)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;

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
        state.jump_to(dst as usize)?;
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
        let channel = &mut row.mem_channels[NUM_GP_CHANNELS - 1];
        channel.used = F::ZERO;
        channel.value[0] = F::ONE;
    } else {
        if jumpdest_bit != U256::one() {
            return Err(ProgramError::InvalidJumpiDestination);
        }
        state.traces.push_memory(jumpdest_bit_log);
    }

    let diff = row.stack_len - F::TWO;
    if let Some(inv) = diff.try_inverse() {
        row.general.stack_mut().stack_inv = inv;
        row.general.stack_mut().stack_inv_aux = F::ONE;
    } else {
        row.general.stack_mut().stack_inv = F::ZERO;
        row.general.stack_mut().stack_inv_aux = F::ZERO;
    }

    state.traces.push_memory(log_cond);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_pc<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    push_with_write(state, &mut row, state.registers.program_counter.into())?;
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
    push_with_write(state, &mut row, state.registers.context.into())?;
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_set_context<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(ctx, _)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;

    let sp_to_save = state.registers.stack_len.into();

    let old_ctx = state.registers.context;
    let new_ctx = u256_to_usize(ctx)?;

    let sp_field = ContextMetadata::StackSize as usize;
    let old_sp_addr = MemoryAddress::new(old_ctx, Segment::ContextMetadata, sp_field);
    let new_sp_addr = MemoryAddress::new(new_ctx, Segment::ContextMetadata, sp_field);

    let log_write_old_sp = mem_write_gp_log_and_fill(1, old_sp_addr, state, &mut row, sp_to_save);
    let (new_sp, log_read_new_sp) = if old_ctx == new_ctx {
        let op = MemoryOp::new(
            MemoryChannel::GeneralPurpose(2),
            state.traces.clock(),
            new_sp_addr,
            MemoryOpKind::Read,
            sp_to_save,
        );

        let channel = &mut row.mem_channels[2];
        assert_eq!(channel.used, F::ZERO);
        channel.used = F::ONE;
        channel.is_read = F::ONE;
        channel.addr_context = F::from_canonical_usize(new_ctx);
        channel.addr_segment = F::from_canonical_usize(Segment::ContextMetadata as usize);
        channel.addr_virtual = F::from_canonical_usize(new_sp_addr.virt);
        let val_limbs: [u64; 4] = sp_to_save.0;
        for (i, limb) in val_limbs.into_iter().enumerate() {
            channel.value[2 * i] = F::from_canonical_u32(limb as u32);
            channel.value[2 * i + 1] = F::from_canonical_u32((limb >> 32) as u32);
        }

        (sp_to_save, op)
    } else {
        mem_read_gp_with_log_and_fill(2, new_sp_addr, state, &mut row)
    };

    // If the new stack isn't empty, read stack_top from memory.
    let new_sp = new_sp.as_usize();
    if new_sp > 0 {
        // Set up columns to disable the channel if it *is* empty.
        let new_sp_field = F::from_canonical_usize(new_sp);
        if let Some(inv) = new_sp_field.try_inverse() {
            row.general.stack_mut().stack_inv = inv;
            row.general.stack_mut().stack_inv_aux = F::ONE;
        } else {
            row.general.stack_mut().stack_inv = F::ZERO;
            row.general.stack_mut().stack_inv_aux = F::ZERO;
        }

        let new_top_addr = MemoryAddress::new(new_ctx, Segment::Stack, new_sp - 1);
        let (new_top, log_read_new_top) =
            mem_read_gp_with_log_and_fill(3, new_top_addr, state, &mut row);
        state.registers.stack_top = new_top;
        state.traces.push_memory(log_read_new_top);
    } else {
        row.general.stack_mut().stack_inv = F::ZERO;
        row.general.stack_mut().stack_inv_aux = F::ZERO;
    }

    state.registers.context = new_ctx;
    state.registers.stack_len = new_sp;
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
    let num_bytes = n as usize;
    if num_bytes > 32 {
        // The call to `U256::from_big_endian()` would panic.
        return Err(ProgramError::IntegerTooLarge);
    }
    let initial_offset = state.registers.program_counter + 1;

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
                .low_u32() as u8
        })
        .collect_vec();

    let val = U256::from_big_endian(&bytes);
    push_with_write(state, &mut row, val)?;
    state.traces.push_cpu(row);

    Ok(())
}

// This instruction is special. The order of the operations are:
// - Write `stack_top` at `stack[stack_len - 1]`
// - Read `val` at `stack[stack_len - 1 - n]`
// - Update `stack_top` with `val` and add 1 to `stack_len`
// Since the write must happen before the read, the normal way of assigning
// GP channels doesn't work and we must handle them manually.
pub(crate) fn generate_dup<F: Field>(
    n: u8,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    // Same logic as in `push_with_write`, but we use the channel GP(0) instead.
    if !state.registers.is_kernel && state.registers.stack_len >= MAX_USER_STACK_SIZE {
        return Err(ProgramError::StackOverflow);
    }
    if n as usize >= state.registers.stack_len {
        return Err(ProgramError::StackUnderflow);
    }
    let stack_top = state.registers.stack_top;
    let address = MemoryAddress::new(
        state.registers.context,
        Segment::Stack,
        state.registers.stack_len - 1,
    );
    let log_push = mem_write_gp_log_and_fill(1, address, state, &mut row, stack_top);
    state.traces.push_memory(log_push);

    let other_addr = MemoryAddress::new(
        state.registers.context,
        Segment::Stack,
        state.registers.stack_len - 1 - n as usize,
    );

    // If n = 0, we read a value that hasn't been written to memory: the corresponding write
    // is buffered in the mem_ops queue, but hasn't been applied yet.
    let (val, log_read) = if n == 0 {
        let op = MemoryOp::new(
            MemoryChannel::GeneralPurpose(2),
            state.traces.clock(),
            other_addr,
            MemoryOpKind::Read,
            stack_top,
        );

        let channel = &mut row.mem_channels[2];
        assert_eq!(channel.used, F::ZERO);
        channel.used = F::ONE;
        channel.is_read = F::ONE;
        channel.addr_context = F::from_canonical_usize(other_addr.context);
        channel.addr_segment = F::from_canonical_usize(other_addr.segment);
        channel.addr_virtual = F::from_canonical_usize(other_addr.virt);
        let val_limbs: [u64; 4] = state.registers.stack_top.0;
        for (i, limb) in val_limbs.into_iter().enumerate() {
            channel.value[2 * i] = F::from_canonical_u32(limb as u32);
            channel.value[2 * i + 1] = F::from_canonical_u32((limb >> 32) as u32);
        }

        (stack_top, op)
    } else {
        mem_read_gp_with_log_and_fill(2, other_addr, state, &mut row)
    };
    push_no_write(state, &mut row, val, None);

    state.traces.push_memory(log_read);
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

    let [(in0, _)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let (in1, log_in1) = mem_read_gp_with_log_and_fill(1, other_addr, state, &mut row);
    let log_out0 = mem_write_gp_log_and_fill(2, other_addr, state, &mut row, in0);
    push_no_write(state, &mut row, in1, None);

    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_out0);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_not<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(x, _)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let result = !x;
    push_no_write(state, &mut row, result, Some(NUM_GP_CHANNELS - 1));

    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_iszero<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(x, _)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let is_zero = x.is_zero();
    let result = {
        let t: u64 = is_zero.into();
        t.into()
    };

    generate_pinv_diff(x, U256::zero(), &mut row);

    push_no_write(state, &mut row, result, None);
    state.traces.push_cpu(row);
    Ok(())
}

fn append_shift<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
    is_shl: bool,
    input0: U256,
    input1: U256,
    log_in1: MemoryOp,
    result: U256,
) -> Result<(), ProgramError> {
    const LOOKUP_CHANNEL: usize = 2;
    let lookup_addr = MemoryAddress::new(0, Segment::ShiftTable, input0.low_u32() as usize);
    if input0.bits() <= 32 {
        let (_, read) = mem_read_gp_with_log_and_fill(LOOKUP_CHANNEL, lookup_addr, state, &mut row);
        state.traces.push_memory(read);
    } else {
        // The shift constraints still expect the address to be set, even though no read will occur.
        let channel = &mut row.mem_channels[LOOKUP_CHANNEL];
        channel.addr_context = F::from_canonical_usize(lookup_addr.context);
        channel.addr_segment = F::from_canonical_usize(lookup_addr.segment);
        channel.addr_virtual = F::from_canonical_usize(lookup_addr.virt);

        // Extra field required by the constraints for large shifts.
        let high_limb_sum = row.mem_channels[0].value[1..].iter().copied().sum::<F>();
        row.general.shift_mut().high_limb_sum_inv = high_limb_sum.inverse();
    }

    let operator = if is_shl {
        BinaryOperator::Shl
    } else {
        BinaryOperator::Shr
    };
    let operation = arithmetic::Operation::binary(operator, input0, input1);

    state.traces.push_arithmetic(operation);
    push_no_write(state, &mut row, result, Some(NUM_GP_CHANNELS - 1));
    state.traces.push_memory(log_in1);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_shl<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(input0, _), (input1, log_in1)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;

    let result = if input0 > U256::from(255u64) {
        U256::zero()
    } else {
        input1 << input0
    };
    append_shift(state, row, true, input0, input1, log_in1, result)
}

pub(crate) fn generate_shr<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(input0, _), (input1, log_in1)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;

    let result = if input0 > U256::from(255u64) {
        U256::zero()
    } else {
        input1 >> input0
    };
    append_shift(state, row, false, input0, input1, log_in1, result)
}

pub(crate) fn generate_syscall<F: Field>(
    opcode: u8,
    stack_values_read: usize,
    stack_len_increased: bool,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    if TryInto::<u64>::try_into(state.registers.gas_used).is_err() {
        return Err(ProgramError::GasLimitError);
    }

    if state.registers.stack_len < stack_values_read {
        return Err(ProgramError::StackUnderflow);
    }
    if stack_len_increased
        && !state.registers.is_kernel
        && state.registers.stack_len >= MAX_USER_STACK_SIZE
    {
        return Err(ProgramError::StackOverflow);
    }

    let handler_jumptable_addr = KERNEL.global_labels["syscall_jumptable"];
    let handler_addr_addr =
        handler_jumptable_addr + (opcode as usize) * (BYTES_PER_OFFSET as usize);
    assert_eq!(BYTES_PER_OFFSET, 3, "Code below assumes 3 bytes per offset");
    let (handler_addr0, log_in0) = mem_read_gp_with_log_and_fill(
        1,
        MemoryAddress::new(0, Segment::Code, handler_addr_addr),
        state,
        &mut row,
    );
    let (handler_addr1, log_in1) = mem_read_gp_with_log_and_fill(
        2,
        MemoryAddress::new(0, Segment::Code, handler_addr_addr + 1),
        state,
        &mut row,
    );
    let (handler_addr2, log_in2) = mem_read_gp_with_log_and_fill(
        3,
        MemoryAddress::new(0, Segment::Code, handler_addr_addr + 2),
        state,
        &mut row,
    );

    let handler_addr = (handler_addr0 << 16) + (handler_addr1 << 8) + handler_addr2;
    let new_program_counter = u256_to_usize(handler_addr)?;

    let syscall_info = U256::from(state.registers.program_counter + 1)
        + (U256::from(u64::from(state.registers.is_kernel)) << 32)
        + (U256::from(state.registers.gas_used) << 192);

    // Set registers before pushing to the stack; in particular, we need to set kernel mode so we
    // can't incorrectly trigger a stack overflow. However, note that we have to do it _after_ we
    // make `syscall_info`, which should contain the old values.
    state.registers.program_counter = new_program_counter;
    state.registers.is_kernel = true;
    state.registers.gas_used = 0;

    push_with_write(state, &mut row, syscall_info)?;

    log::debug!("Syscall to {}", KERNEL.offset_name(new_program_counter));

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_cpu(row);

    Ok(())
}

pub(crate) fn generate_eq<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(in0, _), (in1, log_in1)] = stack_pop_with_log_and_fill::<2, _>(state, &mut row)?;
    let eq = in0 == in1;
    let result = U256::from(u64::from(eq));

    generate_pinv_diff(in0, in1, &mut row);

    push_no_write(state, &mut row, result, None);
    state.traces.push_memory(log_in1);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_exit_kernel<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(kexit_info, _)] = stack_pop_with_log_and_fill::<1, _>(state, &mut row)?;
    let kexit_info_u64 = kexit_info.0[0];
    let program_counter = kexit_info_u64 as u32 as usize;
    let is_kernel_mode_val = (kexit_info_u64 >> 32) as u32;
    assert!(is_kernel_mode_val == 0 || is_kernel_mode_val == 1);
    let is_kernel_mode = is_kernel_mode_val != 0;
    let gas_used_val = kexit_info.0[3];
    if TryInto::<u64>::try_into(gas_used_val).is_err() {
        return Err(ProgramError::GasLimitError);
    }

    state.registers.program_counter = program_counter;
    state.registers.is_kernel = is_kernel_mode;
    state.registers.gas_used = gas_used_val;
    log::debug!(
        "Exiting to {}, is_kernel={}",
        program_counter,
        is_kernel_mode
    );

    state.traces.push_cpu(row);

    Ok(())
}

pub(crate) fn generate_mload_general<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(context, _), (segment, log_in1), (virt, log_in2)] =
        stack_pop_with_log_and_fill::<3, _>(state, &mut row)?;

    let (val, log_read) = mem_read_gp_with_log_and_fill(
        3,
        MemoryAddress::new_u256s(context, segment, virt)?,
        state,
        &mut row,
    );
    push_no_write(state, &mut row, val, None);

    let diff = row.stack_len - F::from_canonical_usize(4);
    if let Some(inv) = diff.try_inverse() {
        row.general.stack_mut().stack_inv = inv;
        row.general.stack_mut().stack_inv_aux = F::ONE;
    } else {
        row.general.stack_mut().stack_inv = F::ZERO;
        row.general.stack_mut().stack_inv_aux = F::ZERO;
    }

    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_read);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_mload_32bytes<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(context, _), (segment, log_in1), (base_virt, log_in2), (len, log_in3)] =
        stack_pop_with_log_and_fill::<4, _>(state, &mut row)?;
    let len = u256_to_usize(len)?;
    if len > 32 {
        // The call to `U256::from_big_endian()` would panic.
        return Err(ProgramError::IntegerTooLarge);
    }

    let base_address = MemoryAddress::new_u256s(context, segment, base_virt)?;
    if usize::MAX - base_address.virt < len {
        return Err(ProgramError::MemoryError(VirtTooLarge {
            virt: base_address.virt.into(),
        }));
    }
    let bytes = (0..len)
        .map(|i| {
            let address = MemoryAddress {
                virt: base_address.virt + i,
                ..base_address
            };
            let val = state.memory.get(address);
            val.low_u32() as u8
        })
        .collect_vec();

    let packed_int = U256::from_big_endian(&bytes);
    push_no_write(state, &mut row, packed_int, Some(4));

    byte_packing_log(state, base_address, bytes);

    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_in3);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_mstore_general<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(context, _), (segment, log_in1), (virt, log_in2), (val, log_in3)] =
        stack_pop_with_log_and_fill::<4, _>(state, &mut row)?;

    let address = MemoryAddress {
        context: context
            .try_into()
            .map_err(|_| MemoryError(ContextTooLarge { context }))?,
        segment: segment
            .try_into()
            .map_err(|_| MemoryError(SegmentTooLarge { segment }))?,
        virt: virt
            .try_into()
            .map_err(|_| MemoryError(VirtTooLarge { virt }))?,
    };
    let log_write = mem_write_gp_log_and_fill(4, address, state, &mut row, val);

    let diff = row.stack_len - F::from_canonical_usize(4);
    if let Some(inv) = diff.try_inverse() {
        row.general.stack_mut().stack_inv = inv;
        row.general.stack_mut().stack_inv_aux = F::ONE;
        row.general.stack_mut().stack_inv_aux_2 = F::ONE;
        state.registers.is_stack_top_read = true;
    } else {
        row.general.stack_mut().stack_inv = F::ZERO;
        row.general.stack_mut().stack_inv_aux = F::ZERO;
    }

    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_in3);
    state.traces.push_memory(log_write);
    state.traces.push_cpu(row);

    Ok(())
}

pub(crate) fn generate_mstore_32bytes<F: Field>(
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    let [(context, _), (segment, log_in1), (base_virt, log_in2), (val, log_in3), (len, log_in4)] =
        stack_pop_with_log_and_fill::<5, _>(state, &mut row)?;
    let len = u256_to_usize(len)?;

    let base_address = MemoryAddress::new_u256s(context, segment, base_virt)?;

    byte_unpacking_log(state, base_address, val, len);

    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_memory(log_in3);
    state.traces.push_memory(log_in4);
    state.traces.push_cpu(row);
    Ok(())
}

pub(crate) fn generate_exception<F: Field>(
    exc_code: u8,
    state: &mut GenerationState<F>,
    mut row: CpuColumnsView<F>,
) -> Result<(), ProgramError> {
    if TryInto::<u64>::try_into(state.registers.gas_used).is_err() {
        return Err(ProgramError::GasLimitError);
    }

    row.op.exception = F::ONE;

    let disallowed_len = F::from_canonical_usize(MAX_USER_STACK_SIZE + 1);
    let diff = row.stack_len - disallowed_len;
    if let Some(inv) = diff.try_inverse() {
        row.stack_len_bounds_aux = inv;
    } else {
        // This is a stack overflow that should have been caught earlier.
        return Err(ProgramError::InterpreterError);
    }

    if let Some(inv) = row.stack_len.try_inverse() {
        row.general.stack_mut().stack_inv = inv;
        row.general.stack_mut().stack_inv_aux = F::ONE;
    }

    if state.registers.is_stack_top_read {
        let channel = &mut row.mem_channels[0];
        channel.used = F::ONE;
        channel.is_read = F::ONE;
        channel.addr_context = F::from_canonical_usize(state.registers.context);
        channel.addr_segment = F::from_canonical_usize(Segment::Stack as usize);
        channel.addr_virtual = F::from_canonical_usize(state.registers.stack_len - 1);

        let address = MemoryAddress {
            context: state.registers.context,
            segment: Segment::Stack as usize,
            virt: state.registers.stack_len - 1,
        };

        let mem_op = MemoryOp::new(
            GeneralPurpose(0),
            state.traces.clock(),
            address,
            MemoryOpKind::Read,
            state.registers.stack_top,
        );
        state.traces.push_memory(mem_op);
        state.registers.is_stack_top_read = false;
    }

    row.general.exception_mut().exc_code_bits = [
        F::from_bool(exc_code & 1 != 0),
        F::from_bool(exc_code & 2 != 0),
        F::from_bool(exc_code & 4 != 0),
    ];

    let handler_jumptable_addr = KERNEL.global_labels["exception_jumptable"];
    let handler_addr_addr =
        handler_jumptable_addr + (exc_code as usize) * (BYTES_PER_OFFSET as usize);
    assert_eq!(BYTES_PER_OFFSET, 3, "Code below assumes 3 bytes per offset");
    let (handler_addr0, log_in0) = mem_read_gp_with_log_and_fill(
        1,
        MemoryAddress::new(0, Segment::Code, handler_addr_addr),
        state,
        &mut row,
    );
    let (handler_addr1, log_in1) = mem_read_gp_with_log_and_fill(
        2,
        MemoryAddress::new(0, Segment::Code, handler_addr_addr + 1),
        state,
        &mut row,
    );
    let (handler_addr2, log_in2) = mem_read_gp_with_log_and_fill(
        3,
        MemoryAddress::new(0, Segment::Code, handler_addr_addr + 2),
        state,
        &mut row,
    );

    let handler_addr = (handler_addr0 << 16) + (handler_addr1 << 8) + handler_addr2;
    let new_program_counter = u256_to_usize(handler_addr)?;

    let exc_info =
        U256::from(state.registers.program_counter) + (U256::from(state.registers.gas_used) << 192);

    // Set registers before pushing to the stack; in particular, we need to set kernel mode so we
    // can't incorrectly trigger a stack overflow. However, note that we have to do it _after_ we
    // make `exc_info`, which should contain the old values.
    state.registers.program_counter = new_program_counter;
    state.registers.is_kernel = true;
    state.registers.gas_used = 0;

    push_with_write(state, &mut row, exc_info)?;

    log::debug!("Exception to {}", KERNEL.offset_name(new_program_counter));

    state.traces.push_memory(log_in0);
    state.traces.push_memory(log_in1);
    state.traces.push_memory(log_in2);
    state.traces.push_cpu(row);

    Ok(())
}
