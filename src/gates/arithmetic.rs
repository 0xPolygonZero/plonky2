use std::ops::Range;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::extension_field::FieldExtension;
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::witness::PartialWitness;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate which can a linear combination `c0*x*y+c1*z` twice with the same `x`.
#[derive(Debug)]
pub struct ArithmeticExtensionGate<const D: usize>;

impl<const D: usize> ArithmeticExtensionGate<D> {
    pub fn wires_first_multiplicand_0() -> Range<usize> {
        0..D
    }
    pub fn wires_first_multiplicand_1() -> Range<usize> {
        D..2 * D
    }
    pub fn wires_first_addend() -> Range<usize> {
        2 * D..3 * D
    }
    pub fn wires_first_output() -> Range<usize> {
        3 * D..4 * D
    }

    pub fn wires_second_multiplicand_0() -> Range<usize> {
        4 * D..5 * D
    }
    pub fn wires_second_multiplicand_1() -> Range<usize> {
        5 * D..6 * D
    }
    pub fn wires_second_addend() -> Range<usize> {
        6 * D..7 * D
    }
    pub fn wires_second_output() -> Range<usize> {
        7 * D..8 * D
    }

    pub fn wires_third_multiplicand_0() -> Range<usize> {
        8 * D..9 * D
    }
    pub fn wires_third_multiplicand_1() -> Range<usize> {
        9 * D..10 * D
    }
    pub fn wires_third_addend() -> Range<usize> {
        10 * D..11 * D
    }
    pub fn wires_third_output() -> Range<usize> {
        11 * D..12 * D
    }

