use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::WitnessGenerator;
use crate::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate which does nothing.
pub struct NoopGate;

impl NoopGate {
    pub fn get<F: Extendable<D>, const D: usize>() -> GateRef<F, D> {
        GateRef::new(NoopGate)
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for NoopGate {
    fn id(&self) -> String {
        "NoopGate".into()
    }

    fn eval_unfiltered(&self, _vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        Vec::new()
    }

    fn eval_unfiltered_base(&self, _vars: EvaluationVarsBase<F>) -> Vec<F> {
        Vec::new()
    }

    fn eval_unfiltered_recursively(
        &self,
        _builder: &mut CircuitBuilder<F, D>,
        _vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        Vec::new()
    }

    fn generators(
        &self,
        _gate_index: usize,
        _local_constants: &[F],
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

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::gates::gate_testing::test_low_degree;
    use crate::gates::noop::NoopGate;

    #[test]
    fn low_degree() {
        test_low_degree(NoopGate::get::<CrandallField, 4>())
    }
}
