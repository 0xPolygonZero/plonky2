use ethereum_types::U256;
use plonky2::field::types::Field;

use super::memory::DUMMY_MEMOP;
use crate::byte_packing::byte_packing_stark::BytePackingOp;
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::keccak_util::keccakf_u8s;
use crate::cpu::membus::NUM_CHANNELS;
use crate::cpu::stack::MAX_USER_STACK_SIZE;
use crate::generation::state::GenerationState;
use crate::keccak_sponge::columns::{KECCAK_RATE_BYTES, KECCAK_WIDTH_BYTES};
use crate::keccak_sponge::keccak_sponge_stark::KeccakSpongeOp;
use crate::logic;
use crate::memory::segments::Segment;
use crate::witness::errors::ProgramError;
use crate::witness::memory::{MemoryAddress, MemoryChannel, MemoryOp, MemoryOpKind};

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

/// Peek at the stack item `i`th from the top. If `i=0` this gives the tip.
pub(crate) fn stack_peek<F: Field>(
    state: &GenerationState<F>,
    i: usize,
) -> Result<U256, ProgramError> {
    if i >= state.registers.stack_len {
        return Err(ProgramError::StackUnderflow);
    }
    if i == 0 {
        return Ok(state.registers.stack_top);
    }

    Ok(state.memory.get(MemoryAddress::new(
        state.registers.context,
        Segment::Stack,
        state.registers.stack_len - 1 - i,
    )))
}

/// Peek at kernel at specified segment and address
pub(crate) fn current_context_peek<F: Field>(
    state: &GenerationState<F>,
    segment: Segment,
    virt: usize,
) -> U256 {
    let context = state.registers.context;
    state.memory.get(MemoryAddress::new(context, segment, virt))
}

pub(crate) fn fill_channel_with_value<F: Field>(row: &mut CpuColumnsView<F>, n: usize, val: U256) {
    let channel = &mut row.mem_channels[n];
    let val_limbs: [u64; 4] = val.0;
    for (i, limb) in val_limbs.into_iter().enumerate() {
        channel.value[2 * i] = F::from_canonical_u32(limb as u32);
        channel.value[2 * i + 1] = F::from_canonical_u32((limb >> 32) as u32);
    }
}

/// Pushes without writing in memory. This happens in opcodes where a push immediately follows a pop.
pub(crate) fn push_no_write<F: Field>(state: &mut GenerationState<F>, val: U256) {
    state.registers.stack_top = val;
    state.registers.stack_len += 1;
}

/// Pushes and (maybe) writes the previous stack top in memory. This happens in opcodes which only push.
pub(crate) fn push_with_write<F: Field>(
    state: &mut GenerationState<F>,
    row: &mut CpuColumnsView<F>,
    val: U256,
) -> Result<(), ProgramError> {
    if !state.registers.is_kernel && state.registers.stack_len >= MAX_USER_STACK_SIZE {
        return Err(ProgramError::StackOverflow);
    }

    let write = if state.registers.stack_len == 0 {
        None
    } else {
        let address = MemoryAddress::new(
            state.registers.context,
            Segment::Stack,
            state.registers.stack_len - 1,
        );
        let res = mem_write_partial_log_and_fill(address, state, row, state.registers.stack_top);
        Some(res)
    };
    push_no_write(state, val);
    if let Some(log) = write {
        state.traces.push_memory(log);
        row.partial_channel.used = F::ONE;
    }
    Ok(())
}

pub(crate) fn mem_read_with_log<F: Field>(
    channel: MemoryChannel,
    address: MemoryAddress,
    state: &GenerationState<F>,
) -> (U256, MemoryOp) {
    let val = state.memory.get(address);
    let op = MemoryOp::new(
        channel,
        state.traces.clock(),
        address,
        MemoryOpKind::Read,
        val,
    );
    (val, op)
}

pub(crate) fn mem_write_log<F: Field>(
    channel: MemoryChannel,
    address: MemoryAddress,
    state: &GenerationState<F>,
    val: U256,
) -> MemoryOp {
    MemoryOp::new(
        channel,
        state.traces.clock(),
        address,
        MemoryOpKind::Write,
        val,
    )
}

pub(crate) fn mem_read_code_with_log_and_fill<F: Field>(
    address: MemoryAddress,
    state: &GenerationState<F>,
    row: &mut CpuColumnsView<F>,
) -> (u8, MemoryOp) {
    let (val, op) = mem_read_with_log(MemoryChannel::Code, address, state);

    let val_u8 = to_byte_checked(val);
    row.opcode_bits = to_bits_le(val_u8);

    (val_u8, op)
}

