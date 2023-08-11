use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;

const KERNEL_CONTEXT: usize = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegistersState {
    pub program_counter: usize,
    pub is_kernel: bool,
    pub stack_len: usize,
    pub stack_top: U256,
    pub is_stack_top_read: bool,
    pub context: usize,
    pub gas_used: u64,
}

impl RegistersState {
    pub(crate) fn code_context(&self) -> usize {
        if self.is_kernel {
            KERNEL_CONTEXT
        } else {
            self.context
        }
    }
}

impl Default for RegistersState {
    fn default() -> Self {
        Self {
            program_counter: KERNEL.global_labels["main"],
            is_kernel: true,
            stack_len: 0,
            stack_top: U256::zero(),
            is_stack_top_read: false,
            context: 0,
            gas_used: 0,
        }
    }
}
