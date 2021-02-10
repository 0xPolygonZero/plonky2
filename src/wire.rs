use crate::circuit_data::CircuitConfig;

/// Represents a wire in the circuit.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Wire {
    /// The index of the associated gate.
    pub gate: usize,
    /// The index of the gate input wherein this wire is inserted.
    pub input: usize,
}

impl Wire {
    pub fn is_routable(&self, config: CircuitConfig) -> bool {
        self.input < config.num_routed_wires
    }
}
