use std::mem;

use ethereum_types::U256;
use plonky2::field::types::Field;

use crate::cpu::columns::{CpuColumnsView, NUM_CPU_COLUMNS};
use crate::generation::memory::MemoryState;
use crate::memory::memory_stark::MemoryOp;
use crate::memory::segments::Segment;
use crate::{keccak, logic};

#[derive(Debug)]
pub(crate) struct GenerationState<F: Field> {
    pub(crate) cpu_rows: Vec<[F; NUM_CPU_COLUMNS]>,
    pub(crate) current_cpu_row: CpuColumnsView<F>,

    pub(crate) current_context: usize,
    pub(crate) memory: MemoryState,

    pub(crate) keccak_inputs: Vec<[u64; keccak::keccak_stark::NUM_INPUTS]>,
    pub(crate) logic_ops: Vec<logic::Operation>,

    /// Non-deterministic inputs provided by the prover.
    pub(crate) prover_inputs: Vec<U256>,
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
        let timestamp = self.cpu_rows.len();
        let context = self.current_context;
        let value = self.memory.contexts[context].segments[segment as usize].get(virt);
        self.memory.log.push(MemoryOp {
            channel_index: Some(channel_index),
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
        let timestamp = self.cpu_rows.len();
        let context = self.current_context;
        self.memory.log.push(MemoryOp {
            channel_index: Some(channel_index),
            timestamp,
            is_read: false,
            context,
            segment,
            virt,
            value,
        });
        self.memory.contexts[context].segments[segment as usize].set(virt, value)
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
            logic_ops: vec![],
            prover_inputs: vec![],
        }
    }
}
