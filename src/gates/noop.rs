use crate::circuit_builder::CircuitBuilder;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::WitnessGenerator;
use crate::target::Target;

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

    fn eval_unfiltered(&self, _vars: EvaluationVars<F>) -> Vec<F> {
        Vec::new()
    }

    fn eval_unfiltered_recursively(
        &self,
        _builder: &mut CircuitBuilder<F>,
        _vars: EvaluationTargets,
    ) -> Vec<Target> {
        Vec::new()
    }

    fn generators(
        &self,
        _gate_index: usize,
        _local_constants: &[F],
        _next_constants: &[F],
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
