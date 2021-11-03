use std::ops::Range;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::extension_field::FieldExtension;
use crate::field::field_types::RichField;
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CircuitConfig;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate which can a linear combination `c0*x*y+c1*z` twice with the same `x`.
#[derive(Debug)]
pub struct ArithmeticExtensionGate<const D: usize> {
    /// Number of arithmetic operations performed by an arithmetic gate.
    pub num_ops: usize,
}

impl<const D: usize> ArithmeticExtensionGate<D> {
    pub fn new_from_config(config: &CircuitConfig) -> Self {
        Self {
            num_ops: Self::num_ops(config),
        }
    }

    /// Determine the maximum number of operations that can fit in one gate for the given config.
    pub(crate) fn num_ops(config: &CircuitConfig) -> usize {
        let wires_per_op = 4 * D;
        config.num_routed_wires / wires_per_op
    }

    pub fn wires_ith_multiplicand_0(i: usize) -> Range<usize> {
        4 * D * i..4 * D * i + D
    }
    pub fn wires_ith_multiplicand_1(i: usize) -> Range<usize> {
        4 * D * i + D..4 * D * i + 2 * D
    }
    pub fn wires_ith_addend(i: usize) -> Range<usize> {
        4 * D * i + 2 * D..4 * D * i + 3 * D
    }
    pub fn wires_ith_output(i: usize) -> Range<usize> {
        4 * D * i + 3 * D..4 * D * i + 4 * D
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for ArithmeticExtensionGate<D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let mut constraints = Vec::new();
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.get_local_ext_algebra(Self::wires_ith_multiplicand_0(i));
            let multiplicand_1 = vars.get_local_ext_algebra(Self::wires_ith_multiplicand_1(i));
            let addend = vars.get_local_ext_algebra(Self::wires_ith_addend(i));
            let output = vars.get_local_ext_algebra(Self::wires_ith_output(i));
            let computed_output =
                (multiplicand_0 * multiplicand_1).scalar_mul(const_0) + addend.scalar_mul(const_1);

            constraints.extend((output - computed_output).to_basefield_array());
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let mut constraints = Vec::new();
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.get_local_ext(Self::wires_ith_multiplicand_0(i));
            let multiplicand_1 = vars.get_local_ext(Self::wires_ith_multiplicand_1(i));
            let addend = vars.get_local_ext(Self::wires_ith_addend(i));
            let output = vars.get_local_ext(Self::wires_ith_output(i));
            let computed_output =
                (multiplicand_0 * multiplicand_1).scalar_mul(const_0) + addend.scalar_mul(const_1);

            constraints.extend((output - computed_output).to_basefield_array());
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
            let multiplicand_0 = vars.get_local_ext_algebra(Self::wires_ith_multiplicand_0(i));
            let multiplicand_1 = vars.get_local_ext_algebra(Self::wires_ith_multiplicand_1(i));
            let addend = vars.get_local_ext_algebra(Self::wires_ith_addend(i));
            let output = vars.get_local_ext_algebra(Self::wires_ith_output(i));
            let computed_output = {
                let mul = builder.mul_ext_algebra(multiplicand_0, multiplicand_1);
                let scaled_mul = builder.scalar_mul_ext_algebra(const_0, mul);
                let scaled_addend = builder.scalar_mul_ext_algebra(const_1, addend);
                builder.add_ext_algebra(scaled_mul, scaled_addend)
            };

            let diff = builder.sub_ext_algebra(output, computed_output);
            constraints.extend(diff.to_ext_target_array());
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
                    ArithmeticExtensionGenerator {
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
        self.num_ops * 4 * D
    }

    fn num_constants(&self) -> usize {
        2
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        self.num_ops * D
    }
}

#[derive(Clone, Debug)]
struct ArithmeticExtensionGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    const_0: F,
    const_1: F,
    i: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for ArithmeticExtensionGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        ArithmeticExtensionGate::<D>::wires_ith_multiplicand_0(self.i)
            .chain(ArithmeticExtensionGate::<D>::wires_ith_multiplicand_1(
                self.i,
            ))
            .chain(ArithmeticExtensionGate::<D>::wires_ith_addend(self.i))
            .map(|i| Target::wire(self.gate_index, i))
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let extract_extension = |range: Range<usize>| -> F::Extension {
            let t = ExtensionTarget::from_range(self.gate_index, range);
            witness.get_extension_target(t)
        };

        let multiplicand_0 = extract_extension(
            ArithmeticExtensionGate::<D>::wires_ith_multiplicand_0(self.i),
        );
        let multiplicand_1 = extract_extension(
            ArithmeticExtensionGate::<D>::wires_ith_multiplicand_1(self.i),
        );
        let addend = extract_extension(ArithmeticExtensionGate::<D>::wires_ith_addend(self.i));

        let output_target = ExtensionTarget::from_range(
            self.gate_index,
            ArithmeticExtensionGate::<D>::wires_ith_output(self.i),
        );

        let computed_output = (multiplicand_0 * multiplicand_1).scalar_mul(self.const_0)
            + addend.scalar_mul(self.const_1);

        out_buffer.set_extension_target(output_target, computed_output)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gates::arithmetic::ArithmeticExtensionGate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::plonk::circuit_data::CircuitConfig;

    #[test]
    fn low_degree() {
        let gate =
            ArithmeticExtensionGate::new_from_config(&CircuitConfig::standard_recursion_config());
        test_low_degree::<GoldilocksField, _, 4>(gate);
    }

    #[test]
    fn eval_fns() -> Result<()> {
        let gate =
            ArithmeticExtensionGate::new_from_config(&CircuitConfig::standard_recursion_config());
        test_eval_fns::<GoldilocksField, _, 4>(gate)
    }
}
