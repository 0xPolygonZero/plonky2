use std::mem;

use ethereum_types::U256;
use plonky2::field::types::Field;
use tiny_keccak::keccakf;

use crate::cpu::columns::{CpuColumnsView, NUM_CPU_COLUMNS};
use crate::generation::memory::MemoryState;
use crate::generation::mpt::all_mpt_prover_inputs_reversed;
use crate::generation::rlp::all_rlp_prover_inputs_reversed;
use crate::generation::GenerationInputs;
use crate::keccak_memory::keccak_memory_stark::KeccakMemoryOp;
use crate::memory::memory_stark::MemoryOp;
use crate::memory::segments::Segment;
use crate::memory::NUM_CHANNELS;
use crate::util::u256_limbs;
use crate::{keccak, logic};

#[derive(Debug)]
pub(crate) struct GenerationState<F: Field> {
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) inputs: GenerationInputs,
    pub(crate) next_txn_index: usize,
    pub(crate) cpu_rows: Vec<[F; NUM_CPU_COLUMNS]>,
    pub(crate) current_cpu_row: CpuColumnsView<F>,

    pub(crate) current_context: usize,
    pub(crate) memory: MemoryState,

    pub(crate) keccak_inputs: Vec<[u64; keccak::keccak_stark::NUM_INPUTS]>,
    pub(crate) keccak_memory_inputs: Vec<KeccakMemoryOp>,
    pub(crate) logic_ops: Vec<logic::Operation>,

    /// Prover inputs containing MPT data, in reverse order so that the next input can be obtained
    /// via `pop()`.
    pub(crate) mpt_prover_inputs: Vec<U256>,

    /// Prover inputs containing RLP data, in reverse order so that the next input can be obtained
    /// via `pop()`.
    pub(crate) rlp_prover_inputs: Vec<U256>,
}

impl<F: Field> GenerationState<F> {
    pub(crate) fn new(inputs: GenerationInputs) -> Self {
        let mpt_prover_inputs = all_mpt_prover_inputs_reversed(&inputs.tries);
        let rlp_prover_inputs = all_rlp_prover_inputs_reversed(&inputs.signed_txns);

        Self {
            inputs,
            next_txn_index: 0,
            cpu_rows: vec![],
            current_cpu_row: [F::ZERO; NUM_CPU_COLUMNS].into(),
            current_context: 0,
            memory: MemoryState::default(),
            keccak_inputs: vec![],
            keccak_memory_inputs: vec![],
            logic_ops: vec![],
            mpt_prover_inputs,
            rlp_prover_inputs,
        }
    }

