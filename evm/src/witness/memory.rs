use ethereum_types::U256;

use crate::cpu::membus::{NUM_CHANNELS, NUM_GP_CHANNELS};

#[derive(Clone, Copy, Debug)]
pub(crate) enum MemoryChannel {
    Code,
    GeneralPurpose(usize),
    PartialChannel,
}

use MemoryChannel::{Code, GeneralPurpose, PartialChannel};

use super::operation::CONTEXT_SCALING_FACTOR;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::memory::segments::{Segment, SEGMENT_SCALING_FACTOR};
use crate::witness::errors::MemoryError::{ContextTooLarge, SegmentTooLarge, VirtTooLarge};
use crate::witness::errors::ProgramError;
use crate::witness::errors::ProgramError::MemoryError;

impl MemoryChannel {
    pub(crate) fn index(&self) -> usize {
        match *self {
            Code => 0,
            GeneralPurpose(n) => {
                assert!(n < NUM_GP_CHANNELS);
                n + 1
            }
            PartialChannel => NUM_GP_CHANNELS + 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) struct MemoryAddress {
    pub(crate) context: usize,
    pub(crate) segment: usize,
    pub(crate) virt: usize,
}

impl MemoryAddress {
    pub(crate) const fn new(context: usize, segment: Segment, virt: usize) -> Self {
        Self {
            context,
            // segment is scaled
            segment: segment.unscale(),
            virt,
        }
    }

    pub(crate) fn new_u256s(
        context: U256,
        segment: U256,
        virt: U256,
    ) -> Result<Self, ProgramError> {
        if context.bits() > 32 {
            return Err(MemoryError(ContextTooLarge { context }));
        }
        if segment >= Segment::COUNT.into() {
            return Err(MemoryError(SegmentTooLarge { segment }));
        }
        if virt.bits() > 32 {
            return Err(MemoryError(VirtTooLarge { virt }));
        }

        // Calling `as_usize` here is safe as those have been checked above.
        Ok(Self {
            context: context.as_usize(),
            segment: segment.as_usize(),
            virt: virt.as_usize(),
        })
    }

    /// Creates a new `MemoryAddress` from a bundled address fitting a `U256`.
    /// It will recover the virtual offset as the lowest 32-bit limb, the segment
    /// as the next limb, and the context as the next one.
    pub(crate) fn new_bundle(addr: U256) -> Result<Self, ProgramError> {
        let virt = addr.low_u32().into();
        let segment = (addr >> SEGMENT_SCALING_FACTOR).low_u32().into();
        let context = (addr >> CONTEXT_SCALING_FACTOR).low_u32().into();

        Self::new_u256s(context, segment, virt)
    }

    pub(crate) fn increment(&mut self) {
        self.virt = self.virt.saturating_add(1);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MemoryOpKind {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MemoryOp {
    /// true if this is an actual memory operation, or false if it's a padding row.
    pub filter: bool,
    pub timestamp: usize,
    pub address: MemoryAddress,
    pub kind: MemoryOpKind,
    pub value: U256,
}

pub(crate) static DUMMY_MEMOP: MemoryOp = MemoryOp {
    filter: false,
    timestamp: 0,
    address: MemoryAddress {
        context: 0,
        segment: 0,
        virt: 0,
    },
    kind: MemoryOpKind::Read,
    value: U256::zero(),
};

impl MemoryOp {
    pub(crate) fn new(
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

    pub(crate) const fn new_dummy_read(
        address: MemoryAddress,
        timestamp: usize,
        value: U256,
    ) -> Self {
        Self {
            filter: false,
            timestamp,
            address,
            kind: MemoryOpKind::Read,
            value,
        }
    }

    pub(crate) const fn sorting_key(&self) -> (usize, usize, usize, usize) {
        (
            self.address.context,
            self.address.segment,
            self.address.virt,
            self.timestamp,
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MemoryState {
    pub(crate) contexts: Vec<MemoryContextState>,
}

impl MemoryState {
    pub(crate) fn new(kernel_code: &[u8]) -> Self {
        let code_u256s = kernel_code.iter().map(|&x| x.into()).collect();
        let mut result = Self::default();
        result.contexts[0].segments[Segment::Code.unscale()].content = code_u256s;
        result
    }

    pub(crate) fn apply_ops(&mut self, ops: &[MemoryOp]) {
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

    pub(crate) fn get(&self, address: MemoryAddress) -> U256 {
        if address.context >= self.contexts.len() {
            return U256::zero();
        }

        let segment = Segment::all()[address.segment];

        if let Some(constant) = Segment::constant(&segment, address.virt) {
            return constant;
        }

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

    pub(crate) fn set(&mut self, address: MemoryAddress, val: U256) {
        while address.context >= self.contexts.len() {
            self.contexts.push(MemoryContextState::default());
        }

        let segment = Segment::all()[address.segment];

        if let Some(constant) = Segment::constant(&segment, address.virt) {
            assert!(
                constant == val,
                "Attempting to set constant {} to incorrect value",
                address.virt
            );
            return;
        }
        assert!(
            val.bits() <= segment.bit_range(),
            "Value {} exceeds {:?} range of {} bits",
            val,
            segment,
            segment.bit_range()
        );
        self.contexts[address.context].segments[address.segment].set(address.virt, val);
    }

    // These fields are already scaled by their respective segment.
    pub(crate) fn read_global_metadata(&self, field: GlobalMetadata) -> U256 {
        self.get(MemoryAddress::new_bundle(U256::from(field as usize)).unwrap())
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

#[derive(Clone, Debug)]
pub(crate) struct MemoryContextState {
    /// The content of each memory segment.
    pub(crate) segments: [MemorySegmentState; Segment::COUNT],
}

impl Default for MemoryContextState {
    fn default() -> Self {
        Self {
            segments: std::array::from_fn(|_| MemorySegmentState::default()),
        }
    }
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
