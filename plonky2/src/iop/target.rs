use std::ops::Range;

use crate::iop::wire::Wire;
use crate::plonk::circuit_data::CircuitConfig;

/// A location in the witness.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Target(pub usize);

impl Target {}

/// A `Target` which has already been constrained such that it can only be 0 or 1.
#[derive(Copy, Clone, Debug)]
#[allow(clippy::manual_non_exhaustive)]
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
