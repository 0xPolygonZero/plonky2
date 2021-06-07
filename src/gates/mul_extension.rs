use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;
use std::ops::Range;

/// A gate which can multiply to field extension elements.
/// TODO: Add an addend if `NUM_ROUTED_WIRES` is large enough.
#[derive(Debug)]
pub struct MulExtensionGate<const D: usize>;

impl<const D: usize> MulExtensionGate<D> {
    pub fn new<F: Extendable<D>>() -> GateRef<F, D> {
        GateRef::new(MulExtensionGate)
    }

    pub fn wires_multiplicand_0() -> Range<usize> {
        0..D
    }
    pub fn wires_multiplicand_1() -> Range<usize> {
        D..2 * D
    }
    pub fn wires_output() -> Range<usize> {
        2 * D..3 * D
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for MulExtensionGate<D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
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
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];
        let multiplicand_0 = vars.local_wires[Self::WIRE_MULTIPLICAND_0];
        let multiplicand_1 = vars.local_wires[Self::WIRE_MULTIPLICAND_1];
        let addend = vars.local_wires[Self::WIRE_ADDEND];
        let output = vars.local_wires[Self::WIRE_OUTPUT];

        let product_term = builder.mul_many_extension(&[const_0, multiplicand_0, multiplicand_1]);
        let addend_term = builder.mul_extension(const_1, addend);
        let computed_output = builder.add_many_extension(&[product_term, addend_term]);
        vec![builder.sub_extension(computed_output, output)]
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
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

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::gates::arithmetic::ArithmeticGate;
    use crate::gates::gate_testing::test_low_degree;

    #[test]
    fn low_degree() {
        test_low_degree(ArithmeticGate::new::<CrandallField, 4>())
    }
}
