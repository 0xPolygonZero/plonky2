use std::collections::HashMap;

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
    pub op: MemoryOpKind,
    pub value: U256,
}

impl MemoryOp {
    pub fn new(
        channel: MemoryChannel,
        clock: usize,
        address: MemoryAddress,
        op: MemoryOpKind,
        value: U256,
    ) -> Self {
        let timestamp = clock * NUM_CHANNELS + channel.index();
        MemoryOp {
            filter: true,
            timestamp,
            address,
            op,
            value,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct MemoryState {
    contents: HashMap<MemoryAddress, U256>,
}

impl MemoryState {
    pub fn new(kernel_code: &[u8]) -> Self {
        let mut contents = HashMap::new();

        for (i, &byte) in kernel_code.iter().enumerate() {
            if byte != 0 {
                let address = MemoryAddress::new(0, Segment::Code, i);
                let val = byte.into();
                contents.insert(address, val);
            }
        }

        Self { contents }
    }

    pub fn apply_ops(&mut self, ops: &[MemoryOp]) {
        for &op in ops {
            let MemoryOp {
                address, op, value, ..
            } = op;
            if op == MemoryOpKind::Write {
                self.set(address, value);
            }
        }
    }

    pub fn get(&self, address: MemoryAddress) -> U256 {
        self.contents
            .get(&address)
            .copied()
            .unwrap_or_else(U256::zero)
    }

    pub fn set(&mut self, address: MemoryAddress, val: U256) {
        if val.is_zero() {
            self.contents.remove(&address);
        } else {
            self.contents.insert(address, val);
        }
    }
}
