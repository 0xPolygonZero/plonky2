use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CircuitConfig;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate which can perform a weighted multiply-add, i.e. `result = c0 x y + c1 z`. If the config
/// supports enough routed wires, it can support several such operations in one gate.
#[derive(Debug)]
pub struct ArithmeticGate {
    /// Number of arithmetic operations performed by an arithmetic gate.
    pub num_ops: usize,
}

impl ArithmeticGate {
    pub fn new_from_config(config: &CircuitConfig) -> Self {
        Self {
            num_ops: Self::num_ops(config),
        }
    }

    /// Determine the maximum number of operations that can fit in one gate for the given config.
    pub(crate) fn num_ops(config: &CircuitConfig) -> usize {
        let wires_per_op = 4;
        config.num_routed_wires / wires_per_op
    }

    pub fn wire_ith_multiplicand_0(i: usize) -> usize {
        4 * i
    }
    pub fn wire_ith_multiplicand_1(i: usize) -> usize {
        4 * i + 1
    }
    pub fn wire_ith_addend(i: usize) -> usize {
        4 * i + 2
    }
    pub fn wire_ith_output(i: usize) -> usize {
        4 * i + 3
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for ArithmeticGate {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let mut constraints = Vec::new();
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];
            let output = vars.local_wires[Self::wire_ith_output(i)];
            let computed_output = multiplicand_0 * multiplicand_1 * const_0 + addend * const_1;

            constraints.push(output - computed_output);
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let mut constraints = Vec::new();
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];
            let output = vars.local_wires[Self::wire_ith_output(i)];
            let computed_output = multiplicand_0 * multiplicand_1 * const_0 + addend * const_1;

            constraints.push(output - computed_output);
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let mut constraints = Vec::new();
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];
            let output = vars.local_wires[Self::wire_ith_output(i)];
            let computed_output = {
                let scaled_mul =
                    builder.mul_many_extension(&[const_0, multiplicand_0, multiplicand_1]);
                builder.mul_add_extension(const_1, addend, scaled_mul)
            };

            let diff = builder.sub_extension(output, computed_output);
            constraints.push(diff);
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..self.num_ops)
            .map(|i| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    ArithmeticBaseGenerator {
                        gate_index,
                        const_0: local_constants[0],
                        const_1: local_constants[1],
                        i,
                    }
                    .adapter(),
                );
                g
            })
            .collect::<Vec<_>>()
    }

    fn num_wires(&self) -> usize {
        self.num_ops * 4
    }

    fn num_constants(&self) -> usize {
        2
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        self.num_ops
    }
}

#[derive(Clone, Debug)]
struct ArithmeticBaseGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    const_0: F,
    const_1: F,
    i: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for ArithmeticBaseGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        [
            ArithmeticGate::wire_ith_multiplicand_0(self.i),
            ArithmeticGate::wire_ith_multiplicand_1(self.i),
            ArithmeticGate::wire_ith_addend(self.i),
        ]
        .iter()
        .map(|&i| Target::wire(self.gate_index, i))
        .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let get_wire =
            |wire: usize| -> F { witness.get_target(Target::wire(self.gate_index, wire)) };

        let multiplicand_0 = get_wire(ArithmeticGate::wire_ith_multiplicand_0(self.i));
        let multiplicand_1 = get_wire(ArithmeticGate::wire_ith_multiplicand_1(self.i));
        let addend = get_wire(ArithmeticGate::wire_ith_addend(self.i));

        let output_target = Target::wire(self.gate_index, ArithmeticGate::wire_ith_output(self.i));

        let computed_output =
            multiplicand_0 * multiplicand_1 * self.const_0 + addend * self.const_1;

        out_buffer.set_target(output_target, computed_output)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gates::arithmetic_base::ArithmeticGate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::plonk::circuit_data::CircuitConfig;

    #[test]
    fn low_degree() {
        let gate = ArithmeticGate::new_from_config(&CircuitConfig::standard_recursion_config());
        test_low_degree::<GoldilocksField, _, 4>(gate);
    }

    #[test]
    fn eval_fns() -> Result<()> {
        let gate = ArithmeticGate::new_from_config(&CircuitConfig::standard_recursion_config());
        test_eval_fns::<GoldilocksField, _, 4>(gate)
    }
}