    /// Compute logical AND, and record the operation to be added in the logic table later.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn and(&mut self, input0: U256, input1: U256) -> U256 {
        self.logic_op(logic::Op::And, input0, input1)
    }

    /// Compute logical OR, and record the operation to be added in the logic table later.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn or(&mut self, input0: U256, input1: U256) -> U256 {
        self.logic_op(logic::Op::Or, input0, input1)
    }

    /// Compute logical XOR, and record the operation to be added in the logic table later.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn xor(&mut self, input0: U256, input1: U256) -> U256 {
        self.logic_op(logic::Op::Xor, input0, input1)
    }

    /// Compute logical AND, and record the operation to be added in the logic table later.
    pub(crate) fn logic_op(&mut self, op: logic::Op, input0: U256, input1: U256) -> U256 {
        let operation = logic::Operation::new(op, input0, input1);
        let result = operation.result;
        self.logic_ops.push(operation);
        result
    }

    /// Like `get_mem_cpu`, but reads from the current context specifically.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn get_mem_cpu_current(
        &mut self,
        channel_index: usize,
        segment: Segment,
        virt: usize,
    ) -> U256 {
        let context = self.current_context;
        self.get_mem_cpu(channel_index, context, segment, virt)
    }

    /// Simulates the CPU reading some memory through the given channel. Besides logging the memory
    /// operation, this also generates the associated registers in the current CPU row.
    pub(crate) fn get_mem_cpu(
        &mut self,
        channel_index: usize,
        context: usize,
        segment: Segment,
        virt: usize,
    ) -> U256 {
        let timestamp = self.cpu_rows.len() * NUM_CHANNELS + channel_index;
        let value = self.get_mem(context, segment, virt, timestamp);

        let channel = &mut self.current_cpu_row.mem_channels[channel_index];
        channel.used = F::ONE;
        channel.is_read = F::ONE;
        channel.addr_context = F::from_canonical_usize(context);
        channel.addr_segment = F::from_canonical_usize(segment as usize);
        channel.addr_virtual = F::from_canonical_usize(virt);
        channel.value = u256_limbs(value);

        value
    }

    /// Read some memory, and log the operation.
    pub(crate) fn get_mem(
        &mut self,
        context: usize,
        segment: Segment,
        virt: usize,
        timestamp: usize,
    ) -> U256 {
        let value = self.memory.contexts[context].segments[segment as usize].get(virt);
        self.memory.log.push(MemoryOp {
            filter: true,
            timestamp,
            is_read: true,
            context,
            segment,
            virt,
            value,
        });
        value
    }

    /// Write some memory within the current execution context, and log the operation.
    pub(crate) fn set_mem_cpu_current(
        &mut self,
        channel_index: usize,
        segment: Segment,
        virt: usize,
        value: U256,
    ) {
        let context = self.current_context;
        self.set_mem_cpu(channel_index, context, segment, virt, value);
    }

    /// Write some memory, and log the operation.
    pub(crate) fn set_mem_cpu(
        &mut self,
        channel_index: usize,
        context: usize,
        segment: Segment,
        virt: usize,
        value: U256,
    ) {
        let timestamp = self.cpu_rows.len() * NUM_CHANNELS + channel_index;
        self.set_mem(context, segment, virt, value, timestamp);

        let channel = &mut self.current_cpu_row.mem_channels[channel_index];
        channel.used = F::ONE;
        channel.is_read = F::ZERO; // For clarity; should already be 0.
        channel.addr_context = F::from_canonical_usize(context);
        channel.addr_segment = F::from_canonical_usize(segment as usize);
        channel.addr_virtual = F::from_canonical_usize(virt);
        channel.value = u256_limbs(value);
    }

    /// Write some memory, and log the operation.
    pub(crate) fn set_mem(
        &mut self,
        context: usize,
        segment: Segment,
        virt: usize,
        value: U256,
        timestamp: usize,
    ) {
        self.memory.log.push(MemoryOp {
            filter: true,
            timestamp,
            is_read: false,
            context,
            segment,
            virt,
            value,
        });
        self.memory.contexts[context].segments[segment as usize].set(virt, value)
    }

    /// Evaluate the Keccak-f permutation in-place on some data in memory, and record the operations
    /// for the purpose of witness generation.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn keccak_memory(
        &mut self,
        context: usize,
        segment: Segment,
        virt: usize,
    ) -> [u64; keccak::keccak_stark::NUM_INPUTS] {
        let read_timestamp = self.cpu_rows.len() * NUM_CHANNELS;
        let _write_timestamp = read_timestamp + 1;
        let input = (0..25)
            .map(|i| {
                let bytes = [0, 1, 2, 3, 4, 5, 6, 7].map(|j| {
                    let virt = virt + i * 8 + j;
                    let byte = self.get_mem(context, segment, virt, read_timestamp);
                    debug_assert!(byte.bits() <= 8);
                    byte.as_u32() as u8
                });
                u64::from_le_bytes(bytes)
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let output = self.keccak(input);
        self.keccak_memory_inputs.push(KeccakMemoryOp {
            context,
            segment,
            virt,
            read_timestamp,
            input,
            output,
        });
        // TODO: Write output to memory.
        output
    }

    /// Evaluate the Keccak-f permutation, and record the operation for the purpose of witness
    /// generation.
    pub(crate) fn keccak(
        &mut self,
        mut input: [u64; keccak::keccak_stark::NUM_INPUTS],
    ) -> [u64; keccak::keccak_stark::NUM_INPUTS] {
        self.keccak_inputs.push(input);
        keccakf(&mut input);
        input
    }

    pub(crate) fn commit_cpu_row(&mut self) {
        let mut swapped_row = [F::ZERO; NUM_CPU_COLUMNS].into();
        mem::swap(&mut self.current_cpu_row, &mut swapped_row);
        self.cpu_rows.push(swapped_row.into());
    }
}