pub(crate) fn mem_read_gp_with_log_and_fill<F: Field>(
    n: usize,
    address: MemoryAddress,
    state: &GenerationState<F>,
    row: &mut CpuColumnsView<F>,
) -> (U256, MemoryOp) {
    let (val, op) = mem_read_with_log(MemoryChannel::GeneralPurpose(n), address, state);
    let val_limbs: [u64; 4] = val.0;

    let channel = &mut row.mem_channels[n];
    assert_eq!(channel.used, F::ZERO);
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
    state: &GenerationState<F>,
    row: &mut CpuColumnsView<F>,
    val: U256,
) -> MemoryOp {
    let op = mem_write_log(MemoryChannel::GeneralPurpose(n), address, state, val);
    let val_limbs: [u64; 4] = val.0;

    let channel = &mut row.mem_channels[n];
    assert_eq!(channel.used, F::ZERO);
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

pub(crate) fn mem_write_partial_log_and_fill<F: Field>(
    address: MemoryAddress,
    state: &GenerationState<F>,
    row: &mut CpuColumnsView<F>,
    val: U256,
) -> MemoryOp {
    let op = mem_write_log(MemoryChannel::PartialChannel, address, state, val);

    let channel = &mut row.partial_channel;
    assert!(channel.used.is_zero());
    channel.used = F::ONE;
    channel.is_read = F::ZERO;
    channel.addr_context = F::from_canonical_usize(address.context);
    channel.addr_segment = F::from_canonical_usize(address.segment);
    channel.addr_virtual = F::from_canonical_usize(address.virt);

    op
}

// Channel 0 already contains the top of the stack. You only need to read
// from the second popped element.
// If the resulting stack isn't empty, update `stack_top`.
pub(crate) fn stack_pop_with_log_and_fill<const N: usize, F: Field>(
    state: &mut GenerationState<F>,
    row: &mut CpuColumnsView<F>,
) -> Result<[(U256, MemoryOp); N], ProgramError> {
    if state.registers.stack_len < N {
        return Err(ProgramError::StackUnderflow);
    }

    let new_stack_top = if state.registers.stack_len == N {
        None
    } else {
        Some(stack_peek(state, N)?)
    };

    let result = core::array::from_fn(|i| {
        if i == 0 {
            (state.registers.stack_top, DUMMY_MEMOP)
        } else {
            let address = MemoryAddress::new(
                state.registers.context,
                Segment::Stack,
                state.registers.stack_len - 1 - i,
            );

            mem_read_gp_with_log_and_fill(i, address, state, row)
        }
    });

    state.registers.stack_len -= N;

    if let Some(val) = new_stack_top {
        state.registers.stack_top = val;
    }

    Ok(result)
}

fn xor_into_sponge<F: Field>(
    state: &mut GenerationState<F>,
    sponge_state: &mut [u8; KECCAK_WIDTH_BYTES],
    block: &[u8; KECCAK_RATE_BYTES],
) {
    for i in (0..KECCAK_RATE_BYTES).step_by(32) {
        let range = i..KECCAK_RATE_BYTES.min(i + 32);
        let lhs = U256::from_little_endian(&sponge_state[range.clone()]);
        let rhs = U256::from_little_endian(&block[range]);
        state
            .traces
            .push_logic(logic::Operation::new(logic::Op::Xor, lhs, rhs));
    }
    for i in 0..KECCAK_RATE_BYTES {
        sponge_state[i] ^= block[i];
    }
}

pub(crate) fn keccak_sponge_log<F: Field>(
    state: &mut GenerationState<F>,
    base_address: MemoryAddress,
    input: Vec<u8>,
) {
    let clock = state.traces.clock();

    let mut address = base_address;
    let mut input_blocks = input.chunks_exact(KECCAK_RATE_BYTES);
    let mut sponge_state = [0u8; KECCAK_WIDTH_BYTES];
    for block in input_blocks.by_ref() {
        for &byte in block {
            state.traces.push_memory(MemoryOp::new(
                MemoryChannel::Code,
                clock,
                address,
                MemoryOpKind::Read,
                byte.into(),
            ));
            address.increment();
        }
        xor_into_sponge(state, &mut sponge_state, block.try_into().unwrap());
        state
            .traces
            .push_keccak_bytes(sponge_state, clock * NUM_CHANNELS);
        keccakf_u8s(&mut sponge_state);
    }

    for &byte in input_blocks.remainder() {
        state.traces.push_memory(MemoryOp::new(
            MemoryChannel::Code,
            clock,
            address,
            MemoryOpKind::Read,
            byte.into(),
        ));
        address.increment();
    }
    let mut final_block = [0u8; KECCAK_RATE_BYTES];
    final_block[..input_blocks.remainder().len()].copy_from_slice(input_blocks.remainder());
    // pad10*1 rule
    if input_blocks.remainder().len() == KECCAK_RATE_BYTES - 1 {
        // Both 1s are placed in the same byte.
        final_block[input_blocks.remainder().len()] = 0b10000001;
    } else {
        final_block[input_blocks.remainder().len()] = 1;
        final_block[KECCAK_RATE_BYTES - 1] = 0b10000000;
    }
    xor_into_sponge(state, &mut sponge_state, &final_block);
    state
        .traces
        .push_keccak_bytes(sponge_state, clock * NUM_CHANNELS);

    state.traces.push_keccak_sponge(KeccakSpongeOp {
        base_address,
        timestamp: clock * NUM_CHANNELS,
        input,
    });
}

pub(crate) fn byte_packing_log<F: Field>(
    state: &mut GenerationState<F>,
    base_address: MemoryAddress,
    bytes: Vec<u8>,
) {
    let clock = state.traces.clock();

    let mut address = base_address;
    for &byte in &bytes {
        state.traces.push_memory(MemoryOp::new(
            MemoryChannel::Code,
            clock,
            address,
            MemoryOpKind::Read,
            byte.into(),
        ));
        address.increment();
    }

    state.traces.push_byte_packing(BytePackingOp {
        is_read: true,
        base_address,
        timestamp: clock * NUM_CHANNELS,
        bytes,
    });
}

pub(crate) fn byte_unpacking_log<F: Field>(
    state: &mut GenerationState<F>,
    base_address: MemoryAddress,
    val: U256,
    len: usize,
) {
    let clock = state.traces.clock();

    let mut bytes = vec![0; 32];
    val.to_little_endian(&mut bytes);
    bytes.resize(len, 0);
    bytes.reverse();

    let mut address = base_address;
    for &byte in &bytes {
        state.traces.push_memory(MemoryOp::new(
            MemoryChannel::Code,
            clock,
            address,
            MemoryOpKind::Write,
            byte.into(),
        ));
        address.increment();
    }

    state.traces.push_byte_packing(BytePackingOp {
        is_read: false,
        base_address,
        timestamp: clock * NUM_CHANNELS,
        bytes,
    });
}
