use crate::circuit_builder::CircuitBuilder;
use crate::constraint_polynomial::{EvaluationTargets, EvaluationVars};
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// A gate which takes a single constant parameter and outputs that value.
pub struct ConstantGate;

impl ConstantGate {
    pub fn get<F: Field>() -> GateRef<F> {
        GateRef::new(ConstantGate)
    }

    pub const CONST_INPUT: usize = 0;

    pub const WIRE_OUTPUT: usize = 0;
}

impl<F: Field> Gate<F> for ConstantGate {
    fn id(&self) -> String {
        "ConstantGate".into()
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F>) -> Vec<F> {
        let input = vars.local_constants[Self::CONST_INPUT];
        let output = vars.local_wires[Self::WIRE_OUTPUT];
        vec![output - input]
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F>,
        vars: EvaluationTargets,
    ) -> Vec<Target> {
        let input = vars.local_constants[Self::CONST_INPUT];
        let output = vars.local_wires[Self::WIRE_OUTPUT];
        vec![builder.sub(output, input)]
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
        _next_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = ConstantGenerator {
            gate_index,
            constant: local_constants[0],
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        1
    }

    fn num_constants(&self) -> usize {
        1
    }

    fn degree(&self) -> usize {
        1
    }

    fn num_constraints(&self) -> usize {
        1
    }
}

#[derive(Debug)]
struct ConstantGenerator<F: Field> {
    gate_index: usize,
    constant: F,
}

impl<F: Field> SimpleGenerator<F> for ConstantGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        Vec::new()
    }

    fn run_once(&self, _witness: &PartialWitness<F>) -> PartialWitness<F> {
        let wire = Wire { gate: self.gate_index, input: ConstantGate::WIRE_OUTPUT };
        PartialWitness::singleton_target(Target::Wire(wire), self.constant)
    }
}
