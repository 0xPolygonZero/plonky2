#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::ops::Range;

use serde::{Deserialize, Serialize};

use crate::plonk::circuit_data::CircuitConfig;

/// Represents a wire in the circuit, seen as a `degree x num_wires` table.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct Wire {
    /// Row index of the wire.
    pub row: usize,
    /// Column index of the wire.
    pub column: usize,
}

impl Wire {
    pub const fn is_routable(&self, config: &CircuitConfig) -> bool {
        self.column < config.num_routed_wires
    }

    pub fn from_range(gate: usize, range: Range<usize>) -> Vec<Self> {
        range
            .map(|i| Wire {
                row: gate,
                column: i,
            })
            .collect()
    }
}
