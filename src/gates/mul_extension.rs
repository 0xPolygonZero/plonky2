use std::ops::Range;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;

/// A gate which can a linear combination `c0*x*y+c1*z` twice with the same `x`.
#[derive(Debug)]
pub struct ArithmeticExtensionGate<const D: usize>;

impl<const D: usize> ArithmeticExtensionGate<D> {
    pub fn new<F: Extendable<D>>() -> GateRef<F, D> {
        GateRef::new(ArithmeticExtensionGate)
    }

    pub fn wires_fixed_multiplicand() -> Range<usize> {
        0..D
    }
    pub fn wires_multiplicand_0() -> Range<usize> {
        D..2 * D
    }
    pub fn wires_addend_0() -> Range<usize> {
        2 * D..3 * D
    }
    pub fn wires_multiplicand_1() -> Range<usize> {
        3 * D..4 * D
    }
    pub fn wires_addend_1() -> Range<usize> {
        4 * D..5 * D
    }
    pub fn wires_output_0() -> Range<usize> {
        5 * D..6 * D
    }
    pub fn wires_output_1() -> Range<usize> {
        6 * D..7 * D
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for ArithmeticExtensionGate<D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let fixed_multiplicand = vars.get_local_ext_algebra(Self::wires_fixed_multiplicand());
        let multiplicand_0 = vars.get_local_ext_algebra(Self::wires_multiplicand_0());
        let addend_0 = vars.get_local_ext_algebra(Self::wires_addend_0());
        let multiplicand_1 = vars.get_local_ext_algebra(Self::wires_multiplicand_1());
        let addend_1 = vars.get_local_ext_algebra(Self::wires_addend_1());
        let output_0 = vars.get_local_ext_algebra(Self::wires_output_0());
        let output_1 = vars.get_local_ext_algebra(Self::wires_output_1());

        let computed_output_0 =
            fixed_multiplicand * multiplicand_0 * const_0.into() + addend_0 * const_1.into();
        let computed_output_1 =
            fixed_multiplicand * multiplicand_1 * const_1.into() + addend_1 * const_1.into();

        let mut constraints = (output_0 - computed_output_0).to_basefield_array().to_vec();
        constraints.extend((output_1 - computed_output_1).to_basefield_array());
        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let fixed_multiplicand = vars.get_local_ext_algebra(Self::wires_fixed_multiplicand());
        let multiplicand_0 = vars.get_local_ext_algebra(Self::wires_multiplicand_0());
        let addend_0 = vars.get_local_ext_algebra(Self::wires_addend_0());
        let multiplicand_1 = vars.get_local_ext_algebra(Self::wires_multiplicand_1());
        let addend_1 = vars.get_local_ext_algebra(Self::wires_addend_1());
        let output_0 = vars.get_local_ext_algebra(Self::wires_output_0());
        let output_1 = vars.get_local_ext_algebra(Self::wires_output_1());

        let computed_output_0 = builder.mul_ext_algebra(fixed_multiplicand, multiplicand_0);
        let computed_output_0 = builder.scalar_mul_ext_algebra(const_0, computed_output_0);
        let scaled_addend_0 = builder.scalar_mul_ext_algebra(const_1, addend_0);
        let computed_output_0 = builder.add_ext_algebra(computed_output_0, scaled_addend_0);

        let computed_output_1 = builder.mul_ext_algebra(fixed_multiplicand, multiplicand_1);
        let computed_output_1 = builder.scalar_mul_ext_algebra(const_0, computed_output_1);
        let scaled_addend_1 = builder.scalar_mul_ext_algebra(const_1, addend_1);
        let computed_output_1 = builder.add_ext_algebra(computed_output_1, scaled_addend_1);

        let diff_0 = builder.sub_ext_algebra(output_0, computed_output_0);
        let diff_1 = builder.sub_ext_algebra(output_1, computed_output_1);
        let mut constraints = diff_0.to_ext_target_array().to_vec();
        constraints.extend(diff_1.to_ext_target_array());
        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = MulExtensionGenerator {
            gate_index,
            const_0: local_constants[0],
            const_1: local_constants[1],
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        28
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

struct MulExtensionGenerator<F: Extendable<D>, const D: usize> {
    gate_index: usize,
    const_0: F,
    const_1: F,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for MulExtensionGenerator<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        ArithmeticExtensionGate::<D>::wires_fixed_multiplicand()
            .chain(ArithmeticExtensionGate::<D>::wires_multiplicand_0())
            .chain(ArithmeticExtensionGate::<D>::wires_addend_0())
            .chain(ArithmeticExtensionGate::<D>::wires_multiplicand_1())
            .chain(ArithmeticExtensionGate::<D>::wires_addend_1())
            .map(|i| Target::wire(self.gate_index, i))
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let extract_extension = |range: Range<usize>| -> F::Extension {
            let t = ExtensionTarget::from_range(self.gate_index, range);
            witness.get_extension_target(t)
        };

        let fixed_multiplicand =
            extract_extension(ArithmeticExtensionGate::<D>::wires_fixed_multiplicand());
        let multiplicand_0 =
            extract_extension(ArithmeticExtensionGate::<D>::wires_multiplicand_0());
        let addend_0 = extract_extension(ArithmeticExtensionGate::<D>::wires_addend_0());
        let multiplicand_1 =
            extract_extension(ArithmeticExtensionGate::<D>::wires_multiplicand_1());
        let addend_1 = extract_extension(ArithmeticExtensionGate::<D>::wires_addend_1());

        let output_target_0 = ExtensionTarget::from_range(
            self.gate_index,
            ArithmeticExtensionGate::<D>::wires_output_0(),
        );
        let output_target_1 = ExtensionTarget::from_range(
            self.gate_index,
            ArithmeticExtensionGate::<D>::wires_output_1(),
        );

        let computed_output_0 = fixed_multiplicand * multiplicand_0 * self.const_0.into()
            + addend_0 * self.const_1.into();
        let computed_output_1 = fixed_multiplicand * multiplicand_1 * self.const_0.into()
            + addend_1 * self.const_1.into();

        let mut pw = PartialWitness::new();
        pw.set_extension_target(output_target_0, computed_output_0);
        pw.set_extension_target(output_target_1, computed_output_1);
        pw
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::gates::gate_testing::test_low_degree;
    use crate::gates::mul_extension::ArithmeticExtensionGate;

    #[test]
    fn low_degree() {
        test_low_degree(ArithmeticExtensionGate::<4>::new::<CrandallField>())
    }
}
