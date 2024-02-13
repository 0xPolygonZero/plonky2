#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::ops::Range;

use serde::{Deserialize, Serialize};

use crate::iop::ext_target::ExtensionTarget;
use crate::iop::wire::Wire;
use crate::plonk::circuit_data::CircuitConfig;

/// A location in the witness.
///
/// Targets can either be placed at a specific location, or be "floating" around,
/// serving as intermediary value holders, and copied to other locations whenever needed.
///
/// When generating a proof for a given circuit, the prover will "set" the values of some
/// (or all) targets, so that they satisfy the circuit constraints.  This is done through
/// the [PartialWitness](crate::iop::witness::PartialWitness) interface.
///
/// There are different "variants" of the `Target` type, namely [`ExtensionTarget`],
/// [ExtensionAlgebraTarget](crate::iop::ext_target::ExtensionAlgebraTarget).
/// The `Target` type is the default one for most circuits verifying some simple statement.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum Target {
    /// A target that has a fixed location in the witness (seen as a `degree x num_wires` grid).
    Wire(Wire),
    /// A target that doesn't have any inherent location in the witness (but it can be copied to
    /// another target that does). This is useful for representing intermediate values in witness
    /// generation.
    VirtualTarget { index: usize },
}

impl Default for Target {
    fn default() -> Self {
        Self::VirtualTarget { index: 0 }
    }
}

impl Target {
    pub const fn wire(row: usize, column: usize) -> Self {
        Self::Wire(Wire { row, column })
    }

    pub const fn is_routable(&self, config: &CircuitConfig) -> bool {
        match self {
            Target::Wire(wire) => wire.is_routable(config),
            Target::VirtualTarget { .. } => true,
        }
    }

    pub fn wires_from_range(row: usize, range: Range<usize>) -> Vec<Self> {
        range.map(|i| Self::wire(row, i)).collect()
    }

    pub fn index(&self, num_wires: usize, degree: usize) -> usize {
        match self {
            Target::Wire(Wire { row, column }) => row * num_wires + column,
            Target::VirtualTarget { index } => degree * num_wires + index,
        }
    }

    /// Conversion to an `ExtensionTarget`.
    pub const fn to_ext_target<const D: usize>(self, zero: Self) -> ExtensionTarget<D> {
        let mut arr = [zero; D];
        arr[0] = self;
        ExtensionTarget(arr)
    }
}

/// A `Target` which has already been constrained such that it can only be 0 or 1.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[allow(clippy::manual_non_exhaustive)]
pub struct BoolTarget {
    pub target: Target,
    /// This private field is here to force all instantiations to go through `new_unsafe`.
    _private: (),
}

impl BoolTarget {
    pub const fn new_unsafe(target: Target) -> BoolTarget {
        BoolTarget {
            target,
            _private: (),
        }
    }
}
