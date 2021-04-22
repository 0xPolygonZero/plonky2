use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::{CircuitConfig, VerifierCircuitTarget};
use crate::field::field::Field;
use crate::gates::gate::GateRef;
use crate::proof::ProofTarget;

const MIN_WIRES: usize = 120; // TODO: Double check.
const MIN_ROUTED_WIRES: usize = 8; // TODO: Double check.

/// Recursively verifies an inner proof.
pub fn add_recursive_verifier<F: Field>(
    builder: &mut CircuitBuilder<F>,
    inner_config: CircuitConfig,
    inner_circuit: VerifierCircuitTarget,
    inner_gates: Vec<GateRef<F>>,
    inner_proof: ProofTarget,
) {
    assert!(builder.config.num_wires >= MIN_WIRES);
    assert!(builder.config.num_wires >= MIN_ROUTED_WIRES);

    todo!()
}
