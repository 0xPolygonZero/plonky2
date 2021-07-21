use std::ops::Range;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::witness::PartialWitness;

/// A gate which can a linear combination `c0*x*y+c1*z` twice with the same `x`.
#[derive(Debug)]
pub struct ArithmeticExtensionGate<const D: usize>;

impl<const D: usize> ArithmeticExtensionGate<D> {
    pub fn new<F: Extendable<D>>() -> GateRef<F, D> {
        GateRef::new(ArithmeticExtensionGate)
    }

    pub fn wires_first_multiplicand_0() -> Range<usize> {
        0..D
    }
    pub fn wires_first_multiplicand_1() -> Range<usize> {
        D..2 * D
    }
    pub fn wires_first_addend() -> Range<usize> {
        2 * D..3 * D
    }
    pub fn wires_second_multiplicand_0() -> Range<usize> {
        3 * D..4 * D
    }
    pub fn wires_second_multiplicand_1() -> Range<usize> {
        4 * D..5 * D
    }
    pub fn wires_second_addend() -> Range<usize> {
        5 * D..6 * D
    }
    pub fn wires_first_output() -> Range<usize> {
        6 * D..7 * D
    }
    pub fn wires_second_output() -> Range<usize> {
        7 * D..8 * D
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for ArithmeticExtensionGate<D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let first_multiplicand_0 = vars.get_local_ext_algebra(Self::wires_first_multiplicand_0());
        let first_multiplicand_1 = vars.get_local_ext_algebra(Self::wires_first_multiplicand_1());
        let first_addend = vars.get_local_ext_algebra(Self::wires_first_addend());
        let second_multiplicand_0 = vars.get_local_ext_algebra(Self::wires_second_multiplicand_0());
        let second_multiplicand_1 = vars.get_local_ext_algebra(Self::wires_second_multiplicand_1());
        let second_addend = vars.get_local_ext_algebra(Self::wires_second_addend());
        let first_output = vars.get_local_ext_algebra(Self::wires_first_output());
        let second_output = vars.get_local_ext_algebra(Self::wires_second_output());

        let first_computed_output = first_multiplicand_0 * first_multiplicand_1 * const_0.into()
            + first_addend * const_1.into();
        let second_computed_output = second_multiplicand_0 * second_multiplicand_1 * const_0.into()
            + second_addend * const_1.into();

        let mut constraints = (first_output - first_computed_output)
            .to_basefield_array()
            .to_vec();
        constraints.extend((second_output - second_computed_output).to_basefield_array());
        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let first_multiplicand_0 = vars.get_local_ext_algebra(Self::wires_first_multiplicand_0());
        let first_multiplicand_1 = vars.get_local_ext_algebra(Self::wires_first_multiplicand_1());
        let first_addend = vars.get_local_ext_algebra(Self::wires_first_addend());
        let second_multiplicand_0 = vars.get_local_ext_algebra(Self::wires_second_multiplicand_0());
        let second_multiplicand_1 = vars.get_local_ext_algebra(Self::wires_second_multiplicand_1());
        let second_addend = vars.get_local_ext_algebra(Self::wires_second_addend());
        let first_output = vars.get_local_ext_algebra(Self::wires_first_output());
        let second_output = vars.get_local_ext_algebra(Self::wires_second_output());

        let first_computed_output =
            builder.mul_ext_algebra(first_multiplicand_0, first_multiplicand_1);
        let first_computed_output = builder.scalar_mul_ext_algebra(const_0, first_computed_output);
        let first_scaled_addend = builder.scalar_mul_ext_algebra(const_1, first_addend);
        let first_computed_output =
            builder.add_ext_algebra(first_computed_output, first_scaled_addend);

        let second_computed_output =
            builder.mul_ext_algebra(second_multiplicand_0, second_multiplicand_1);
        let second_computed_output =
            builder.scalar_mul_ext_algebra(const_0, second_computed_output);
        let second_scaled_addend = builder.scalar_mul_ext_algebra(const_1, second_addend);
        let second_computed_output =
            builder.add_ext_algebra(second_computed_output, second_scaled_addend);

        let diff_0 = builder.sub_ext_algebra(first_output, first_computed_output);
        let diff_1 = builder.sub_ext_algebra(second_output, second_computed_output);
        let mut constraints = diff_0.to_ext_target_array().to_vec();
        constraints.extend(diff_1.to_ext_target_array());
        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen0 = ArithmeticExtensionGenerator0 {
            gate_index,
            const_0: local_constants[0],
            const_1: local_constants[1],
        };
        let gen1 = ArithmeticExtensionGenerator1 {
            gate_index,
            const_0: local_constants[0],
            const_1: local_constants[1],
        };
        vec![Box::new(gen0), Box::new(gen1)]
    }

