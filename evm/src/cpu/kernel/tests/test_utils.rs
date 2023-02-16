use std::ops::Range;

use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment;
use crate::witness::memory::MemoryAddress;

pub struct InterpreterSetup {
    pub label: String,
    pub stack: Vec<U256>,
    pub segment: Segment,
    pub memory: Vec<(usize, Vec<U256>)>,
}

impl InterpreterSetup {
    pub fn run(self) -> Result<Interpreter<'static>> {
        let label = KERNEL.global_labels[&self.label];
        let mut stack = self.stack;
        stack.reverse();
        let mut interpreter = Interpreter::new_with_kernel(label, stack);
        for (pointer, data) in self.memory {
            for (i, term) in data.iter().enumerate() {
                interpreter
                    .generation_state
                    .memory
                    .set(MemoryAddress::new(0, self.segment, pointer + i), *term)
            }
        }
        interpreter.run()?;
        Ok(interpreter)
    }
}
