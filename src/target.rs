use std::convert::Infallible;
use std::marker::PhantomData;

use crate::circuit_data::CircuitConfig;
use crate::field::field::Field;
use crate::wire::Wire;

/// A location in the witness.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Target2 {
    Wire(Wire),
    PublicInput { index: usize },
    VirtualAdviceTarget { index: usize },
}

impl Target2 {
    pub fn wire(gate: usize, input: usize) -> Self {
        Self::Wire(Wire { gate, input })
    }

    pub fn is_routable(&self, config: CircuitConfig) -> bool {
        match self {
            Target2::Wire(wire) => wire.is_routable(config),
            Target2::PublicInput { .. } => true,
            Target2::VirtualAdviceTarget { .. } => false,
        }
    }
}
