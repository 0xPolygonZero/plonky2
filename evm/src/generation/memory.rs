use ethereum_types::U256;

use crate::memory::memory_stark::MemoryOp;
use crate::memory::segments::Segment;

#[allow(unused)] // TODO: Should be used soon.
#[derive(Debug)]
pub(crate) struct MemoryState {
    /// A log of each memory operation, in the order that it occurred.
    pub log: Vec<MemoryOp>,

    pub contexts: Vec<MemoryContextState>,
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            log: vec![],
            // We start with an initial context for the kernel.
            contexts: vec![MemoryContextState::default()],
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct MemoryContextState {
    /// The content of each memory segment.
    pub segments: [MemorySegmentState; Segment::COUNT],
}

#[derive(Default, Debug)]
pub(crate) struct MemorySegmentState {
    pub content: Vec<U256>,
}

impl MemorySegmentState {
    pub(crate) fn get(&self, virtual_addr: usize) -> U256 {
        self.content
            .get(virtual_addr)
            .copied()
            .unwrap_or(U256::zero())
    }

    pub(crate) fn set(&mut self, virtual_addr: usize, value: U256) {
        if virtual_addr >= self.content.len() {
            self.content.resize(virtual_addr + 1, U256::zero());
        }
        self.content[virtual_addr] = value;
    }
}
