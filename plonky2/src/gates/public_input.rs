use std::ops::Range;
use std::sync::Arc;

use plonky2_field::extension_field::Extendable;
use plonky2_field::packed_field::PackedField;

use crate::gates::gate::{Gate, GateRef};
use crate::gates::packed_util::PackedEvaluableBase;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::{HashOutTarget, RichField};
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::operation::Operation;
use crate::iop::target::Target;
use crate::iop::witness::PartitionWitness;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};

/// A gate whose first four wires will be equal to a hash of public inputs.
#[derive(Copy, Clone, Debug)]
pub struct PublicInputGate;

impl PublicInputGate {
    pub fn wires_public_inputs_hash() -> Range<usize> {
        0..4
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for PublicInputGate {
    fn id(&self) -> String {
        "PublicInputGate".into()
    }

    fn add_operation(&self, targets: Vec<Target>, rows: &mut Vec<Vec<Target>>) {
        todo!()
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        Self::wires_public_inputs_hash()
            .zip(vars.public_inputs_hash.elements)
            .map(|(wire, hash_part)| vars.local_wires[wire] - hash_part.into())
            .collect()
    }

    fn eval_unfiltered_base_one(
        &self,
        _vars: EvaluationVarsBase<F>,
        _yield_constr: StridedConstraintConsumer<F>,
    ) {
        panic!("use eval_unfiltered_base_packed instead");
    }

    fn eval_unfiltered_base_batch(&self, vars_base: EvaluationVarsBaseBatch<F>) -> Vec<F> {
        self.eval_unfiltered_base_batch_packed(vars_base)
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

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D> for PublicInputGate {
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        vars: EvaluationVarsBasePacked<P>,
        mut yield_constr: StridedConstraintConsumer<P>,
    ) {
        yield_constr.many(
            Self::wires_public_inputs_hash()
                .zip(vars.public_inputs_hash.elements)
                .map(|(wire, hash_part)| vars.local_wires[wire] - hash_part),
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PublicInputOperation {
    public_inputs_hash: HashOutTarget,
    pub(crate) gate: PublicInputGate,
}

impl<F: RichField> SimpleGenerator<F> for PublicInputOperation {
    fn dependencies(&self) -> Vec<Target> {
        vec![]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {}
}

impl<F: RichField + Extendable<D>, const D: usize> Operation<F, D> for PublicInputOperation {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn targets(&self) -> Vec<Target> {
        self.public_inputs_hash.elements.to_vec()
    }

    fn gate(&self) -> Option<GateRef<F, D>> {
        Some(GateRef::new(self.gate))
    }

    fn constants(&self) -> Vec<F> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use plonky2_field::goldilocks_field::GoldilocksField;

    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::public_input::PublicInputGate;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(PublicInputGate)
    }

    #[test]
    fn eval_fns() -> anyhow::Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(PublicInputGate)
    }
}
