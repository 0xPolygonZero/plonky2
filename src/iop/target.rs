use std::ops::Range;

use crate::iop::wire::Wire;
use crate::plonk::circuit_data::CircuitConfig;

/// A location in the witness.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Target {
    Wire(Wire),
    /// A target that doesn't have any inherent location in the witness (but it can be copied to
    /// another target that does). This is useful for representing intermediate values in witness
    /// generation.
    VirtualTarget {
        index: usize,
    },
}

impl Target {
    pub fn wire(gate: usize, input: usize) -> Self {
        Self::Wire(Wire { gate, input })
    }

    pub fn is_routable(&self, config: &CircuitConfig) -> bool {
        match self {
            Target::Wire(wire) => wire.is_routable(config),
            Target::VirtualTarget { .. } => true,
        }
    }

    pub fn wires_from_range(gate: usize, range: Range<usize>) -> Vec<Self> {
        range.map(|i| Self::wire(gate, i)).collect()
    }
}

/// A `Target` which has already been constrained such that it can only be 0 or 1.
#[derive(Copy, Clone, Debug)]
pub struct BoolTarget {
    pub target: Target,
    /// This private field is here to force all instantiations to go through `new_unsafe`.
    _private: (),
}

impl BoolTarget {
    pub fn new_unsafe(target: Target) -> BoolTarget {
        BoolTarget {
            target,
            _private: (),
        }
    }
}
