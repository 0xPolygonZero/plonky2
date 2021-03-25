use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;
use crate::gates::deterministic_gate::{DeterministicGate, DeterministicGateAdapter};
use crate::gates::gate::{Gate, GateRef};
use crate::generator::WitnessGenerator;

/// A gate which takes a single constant parameter and outputs that value.
pub struct NoopGate;

impl NoopGate {
    pub fn get<F: Field>() -> GateRef<F> {
        GateRef::new(NoopGate)
    }
}

impl<F: Field> Gate<F> for NoopGate {
    fn id(&self) -> String {
        "NoopGate".into()
    }

    fn constraints(&self, _config: CircuitConfig) -> Vec<ConstraintPolynomial<F>> {
        Vec::new()
    }

    fn generators(
        &self,
        _config: CircuitConfig,
        _gate_index: usize,
        _local_constants: Vec<F>,
        _next_constants: Vec<F>
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        Vec::new()
    }
}
