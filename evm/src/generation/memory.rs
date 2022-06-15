use plonky2::field::types::Field;

use crate::memory::memory_stark::MemoryOp;
use crate::memory::segments::NUM_SEGMENTS;
use crate::memory::VALUE_LIMBS;

#[allow(unused)] // TODO: Should be used soon.
#[derive(Debug)]
pub(crate) struct MemoryState<F: Field> {
    /// A log of each memory operation, in the order that it occurred.
    pub log: Vec<MemoryOp<F>>,

    pub contexts: Vec<MemoryContextState<F>>,
}

impl<F: Field> Default for MemoryState<F> {
    fn default() -> Self {
        Self {
            log: vec![],
            // We start with an initial context for the kernel.
            contexts: vec![MemoryContextState::default()],
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct MemoryContextState<F: Field> {
    /// The content of each memory segment.
    pub segments: [MemorySegmentState<F>; NUM_SEGMENTS],
}

#[derive(Default, Debug)]
pub(crate) struct MemorySegmentState<F: Field> {
    pub content: Vec<[F; VALUE_LIMBS]>,
}

impl<F: Field> MemorySegmentState<F> {
    pub(super) fn get(&self, virtual_addr: usize) -> [F; VALUE_LIMBS] {
        self.content
            .get(virtual_addr)
            .copied()
            .unwrap_or([F::ZERO; VALUE_LIMBS])
    }

    pub(super) fn set(&mut self, virtual_addr: usize, value: [F; VALUE_LIMBS]) {
        if virtual_addr + 1 > self.content.len() {
            self.content
                .resize(virtual_addr + 1, [F::ZERO; VALUE_LIMBS]);
        }
        self.content[virtual_addr] = value;
    }
}
