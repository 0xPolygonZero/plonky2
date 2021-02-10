use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;
use crate::gates::deterministic_gate::DeterministicGate;
use crate::gates::gate::{Gate2, GateRef};

/// Evaluates a full GMiMC permutation.
#[derive(Debug)]
pub struct GMiMCGate<F: Field> {
    num_rounds: usize,
    width: usize,
    round_constants: Vec<F>,
}

impl<F: Field> GMiMCGate<F> {
    fn new(width: usize) -> GateRef<F> {
        todo!()
    }
}

impl<F: Field> DeterministicGate<F> for GMiMCGate<F> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn outputs(&self, config: CircuitConfig) -> Vec<(usize, ConstraintPolynomial<F>)> {
        unimplemented!()
    }
}
