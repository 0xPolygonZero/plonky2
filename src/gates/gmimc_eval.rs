use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::witness::PartialWitness;

/// Performs some arithmetic involved in the evaluation of GMiMC's constraint polynomials for one
/// round.
#[derive(Debug)]
pub struct GMiMCEvalGate;

impl GMiMCEvalGate {
    pub fn get<F: Field>() -> GateRef<F> {
        GateRef::new(GMiMCEvalGate)
    }
}

impl<F: Field> Gate<F> for GMiMCEvalGate {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F>) -> Vec<F> {
        todo!()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F>,
        vars: EvaluationTargets,
    ) -> Vec<Target> {
        unimplemented!()
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
        _next_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = GMiMCEvalGenerator::<F> {
            gate_index,
            constant: local_constants[0],
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        6
    }

    fn num_constants(&self) -> usize {
        1
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        unimplemented!()
    }
}

#[derive(Debug)]
struct GMiMCEvalGenerator<F: Field> {
    gate_index: usize,
    constant: F,
}

impl<F: Field> SimpleGenerator<F> for GMiMCEvalGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        todo!()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        todo!()
    }
}
