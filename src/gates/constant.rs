use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;
use crate::gates::deterministic_gate::{DeterministicGate, DeterministicGateAdapter};
use crate::gates::gate::GateRef;
use crate::gates::output_graph::{GateOutputLocation, OutputGraph};

/// A gate which takes a single constant parameter and outputs that value.
pub struct ConstantGate2;

impl ConstantGate2 {
    pub fn get<F: Field>() -> GateRef<F> {
        GateRef::new(DeterministicGateAdapter::new(ConstantGate2))
    }

    pub const CONST_INPUT: usize = 0;

    pub const WIRE_OUTPUT: usize = 0;
}

impl<F: Field> DeterministicGate<F> for ConstantGate2 {
    fn id(&self) -> String {
        "ConstantGate".into()
    }

    fn outputs(&self, _config: CircuitConfig) -> OutputGraph<F> {
        let loc = GateOutputLocation::LocalWire(Self::WIRE_OUTPUT);
        let out = ConstraintPolynomial::local_constant(Self::CONST_INPUT);
        OutputGraph::single_output(loc, out)
    }
}
