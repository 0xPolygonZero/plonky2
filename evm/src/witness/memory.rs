use std::collections::HashMap;

use ethereum_types::U256;

use crate::cpu::membus::{NUM_CHANNELS, NUM_GP_CHANNELS};

#[derive(Clone, Copy, Debug)]
pub enum MemoryChannel {
    Code,
    GeneralPurpose(usize),
}

use MemoryChannel::{Code, GeneralPurpose};

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

pub type MemoryAddress = (u32, u32, u32);

#[derive(Clone, Copy, Debug)]
pub enum MemoryOpKind {
    Read,
    Write(U256),
}

#[derive(Clone, Copy, Debug)]
pub struct MemoryOp {
    pub timestamp: u64,
    pub address: MemoryAddress,
    pub op: MemoryOpKind,
}

impl MemoryOp {
    pub fn new(
        channel: MemoryChannel,
        clock: usize,
        address: MemoryAddress,
        op: MemoryOpKind,
    ) -> Self {
        let timestamp = (clock * NUM_CHANNELS + channel.index()) as u64;
        MemoryOp {
            timestamp,
            address,
            op,
        }
    }
}

#[derive(Clone)]
pub struct MemoryState {
    contents: HashMap<MemoryAddress, U256>,
}

impl MemoryState {
    pub fn new(kernel_code: &[u8]) -> Self {
        let mut contents = HashMap::new();

        for (i, &byte) in kernel_code.iter().enumerate() {
            if byte != 0 {
                let address = (0, 0, i as u32);
                let val = byte.into();
                contents.insert(address, val);
            }
        }

        Self { contents }
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
