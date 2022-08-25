use std::mem;

use ethereum_types::U256;
use plonky2::field::types::Field;
use tiny_keccak::keccakf;

use crate::cpu::columns::{CpuColumnsView, NUM_CPU_COLUMNS};
use crate::generation::memory::MemoryState;
use crate::keccak_memory::keccak_memory_stark::KeccakMemoryOp;
use crate::memory::memory_stark::MemoryOp;
use crate::memory::segments::Segment;
use crate::memory::NUM_CHANNELS;
use crate::{keccak, logic};

#[derive(Debug)]
pub(crate) struct GenerationState<F: Field> {
    pub(crate) cpu_rows: Vec<[F; NUM_CPU_COLUMNS]>,
    pub(crate) current_cpu_row: CpuColumnsView<F>,

    pub(crate) current_context: usize,
    pub(crate) memory: MemoryState,

    pub(crate) keccak_inputs: Vec<[u64; keccak::keccak_stark::NUM_INPUTS]>,
    pub(crate) keccak_memory_inputs: Vec<KeccakMemoryOp>,
    pub(crate) logic_ops: Vec<logic::Operation>,
}

impl<F: Field> GenerationState<F> {
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

    /// Read some memory within the current execution context, and log the operation.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn get_mem_current(
        &mut self,
        channel_index: usize,
        segment: Segment,
        virt: usize,
    ) -> U256 {
        let context = self.current_context;
        self.get_mem(channel_index, context, segment, virt)
    }

    /// Read some memory, and log the operation.
    pub(crate) fn get_mem(
        &mut self,
        channel_index: usize,
        context: usize,
        segment: Segment,
        virt: usize,
    ) -> U256 {
        self.current_cpu_row.mem_channel_used[channel_index] = F::ONE;
        let timestamp = self.cpu_rows.len();
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
    pub(crate) fn set_mem_current(
        &mut self,
        channel_index: usize,
        segment: Segment,
        virt: usize,
        value: U256,
    ) {
        let context = self.current_context;
        self.set_mem(channel_index, context, segment, virt, value);
    }

    /// Write some memory, and log the operation.
    pub(crate) fn set_mem(
        &mut self,
        channel_index: usize,
        context: usize,
        segment: Segment,
        virt: usize,
        value: U256,
    ) {
        self.current_cpu_row.mem_channel_used[channel_index] = F::ONE;
        let timestamp = self.cpu_rows.len();
        let timestamp = timestamp * NUM_CHANNELS + channel_index;
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
        let input = (0..25)
            .map(|i| {
                let bytes = [0, 1, 2, 3, 4, 5, 6, 7].map(|j| {
                    let virt = virt + i * 8 + j;
                    let byte = self.get_mem(0, context, segment, virt);
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

// `GenerationState` can't `derive(Default)` because `Default` is only implemented for arrays up to
// length 32 :-\.
impl<F: Field> Default for GenerationState<F> {
    fn default() -> Self {
        Self {
            cpu_rows: vec![],
            current_cpu_row: [F::ZERO; NUM_CPU_COLUMNS].into(),
            current_context: 0,
            memory: MemoryState::default(),
            keccak_inputs: vec![],
            keccak_memory_inputs: vec![],
            logic_ops: vec![],
        }
    }
}
