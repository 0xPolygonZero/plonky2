use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// A gate which can be configured to perform various arithmetic. In particular, it computes
///
/// ```text
/// output := const_0 * multiplicand_0 * multiplicand_1 + const_1 * addend
/// ```
#[derive(Debug)]
pub struct ArithmeticGate;

impl ArithmeticGate {
    pub fn new<F: Field>() -> GateRef<F> {
        GateRef::new(ArithmeticGate)
    }

    pub const WIRE_MULTIPLICAND_0: usize = 0;
    pub const WIRE_MULTIPLICAND_1: usize = 1;
    pub const WIRE_ADDEND: usize = 2;
    pub const WIRE_OUTPUT: usize = 3;
}

impl<F: Field> Gate<F> for ArithmeticGate {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F>) -> Vec<F> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];
        let multiplicand_0 = vars.local_wires[Self::WIRE_MULTIPLICAND_0];
        let multiplicand_1 = vars.local_wires[Self::WIRE_MULTIPLICAND_1];
        let addend = vars.local_wires[Self::WIRE_ADDEND];
        let output = vars.local_wires[Self::WIRE_OUTPUT];
        let computed_output = const_0 * multiplicand_0 * multiplicand_1 + const_1 * addend;
        vec![computed_output - output]
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F>,
        vars: EvaluationTargets,
    ) -> Vec<Target> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];
        let multiplicand_0 = vars.local_wires[Self::WIRE_MULTIPLICAND_0];
        let multiplicand_1 = vars.local_wires[Self::WIRE_MULTIPLICAND_1];
        let addend = vars.local_wires[Self::WIRE_ADDEND];
        let output = vars.local_wires[Self::WIRE_OUTPUT];

        let product_term = builder.mul_many(&[const_0, multiplicand_0, multiplicand_1]);
        let addend_term = builder.mul(const_1, addend);
        let computed_output = builder.add_many(&[product_term, addend_term]);
        vec![builder.sub(computed_output, output)]
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
        _next_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = ArithmeticGenerator {
            gate_index,
            const_0: local_constants[0],
            const_1: local_constants[1],
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        4
    }

    fn num_constants(&self) -> usize {
        2
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        1
    }
}

struct ArithmeticGenerator<F: Field> {
    gate_index: usize,
    const_0: F,
    const_1: F,
}

impl<F: Field> SimpleGenerator<F> for ArithmeticGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        vec![
            Target::Wire(Wire {
                gate: self.gate_index,
                input: ArithmeticGate::WIRE_MULTIPLICAND_0,
            }),
            Target::Wire(Wire {
                gate: self.gate_index,
                input: ArithmeticGate::WIRE_MULTIPLICAND_1,
            }),
            Target::Wire(Wire {
                gate: self.gate_index,
                input: ArithmeticGate::WIRE_ADDEND,
            }),
        ]
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let multiplicand_0_target = Wire {
            gate: self.gate_index,
            input: ArithmeticGate::WIRE_MULTIPLICAND_0,
        };
        let multiplicand_1_target = Wire {
            gate: self.gate_index,
            input: ArithmeticGate::WIRE_MULTIPLICAND_1,
        };
        let addend_target = Wire {
            gate: self.gate_index,
            input: ArithmeticGate::WIRE_ADDEND,
        };
        let output_target = Wire {
            gate: self.gate_index,
            input: ArithmeticGate::WIRE_OUTPUT,
        };

        let multiplicand_0 = witness.get_wire(multiplicand_0_target);
        let multiplicand_1 = witness.get_wire(multiplicand_1_target);
        let addend = witness.get_wire(addend_target);

        let output = self.const_0 * multiplicand_0 * multiplicand_1 + self.const_1 * addend;

        PartialWitness::singleton_wire(output_target, output)
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::{test_gate_low_degree, ArithmeticGate, Tweedledum};
//
//     test_gate_low_degree!(
//         low_degree_ArithmeticGate,
//         Tweedledum,
//         ArithmeticGate<Tweedledum>
//     );
// }
