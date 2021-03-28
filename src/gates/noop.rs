use crate::circuit_data::CircuitConfig;
use crate::constraint_polynomial::{EvaluationVars, EvaluationTargets};
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::WitnessGenerator;
use crate::target::Target;
use crate::circuit_builder::CircuitBuilder;

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

    fn eval_unfiltered(&self, vars: EvaluationVars<F>) -> Vec<F> {
        Vec::new()
    }

    fn eval_unfiltered_recursively(
        &self,
        _builder: &mut CircuitBuilder<F>,
        vars: EvaluationTargets,
    ) -> Vec<Target> {
        Vec::new()
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
        next_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        Vec::new()
    }

    fn num_wires(&self) -> usize {
        0
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        0
    }

    fn num_constraints(&self) -> usize {
        0
    }
}
