use std::ops::Range;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::extension_field::FieldExtension;
use crate::gates::gate::Gate;
use crate::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use crate::witness::PartialWitness;

/// Computes `sum alpha^i c_i` for a vector `c_i` of `num_coeffs` elements of the base field.
#[derive(Debug, Clone)]
pub struct ReducingGate<const D: usize> {
    pub num_coeffs: usize,
}

impl<const D: usize> ReducingGate<D> {
    pub fn new(num_coeffs: usize) -> Self {
        Self { num_coeffs }
    }

    pub fn max_coeffs_len(num_wires: usize, num_routed_wires: usize) -> usize {
        (num_routed_wires - 3 * D).min((num_wires - 3 * D) / (D + 1))
    }

    pub fn wires_output() -> Range<usize> {
        0..D
    }
    pub fn wires_alpha() -> Range<usize> {
        D..2 * D
    }
    pub fn wires_old_acc() -> Range<usize> {
        2 * D..3 * D
    }
    const START_COEFFS: usize = 3 * D;
    pub fn wires_coeffs(&self) -> Range<usize> {
        Self::START_COEFFS..Self::START_COEFFS + self.num_coeffs
    }
    fn start_accs(&self) -> usize {
        Self::START_COEFFS + self.num_coeffs
    }
    fn wires_accs(&self, i: usize) -> Range<usize> {
        if i == self.num_coeffs - 1 {
            // The last accumulator is the output.
            return Self::wires_output();
        }
        self.start_accs() + D * i..self.start_accs() + D * (i + 1)
    }
}

impl<F: Extendable<D>, const D: usize> Gate<F, D> for ReducingGate<D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let output = vars.get_local_ext_algebra(Self::wires_output());
        let alpha = vars.get_local_ext_algebra(Self::wires_alpha());
        let old_acc = vars.get_local_ext_algebra(Self::wires_old_acc());
        let coeffs = self
            .wires_coeffs()
            .map(|i| vars.local_wires[i])
            .collect::<Vec<_>>();
        let accs = (0..self.num_coeffs)
            .map(|i| vars.get_local_ext_algebra(self.wires_accs(i)))
            .collect::<Vec<_>>();

        let mut constraints = Vec::new();
        let mut acc = old_acc;
        for i in 0..self.num_coeffs {
            constraints.push(acc * alpha + coeffs[i].into() - accs[i]);
            acc = accs[i];
        }

        constraints
            .into_iter()
            .flat_map(|alg| alg.to_basefield_array())
            .collect()
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let output = vars.get_local_ext(Self::wires_output());
        let alpha = vars.get_local_ext(Self::wires_alpha());
        let old_acc = vars.get_local_ext(Self::wires_old_acc());
        let coeffs = self
            .wires_coeffs()
            .map(|i| vars.local_wires[i])
            .collect::<Vec<_>>();
        let accs = (0..self.num_coeffs)
            .map(|i| vars.get_local_ext(self.wires_accs(i)))
            .collect::<Vec<_>>();

        let mut constraints = Vec::new();
        let mut acc = old_acc;
        for i in 0..self.num_coeffs {
            constraints.push(acc * alpha + coeffs[i].into() - accs[i]);
            acc = accs[i];
        }

        constraints
            .into_iter()
            .flat_map(|alg| alg.to_basefield_array())
            .collect()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let output = vars.get_local_ext_algebra(Self::wires_output());
        let alpha = vars.get_local_ext_algebra(Self::wires_alpha());
        let old_acc = vars.get_local_ext_algebra(Self::wires_old_acc());
        let coeffs = self
            .wires_coeffs()
            .map(|i| vars.local_wires[i])
            .collect::<Vec<_>>();
        let accs = (0..self.num_coeffs)
            .map(|i| vars.get_local_ext_algebra(self.wires_accs(i)))
            .collect::<Vec<_>>();

        let mut constraints = Vec::new();
        let mut acc = old_acc;
        for i in 0..self.num_coeffs {
            let mut tmp = builder.mul_ext_algebra(acc, alpha);
            let coeff = builder.convert_to_ext_algebra(coeffs[i]);
            tmp = builder.add_ext_algebra(tmp, coeff);
            tmp = builder.sub_ext_algebra(tmp, accs[i]);
            constraints.push(tmp);
            acc = accs[i];
        }

        constraints
            .into_iter()
            .flat_map(|alg| alg.to_ext_target_array())
            .collect()
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        vec![Box::new(ReducingGenerator {
            gate_index,
            gate: self.clone(),
        })]
    }

    fn num_wires(&self) -> usize {
        3 * D + self.num_coeffs * (D + 1)
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        D * (self.num_coeffs + 1)
    }
}

struct ReducingGenerator<const D: usize> {
    gate_index: usize,
    gate: ReducingGate<D>,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for ReducingGenerator<D> {
    fn dependencies(&self) -> Vec<Target> {
        ReducingGate::<D>::wires_alpha()
            .chain(ReducingGate::<D>::wires_old_acc())
            .chain(self.gate.wires_coeffs())
            .map(|i| Target::wire(self.gate_index, i))
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> GeneratedValues<F> {
        let extract_extension = |range: Range<usize>| -> F::Extension {
            let t = ExtensionTarget::from_range(self.gate_index, range);
            witness.get_extension_target(t)
        };

        let alpha = extract_extension(ReducingGate::<D>::wires_alpha());
        let old_acc = extract_extension(ReducingGate::<D>::wires_old_acc());
        let coeffs = witness.get_targets(
            &self
                .gate
                .wires_coeffs()
                .map(|i| Target::wire(self.gate_index, i))
                .collect::<Vec<_>>(),
        );
        let accs = (0..self.gate.num_coeffs)
            .map(|i| ExtensionTarget::from_range(self.gate_index, self.gate.wires_accs(i)))
            .collect::<Vec<_>>();
        let output =
            ExtensionTarget::from_range(self.gate_index, ReducingGate::<D>::wires_output());

        let mut result = GeneratedValues::<F>::with_capacity(self.gate.num_coeffs + 1);
        let mut acc = old_acc;
        for i in 0..self.gate.num_coeffs {
            let computed_acc = acc * alpha + coeffs[i].into();
            result.set_extension_target(accs[i], computed_acc);
            acc = computed_acc;
        }
        result.set_extension_target(output, acc);

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::gates::gate_testing::test_low_degree;
    use crate::gates::reducing::ReducingGate;

    #[test]
    fn low_degree() {
        type F = CrandallField;
        test_low_degree::<CrandallField, _, 4>(ReducingGate::new(22));
    }
}
