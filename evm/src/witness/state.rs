use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::witness::errors::ProgramError;
use crate::witness::memory::{MemoryAddress, MemoryChannel, MemoryOp, MemoryOpKind, MemoryState};

pub const KERNEL_CONTEXT: u32 = 0;
pub const MAX_USER_STACK_SIZE: u32 = crate::cpu::stack_bounds::MAX_USER_STACK_SIZE as u32;

#[derive(Clone)]
pub struct State {
    pub clock: usize,
    
    pub program_counter: u32,

    pub is_kernel: bool,
    pub stack_len: u32,
    pub context: u32,

    pub memory: MemoryState,
}

impl State {
    pub fn initial(clock: usize, memory: MemoryState) -> Self {
        Self {
            clock,
            program_counter: KERNEL.global_labels["main"] as u32,
            is_kernel: true,
            stack_len: 0,
            context: KERNEL_CONTEXT,
            memory: memory,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.is_kernel && [
            KERNEL.global_labels["halt_pc0"] as u32,
            KERNEL.global_labels["halt_pc1"] as u32,
        ].contains(&self.program_counter)
    }
    
    pub fn mem_write_log(
        &self,
        channel: MemoryChannel,
        address: MemoryAddress,
        val: U256,
    ) -> MemoryOp {
        MemoryOp::new(channel, self.clock, address, MemoryOpKind::Write(val))
    }

    pub fn mem_write_with_log(
        &mut self,
        channel: MemoryChannel,
        address: MemoryAddress,
        val: U256,
    ) -> MemoryOp {
        (self.memory.set(address, val), self.mem_write_log(channel, address, val))
    }

    pub fn mem_read_log(
        &self,
        channel: MemoryChannel,
        address: MemoryAddress,
    ) -> MemoryOp {
        MemoryOp::new(channel, self.clock, address, MemoryOpKind::Read)
    }

    pub fn mem_read_with_log(
        &self,
        channel: MemoryChannel,
        address: MemoryAddress,
    ) -> (U256, MemoryOp) {
        (self.memory.get(address), self.mem_read_log(channel, address))
    }

    pub fn pop_stack_with_log<const N: usize>(&mut self) -> Result<[(U256, MemoryOp); N], ProgramError> {
        if stack_len < N {
            return Err(ProgramError::StackUnderflow);
        }

        let mut result = [U256::default(); N];
        for i in 0..N {
            let channel = GeneralPurpose(i);
            let address = (self.context, Segment::Stack as u32, self.stack_len - 1 - i);
            result[i] = self.mem_read_with_log(channel, address);
        }

        self.stack_len -= N;
        Ok(result)
    }

    pub fn push_stack_with_log(&mut self, val: U256) -> Result<MemoryOp, ProgramError> {
        if !self.is_kernel_mode {
            assert!(self.stack_len <= MAX_USER_STACK_SIZE);
            if self.stack_len == MAX_USER_STACK_SIZE {
                return Err(ProgramError::StackOverflow);
            }
        }

        let channel = GeneralPurpose(NUM_GP_CHANNELS - 1);
        let address = (self.context, Segment::Stack as u32, self.stack_len);
        let result = self.mem_write_with_log(channel, address, val);

        self.stack_len += 1;
        Ok(result)
    }

}
