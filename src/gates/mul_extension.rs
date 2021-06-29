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

/// A gate which can multiply two field extension elements.
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
        let multiplicand_0 = vars.get_local_ext_algebra(Self::wires_multiplicand_0());
        let multiplicand_1 = vars.get_local_ext_algebra(Self::wires_multiplicand_1());
        let output = vars.get_local_ext_algebra(Self::wires_output());
        let computed_output = multiplicand_0 * multiplicand_1 * const_0.into();
        (output - computed_output).to_basefield_array().to_vec()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let const_0 = vars.local_constants[0];
        let multiplicand_0 = vars.get_local_ext_algebra(Self::wires_multiplicand_0());
        let multiplicand_1 = vars.get_local_ext_algebra(Self::wires_multiplicand_1());
        let output = vars.get_local_ext_algebra(Self::wires_output());
        let computed_output = builder.mul_ext_algebra(multiplicand_0, multiplicand_1);
        let computed_output = builder.scalar_mul_ext_algebra(const_0, computed_output);
        let diff = builder.sub_ext_algebra(output, computed_output);
        diff.to_ext_target_array().to_vec()
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = MulExtensionGenerator {
            gate_index,
            const_0: local_constants[0],
        };
        vec![Box::new(gen)]
    }

    fn num_wires(&self) -> usize {
        12
    }

    fn num_constants(&self) -> usize {
        1
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        D
    }
}

struct MulExtensionGenerator<F: Extendable<D>, const D: usize> {
    gate_index: usize,
    const_0: F,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for MulExtensionGenerator<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        MulExtensionGate::<D>::wires_multiplicand_0()
            .chain(MulExtensionGate::<D>::wires_multiplicand_1())
            .map(|i| {
                Target::Wire(Wire {
                    gate: self.gate_index,
                    input: i,
                })
            })
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let multiplicand_0_target = ExtensionTarget::from_range(
            self.gate_index,
            MulExtensionGate::<D>::wires_multiplicand_0(),
        );
        let multiplicand_0 = witness.get_extension_target(multiplicand_0_target);

        let multiplicand_1_target = ExtensionTarget::from_range(
            self.gate_index,
            MulExtensionGate::<D>::wires_multiplicand_1(),
        );
        let multiplicand_1 = witness.get_extension_target(multiplicand_1_target);

        let output_target =
            ExtensionTarget::from_range(self.gate_index, MulExtensionGate::<D>::wires_output());

        let computed_output =
            F::Extension::from_basefield(self.const_0) * multiplicand_0 * multiplicand_1;

        let mut pw = PartialWitness::new();
        pw.set_extension_target(output_target, computed_output);
        pw
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::gates::gate_testing::test_low_degree;
    use crate::gates::mul_extension::MulExtensionGate;

    #[test]
    fn low_degree() {
        test_low_degree(MulExtensionGate::<4>::new::<CrandallField>())
    }
}
