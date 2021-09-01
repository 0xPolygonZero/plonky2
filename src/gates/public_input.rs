use std::ops::Range;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::Field64;
use crate::gates::gate::Gate;
use crate::iop::generator::WitnessGenerator;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// A gate whose first four wires will be equal to a hash of public inputs.
pub struct PublicInputGate;

impl PublicInputGate {
    pub fn wires_public_inputs_hash() -> Range<usize> {
        0..4
    }
}

impl<F: Field64 + Extendable<D>, const D: usize> Gate<F, D> for PublicInputGate {
    fn id(&self) -> String {
        "PublicInputGate".into()
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        Self::wires_public_inputs_hash()
            .zip(vars.public_inputs_hash.elements)
            .map(|(wire, hash_part)| vars.local_wires[wire] - hash_part.into())
            .collect()
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        Self::wires_public_inputs_hash()
            .zip(vars.public_inputs_hash.elements)
            .map(|(wire, hash_part)| vars.local_wires[wire] - hash_part)
            .collect()
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        Self::wires_public_inputs_hash()
            .zip(vars.public_inputs_hash.elements)
            .map(|(wire, hash_part)| {
                let hash_part_ext = builder.convert_to_ext(hash_part);
                builder.sub_extension(vars.local_wires[wire], hash_part_ext)
            })
            .collect()
    }

    fn generators(
        &self,
        _gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        Vec::new()
    }

    fn num_wires(&self) -> usize {
        4
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1
    }

    fn num_constraints(&self) -> usize {
        4
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::public_input::PublicInputGate;

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(PublicInputGate)
    }

    #[test]
    fn eval_fns() -> anyhow::Result<()> {
        test_eval_fns::<CrandallField, _, 4>(PublicInputGate)
    }
}
