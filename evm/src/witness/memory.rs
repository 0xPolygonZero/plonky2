use ethereum_types::U256;

use crate::cpu::membus::{NUM_CHANNELS, NUM_GP_CHANNELS};

#[derive(Clone, Copy, Debug)]
pub enum MemoryChannel {
    Code,
    GeneralPurpose(usize),
}

use MemoryChannel::{Code, GeneralPurpose};

use crate::memory::segments::Segment;

impl MemoryChannel {
    pub fn index(&self) -> usize {
        match *self {
            Code => 0,
            GeneralPurpose(n) => {
                assert!(n < NUM_GP_CHANNELS);
                n + 1
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct MemoryAddress {
    pub(crate) context: usize,
    pub(crate) segment: usize,
    pub(crate) virt: usize,
}

impl MemoryAddress {
    pub(crate) fn new(context: usize, segment: Segment, virt: usize) -> Self {
        Self {
            context,
            segment: segment as usize,
            virt,
        }
    }

    pub(crate) fn new_u256s(context: U256, segment: U256, virt: U256) -> Self {
        assert!(context.bits() <= 32, "context too large: {}", context);
        assert!(
            segment < Segment::COUNT.into(),
            "segment too large: {}",
            segment
        );
        assert!(virt.bits() <= 32, "virt too large: {}", virt);
        Self {
            context: context.as_usize(),
            segment: segment.as_usize(),
            virt: virt.as_usize(),
        }
    }

    pub(crate) fn increment(&mut self) {
        self.virt = self.virt.saturating_add(1);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryOpKind {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug)]
pub struct MemoryOp {
    /// true if this is an actual memory operation, or false if it's a padding row.
    pub filter: bool,
    pub timestamp: usize,
    pub address: MemoryAddress,
    pub kind: MemoryOpKind,
    pub value: U256,
}

impl MemoryOp {
    pub fn new(
        channel: MemoryChannel,
        clock: usize,
        address: MemoryAddress,
        kind: MemoryOpKind,
        value: U256,
    ) -> Self {
        let timestamp = clock * NUM_CHANNELS + channel.index();
        MemoryOp {
            filter: true,
            timestamp,
            address,
            kind,
            value,
        }
    }

    pub(crate) fn new_dummy_read(address: MemoryAddress, timestamp: usize, value: U256) -> Self {
        Self {
            filter: false,
            timestamp,
            address,
            kind: MemoryOpKind::Read,
            value,
        }
    }

    pub(crate) fn sorting_key(&self) -> (usize, usize, usize, usize) {
        (
            self.address.context,
            self.address.segment,
            self.address.virt,
            self.timestamp,
        )
    }
}

#[derive(Clone, Debug)]
pub struct MemoryState {
    pub(crate) contexts: Vec<MemoryContextState>,
}

impl MemoryState {
    pub fn new(kernel_code: &[u8]) -> Self {
        let code_u256s = kernel_code.iter().map(|&x| x.into()).collect();
        let mut result = Self::default();
        result.contexts[0].segments[Segment::Code as usize].content = code_u256s;
        result
    }

    pub fn apply_ops(&mut self, ops: &[MemoryOp]) {
        for &op in ops {
            let MemoryOp {
                address,
                kind,
                value,
                ..
            } = op;
            if kind == MemoryOpKind::Write {
                self.set(address, value);
            }
        }
    }

    pub fn get(&self, address: MemoryAddress) -> U256 {
        if address.context >= self.contexts.len() {
            return U256::zero();
        }

        let segment = Segment::all()[address.segment];
        let val = self.contexts[address.context].segments[address.segment].get(address.virt);
        assert!(
            val.bits() <= segment.bit_range(),
            "Value {} exceeds {:?} range of {} bits",
            val,
            segment,
            segment.bit_range()
        );
        val
    }

    pub fn set(&mut self, address: MemoryAddress, val: U256) {
        while address.context >= self.contexts.len() {
            self.contexts.push(MemoryContextState::default());
        }

        let segment = Segment::all()[address.segment];
        assert!(
            val.bits() <= segment.bit_range(),
            "Value {} exceeds {:?} range of {} bits",
            val,
            segment,
            segment.bit_range()
        );
        self.contexts[address.context].segments[address.segment].set(address.virt, val);
    }
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            // We start with an initial context for the kernel.
            contexts: vec![MemoryContextState::default()],
        }
    }
}

#[derive(Clone, Default, Debug)]
pub(crate) struct MemoryContextState {
    /// The content of each memory segment.
    pub(crate) segments: [MemorySegmentState; Segment::COUNT],
}

#[derive(Clone, Default, Debug)]
pub(crate) struct MemorySegmentState {
    pub(crate) content: Vec<U256>,
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
