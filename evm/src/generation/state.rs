use ethereum_types::U256;
use plonky2::field::types::Field;

use crate::cpu::columns::NUM_CPU_COLUMNS;
use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::assembler::Kernel;
use crate::generation::memory::MemoryState;
use crate::logic::{Op, Operation};
use crate::memory::memory_stark::MemoryOp;
use crate::{keccak, logic};

#[derive(Debug)]
pub(crate) struct GenerationState<F: Field> {
    pub(crate) kernel: Kernel,

    pub(crate) cpu_rows: Vec<[F; NUM_CPU_COLUMNS]>,
    pub(crate) current_cpu_row: [F; NUM_CPU_COLUMNS],

    pub(crate) current_context: usize,
    pub(crate) memory: MemoryState<F>,

    pub(crate) keccak_inputs: Vec<[u64; keccak::keccak_stark::NUM_INPUTS]>,
    pub(crate) logic_ops: Vec<logic::Operation>,
}

impl<F: Field> GenerationState<F> {
    /// Compute logical AND, and record the operation to be added in the logic table later.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn and(&mut self, input0: U256, input1: U256) -> U256 {
        self.logic_op(Op::And, input0, input1)
    }

    /// Compute logical OR, and record the operation to be added in the logic table later.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn or(&mut self, input0: U256, input1: U256) -> U256 {
        self.logic_op(Op::Or, input0, input1)
    }

    /// Compute logical XOR, and record the operation to be added in the logic table later.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn xor(&mut self, input0: U256, input1: U256) -> U256 {
        self.logic_op(Op::Xor, input0, input1)
    }

    /// Compute logical AND, and record the operation to be added in the logic table later.
    pub(crate) fn logic_op(&mut self, op: Op, input0: U256, input1: U256) -> U256 {
        let operation = Operation::new(op, input0, input1);
        let result = operation.result;
        self.logic_ops.push(operation);
        result
    }

    /// Read some memory within the current execution context, and log the operation.
    #[allow(unused)] // TODO: Should be used soon.
    pub(crate) fn get_mem_current(
        &mut self,
        channel_index: usize,
        segment: usize,
        virt: usize,
    ) -> [F; crate::memory::VALUE_LIMBS] {
        let timestamp = self.cpu_rows.len();
        let context = self.current_context;
        let value = self.memory.contexts[context].segments[segment].get(virt);
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
        segment: usize,
        virt: usize,
        value: [F; crate::memory::VALUE_LIMBS],
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
        self.memory.contexts[context].segments[segment].set(virt, value)
    }

    pub(crate) fn commit_cpu_row(&mut self) {
        self.cpu_rows.push(self.current_cpu_row);
        self.current_cpu_row = [F::ZERO; NUM_CPU_COLUMNS];
    }
}

// `GenerationState` can't `derive(Default)` because `Default` is only implemented for arrays up to
// length 32 :-\.
impl<F: Field> Default for GenerationState<F> {
    fn default() -> Self {
        Self {
            kernel: combined_kernel(),
            cpu_rows: vec![],
            current_cpu_row: [F::ZERO; NUM_CPU_COLUMNS],
            current_context: 0,
            memory: MemoryState::default(),
            keccak_inputs: vec![],
            logic_ops: vec![],
        }
    }
}