    pub fn wires_fourth_multiplicand_0() -> Range<usize> {
        12 * D..13 * D
    }
    pub fn wires_fourth_multiplicand_1() -> Range<usize> {
        13 * D..14 * D
    }
    pub fn wires_fourth_addend() -> Range<usize> {
        14 * D..15 * D
    }
    pub fn wires_fourth_output() -> Range<usize> {
        15 * D..16 * D
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
        let third_multiplicand_0 = vars.get_local_ext_algebra(Self::wires_third_multiplicand_0());
        let third_multiplicand_1 = vars.get_local_ext_algebra(Self::wires_third_multiplicand_1());
        let third_addend = vars.get_local_ext_algebra(Self::wires_third_addend());
        let fourth_multiplicand_0 = vars.get_local_ext_algebra(Self::wires_fourth_multiplicand_0());
        let fourth_multiplicand_1 = vars.get_local_ext_algebra(Self::wires_fourth_multiplicand_1());
        let fourth_addend = vars.get_local_ext_algebra(Self::wires_fourth_addend());
        let first_output = vars.get_local_ext_algebra(Self::wires_first_output());
        let second_output = vars.get_local_ext_algebra(Self::wires_second_output());
        let third_output = vars.get_local_ext_algebra(Self::wires_third_output());
        let fourth_output = vars.get_local_ext_algebra(Self::wires_fourth_output());

        let first_computed_output = first_multiplicand_0 * first_multiplicand_1 * const_0.into()
            + first_addend * const_1.into();
        let second_computed_output = second_multiplicand_0 * second_multiplicand_1 * const_0.into()
            + second_addend * const_1.into();
        let third_computed_output = third_multiplicand_0 * third_multiplicand_1 * const_0.into()
            + third_addend * const_1.into();
        let fourth_computed_output = fourth_multiplicand_0 * fourth_multiplicand_1 * const_0.into()
            + fourth_addend * const_1.into();

        let mut constraints = (first_output - first_computed_output)
            .to_basefield_array()
            .to_vec();
        constraints.extend((second_output - second_computed_output).to_basefield_array());
        constraints.extend((third_output - third_computed_output).to_basefield_array());
        constraints.extend((fourth_output - fourth_computed_output).to_basefield_array());
        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let const_0 = vars.local_constants[0];
        let const_1 = vars.local_constants[1];

        let first_multiplicand_0 = vars.get_local_ext(Self::wires_first_multiplicand_0());
        let first_multiplicand_1 = vars.get_local_ext(Self::wires_first_multiplicand_1());
        let first_addend = vars.get_local_ext(Self::wires_first_addend());
        let second_multiplicand_0 = vars.get_local_ext(Self::wires_second_multiplicand_0());
        let second_multiplicand_1 = vars.get_local_ext(Self::wires_second_multiplicand_1());
        let second_addend = vars.get_local_ext(Self::wires_second_addend());
        let third_multiplicand_0 = vars.get_local_ext(Self::wires_third_multiplicand_0());
        let third_multiplicand_1 = vars.get_local_ext(Self::wires_third_multiplicand_1());
        let third_addend = vars.get_local_ext(Self::wires_third_addend());
        let fourth_multiplicand_0 = vars.get_local_ext(Self::wires_fourth_multiplicand_0());
        let fourth_multiplicand_1 = vars.get_local_ext(Self::wires_fourth_multiplicand_1());
        let fourth_addend = vars.get_local_ext(Self::wires_fourth_addend());
        let first_output = vars.get_local_ext(Self::wires_first_output());
        let second_output = vars.get_local_ext(Self::wires_second_output());
        let third_output = vars.get_local_ext(Self::wires_third_output());
        let fourth_output = vars.get_local_ext(Self::wires_fourth_output());

        let first_computed_output = first_multiplicand_0 * first_multiplicand_1 * const_0.into()
            + first_addend * const_1.into();
        let second_computed_output = second_multiplicand_0 * second_multiplicand_1 * const_0.into()
            + second_addend * const_1.into();
        let third_computed_output = third_multiplicand_0 * third_multiplicand_1 * const_0.into()
            + third_addend * const_1.into();
        let fourth_computed_output = fourth_multiplicand_0 * fourth_multiplicand_1 * const_0.into()
            + fourth_addend * const_1.into();

        let mut constraints = (first_output - first_computed_output)
            .to_basefield_array()
            .to_vec();
        constraints.extend((second_output - second_computed_output).to_basefield_array());
        constraints.extend((third_output - third_computed_output).to_basefield_array());
        constraints.extend((fourth_output - fourth_computed_output).to_basefield_array());
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
        let third_multiplicand_0 = vars.get_local_ext_algebra(Self::wires_third_multiplicand_0());
        let third_multiplicand_1 = vars.get_local_ext_algebra(Self::wires_third_multiplicand_1());
        let third_addend = vars.get_local_ext_algebra(Self::wires_third_addend());
        let fourth_multiplicand_0 = vars.get_local_ext_algebra(Self::wires_fourth_multiplicand_0());
        let fourth_multiplicand_1 = vars.get_local_ext_algebra(Self::wires_fourth_multiplicand_1());
        let fourth_addend = vars.get_local_ext_algebra(Self::wires_fourth_addend());
        let first_output = vars.get_local_ext_algebra(Self::wires_first_output());
        let second_output = vars.get_local_ext_algebra(Self::wires_second_output());
        let third_output = vars.get_local_ext_algebra(Self::wires_third_output());
        let fourth_output = vars.get_local_ext_algebra(Self::wires_fourth_output());

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

        let third_computed_output =
            builder.mul_ext_algebra(third_multiplicand_0, third_multiplicand_1);
        let third_computed_output = builder.scalar_mul_ext_algebra(const_0, third_computed_output);
        let third_scaled_addend = builder.scalar_mul_ext_algebra(const_1, third_addend);
        let third_computed_output =
            builder.add_ext_algebra(third_computed_output, third_scaled_addend);

        let fourth_computed_output =
            builder.mul_ext_algebra(fourth_multiplicand_0, fourth_multiplicand_1);
        let fourth_computed_output =
            builder.scalar_mul_ext_algebra(const_0, fourth_computed_output);
        let fourth_scaled_addend = builder.scalar_mul_ext_algebra(const_1, fourth_addend);
        let fourth_computed_output =
            builder.add_ext_algebra(fourth_computed_output, fourth_scaled_addend);

        let diff_0 = builder.sub_ext_algebra(first_output, first_computed_output);
        let diff_1 = builder.sub_ext_algebra(second_output, second_computed_output);
        let diff_2 = builder.sub_ext_algebra(third_output, third_computed_output);
        let diff_3 = builder.sub_ext_algebra(fourth_output, fourth_computed_output);
        let mut constraints = diff_0.to_ext_target_array().to_vec();
        constraints.extend(diff_1.to_ext_target_array());
        constraints.extend(diff_2.to_ext_target_array());
        constraints.extend(diff_3.to_ext_target_array());
        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gens = (0..4)
            .map(|i| ArithmeticExtensionGenerator {
                gate_index,
                const_0: local_constants[0],
                const_1: local_constants[1],
                i,
            })
            .collect::<Vec<_>>();
        vec![
            Box::new(gens[0].clone()),
            Box::new(gens[1].clone()),
            Box::new(gens[2].clone()),
            Box::new(gens[3].clone()),
        ]
    }

    fn num_wires(&self) -> usize {
        16 * D
    }

    fn num_constants(&self) -> usize {
        2
    }

    fn degree(&self) -> usize {
        3
    }

    fn num_constraints(&self) -> usize {
        4 * D
    }
}

#[derive(Clone)]
struct ArithmeticExtensionGenerator<F: Extendable<D>, const D: usize> {
    gate_index: usize,
    const_0: F,
    const_1: F,
    i: usize,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for ArithmeticExtensionGenerator<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        (4 * self.i * D..(4 * self.i + 3) * D)
            .map(|i| Target::wire(self.gate_index, i))
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let extract_extension = |range: Range<usize>| -> F::Extension {
            let t = ExtensionTarget::from_range(self.gate_index, range);
            witness.get_extension_target(t)
        };

        let start = 4 * self.i * D;
        let multiplicand_0 = extract_extension(start..start + D);
        let multiplicand_1 = extract_extension(start + D..start + 2 * D);
        let addend = extract_extension(start + 2 * D..start + 3 * D);

        let output_target =
            ExtensionTarget::from_range(self.gate_index, start + 3 * D..start + 4 * D);

        let computed_output =
            multiplicand_0 * multiplicand_1 * self.const_0.into() + addend * self.const_1.into();

        out_buffer.set_extension_target(output_target, computed_output)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;
    use crate::gates::arithmetic::ArithmeticExtensionGate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(ArithmeticExtensionGate)
    }
    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<CrandallField, _, 4>(ArithmeticExtensionGate)
    }
}