    fn num_wires(&self) -> usize {
        8 * D
    }

    fn num_constants(&self) -> usize {
        2
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        2 * D
    }
}

struct ArithmeticExtensionGenerator0<F: Extendable<D>, const D: usize> {
    gate_index: usize,
    const_0: F,
    const_1: F,
}

struct ArithmeticExtensionGenerator1<F: Extendable<D>, const D: usize> {
    gate_index: usize,
    const_0: F,
    const_1: F,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for ArithmeticExtensionGenerator0<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        ArithmeticExtensionGate::<D>::wires_first_multiplicand_0()
            .chain(ArithmeticExtensionGate::<D>::wires_first_multiplicand_1())
            .chain(ArithmeticExtensionGate::<D>::wires_first_addend())
            .map(|i| Target::wire(self.gate_index, i))
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let extract_extension = |range: Range<usize>| -> F::Extension {
            let t = ExtensionTarget::from_range(self.gate_index, range);
            witness.get_extension_target(t)
        };

        let multiplicand_0 =
            extract_extension(ArithmeticExtensionGate::<D>::wires_first_multiplicand_0());
        let multiplicand_1 =
            extract_extension(ArithmeticExtensionGate::<D>::wires_first_multiplicand_1());
        let addend = extract_extension(ArithmeticExtensionGate::<D>::wires_first_addend());

        let output_target = ExtensionTarget::from_range(
            self.gate_index,
            ArithmeticExtensionGate::<D>::wires_first_output(),
        );

        let computed_output =
            multiplicand_0 * multiplicand_1 * self.const_0.into() + addend * self.const_1.into();

        PartialWitness::singleton_extension_target(output_target, computed_output)
    }
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for ArithmeticExtensionGenerator1<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        ArithmeticExtensionGate::<D>::wires_second_multiplicand_0()
            .chain(ArithmeticExtensionGate::<D>::wires_second_multiplicand_1())
            .chain(ArithmeticExtensionGate::<D>::wires_second_addend())
            .map(|i| Target::wire(self.gate_index, i))
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let extract_extension = |range: Range<usize>| -> F::Extension {
            let t = ExtensionTarget::from_range(self.gate_index, range);
            witness.get_extension_target(t)
        };

        let multiplicand_0 =
            extract_extension(ArithmeticExtensionGate::<D>::wires_second_multiplicand_0());
        let multiplicand_1 =
            extract_extension(ArithmeticExtensionGate::<D>::wires_second_multiplicand_1());
        let addend = extract_extension(ArithmeticExtensionGate::<D>::wires_second_addend());

        let output_target = ExtensionTarget::from_range(
            self.gate_index,
            ArithmeticExtensionGate::<D>::wires_second_output(),
        );

        let computed_output =
            multiplicand_0 * multiplicand_1 * self.const_0.into() + addend * self.const_1.into();

        PartialWitness::singleton_extension_target(output_target, computed_output)
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::gates::arithmetic::ArithmeticExtensionGate;
    use crate::gates::gate_testing::test_low_degree;

    #[test]
    fn low_degree() {
        test_low_degree(ArithmeticExtensionGate::<4>::new::<CrandallField>())
    }
}
