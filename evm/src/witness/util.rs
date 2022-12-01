use ethereum_types::U256;
use plonky2::field::types::Field;

use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::cpu::stack_bounds::MAX_USER_STACK_SIZE;
use crate::memory::segments::Segment;
use crate::witness::errors::ProgramError;
use crate::witness::memory::{MemoryAddress, MemoryChannel, MemoryOp, MemoryOpKind, MemoryState};
use crate::witness::state::RegistersState;
use crate::witness::traces::Traces;

fn to_byte_checked(n: U256) -> u8 {
    let res = n.byte(0);
    assert_eq!(n, res.into());
    res
}

fn to_bits_le<F: Field>(n: u8) -> [F; 8] {
    let mut res = [F::ZERO; 8];
    for (i, bit) in res.iter_mut().enumerate() {
        *bit = F::from_bool(n & (1 << i) != 0);
    }
    res
}

pub(crate) fn mem_read_with_log<T: Copy>(
    channel: MemoryChannel,
    address: MemoryAddress,
    memory_state: &MemoryState,
    traces: &Traces<T>,
) -> (U256, MemoryOp) {
    let val = memory_state.get(address);
    let op = MemoryOp::new(channel, traces.clock(), address, MemoryOpKind::Read, val);
    (val, op)
}

pub(crate) fn mem_write_log<T: Copy>(
    channel: MemoryChannel,
    address: MemoryAddress,
    traces: &Traces<T>,
    val: U256,
) -> MemoryOp {
    MemoryOp::new(channel, traces.clock(), address, MemoryOpKind::Write, val)
}

pub(crate) fn mem_read_code_with_log_and_fill<F: Field>(
    address: MemoryAddress,
    memory_state: &MemoryState,
    traces: &Traces<F>,
    row: &mut CpuColumnsView<F>,
) -> (u8, MemoryOp) {
    let (val, op) = mem_read_with_log(MemoryChannel::Code, address, memory_state, traces);

    let val_u8 = to_byte_checked(val);
    row.opcode_bits = to_bits_le(val_u8);

    (val_u8, op)
}

pub(crate) fn mem_read_gp_with_log_and_fill<F: Field>(
    n: usize,
    address: MemoryAddress,
    memory_state: &MemoryState,
    traces: &Traces<F>,
    row: &mut CpuColumnsView<F>,
) -> (U256, MemoryOp) {
    let (val, op) = mem_read_with_log(
        MemoryChannel::GeneralPurpose(n),
        address,
        memory_state,
        traces,
    );
    let val_limbs: [u64; 4] = val.0;

    let channel = &mut row.mem_channels[n];
    channel.used = F::ONE;
    channel.is_read = F::ONE;
    channel.addr_context = F::from_canonical_usize(address.context);
    channel.addr_segment = F::from_canonical_usize(address.segment);
    channel.addr_virtual = F::from_canonical_usize(address.virt);
    for (i, limb) in val_limbs.into_iter().enumerate() {
        channel.value[2 * i] = F::from_canonical_u32(limb as u32);
        channel.value[2 * i + 1] = F::from_canonical_u32((limb >> 32) as u32);
    }

    (val, op)
}

pub(crate) fn mem_write_gp_log_and_fill<F: Field>(
    n: usize,
    address: MemoryAddress,
    traces: &Traces<F>,
    row: &mut CpuColumnsView<F>,
    val: U256,
) -> MemoryOp {
    let op = mem_write_log(MemoryChannel::GeneralPurpose(n), address, traces, val);
    let val_limbs: [u64; 4] = val.0;

    let channel = &mut row.mem_channels[n];
    channel.used = F::ONE;
    channel.is_read = F::ZERO;
    channel.addr_context = F::from_canonical_usize(address.context);
    channel.addr_segment = F::from_canonical_usize(address.segment);
    channel.addr_virtual = F::from_canonical_usize(address.virt);
    for (i, limb) in val_limbs.into_iter().enumerate() {
        channel.value[2 * i] = F::from_canonical_u32(limb as u32);
        channel.value[2 * i + 1] = F::from_canonical_u32((limb >> 32) as u32);
    }

    op
}

pub(crate) fn stack_pop_with_log_and_fill<const N: usize, F: Field>(
    registers_state: &mut RegistersState,
    memory_state: &MemoryState,
    traces: &Traces<F>,
    row: &mut CpuColumnsView<F>,
) -> Result<[(U256, MemoryOp); N], ProgramError> {
    if (registers_state.stack_len as usize) < N {
        return Err(ProgramError::StackUnderflow);
    }

    let result = {
        let mut i = 0usize;
        [(); N].map(|_| {
            let address = MemoryAddress::new(
                registers_state.context,
                Segment::Stack,
                registers_state.stack_len - 1 - i,
            );
            let res = mem_read_gp_with_log_and_fill(i, address, memory_state, traces, row);
            i += 1;
            res
        })
    };

    registers_state.stack_len -= N;

    Ok(result)
}

pub(crate) fn stack_push_log_and_fill<F: Field>(
    registers_state: &mut RegistersState,
    traces: &Traces<F>,
    row: &mut CpuColumnsView<F>,
    val: U256,
) -> Result<MemoryOp, ProgramError> {
    if !registers_state.is_kernel && registers_state.stack_len >= MAX_USER_STACK_SIZE {
        return Err(ProgramError::StackOverflow);
    }

    let address = MemoryAddress::new(
        registers_state.context,
        Segment::Stack,
        registers_state.stack_len,
    );
    let res = mem_write_gp_log_and_fill(NUM_GP_CHANNELS - 1, address, traces, row, val);

    registers_state.stack_len += 1;

    Ok(res)
}
