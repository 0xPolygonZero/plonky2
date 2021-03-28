use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;

const MIN_WIRES: usize = 120; // TODO: Double check.
const MIN_ROUTED_WIRES: usize = 12; // TODO: Double check.

pub fn add_recursive_verifier<F: Field>(builder: &mut CircuitBuilder<F>) {
    assert!(builder.config.num_wires >= MIN_WIRES);
    assert!(builder.config.num_wires >= MIN_ROUTED_WIRES);
}
