use ethereum_types::U256;
use plonky2_util::ceil_div_usize;

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

#[derive(Clone, Debug)]
pub(crate) struct MemoryContextState {
    /// The content of each memory segment.
    pub segments: [MemorySegmentState; Segment::COUNT],
}

impl Default for MemoryContextState {
    fn default() -> Self {
        Self {
            segments: Segment::all().map(|segment| MemorySegmentState {
                content: vec![],
                segment,
                msize: 0,
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MemorySegmentState {
    pub content: Vec<U256>,
    pub segment: Segment,
    pub msize: usize,
}

impl MemorySegmentState {
    pub(crate) fn get(&mut self, virtual_addr: usize) -> U256 {
        self.update_msize(virtual_addr);
        self.content
            .get(virtual_addr)
            .copied()
            .unwrap_or(U256::zero())
    }

    pub(crate) fn set(&mut self, virtual_addr: usize, value: U256) {
        assert_eq!(value >> self.segment.bit_range(), U256::zero());
        self.update_msize(virtual_addr);
        if virtual_addr >= self.content.len() {
            self.content.resize(virtual_addr + 1, U256::zero());
        }
        self.content[virtual_addr] = value;
    }

    fn update_msize(&mut self, virtual_addr: usize) {
        let word_size = 256 / self.segment.bit_range();
        let new_msize = ceil_div_usize(virtual_addr + 1, word_size) * 32;
        self.msize = self.msize.max(new_msize);
    }
}
