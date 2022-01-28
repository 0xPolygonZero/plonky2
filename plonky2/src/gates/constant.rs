use std::ops::Range;

use plonky2_field::extension_field::Extendable;
use plonky2_field::field_types::Field;
use plonky2_field::packed_field::PackedField;

use crate::gates::batchable::MultiOpsGate;
use crate::gates::gate::Gate;
use crate::gates::packed_util::PackedEvaluableBase;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::PartitionWitness;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};

/// A gate which takes a single constant parameter and outputs that value.
#[derive(Copy, Clone, Debug)]
pub struct ConstantGate {
    pub(crate) num_consts: usize,
}

impl ConstantGate {
    pub fn consts_inputs(&self) -> Range<usize> {
        0..self.num_consts
    }

    pub fn wires_outputs(&self) -> Range<usize> {
        0..self.num_consts
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for ConstantGate {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        self.consts_inputs()
            .zip(self.wires_outputs())
            .map(|(con, out)| vars.local_constants[con] - vars.local_wires[out])
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
        self.consts_inputs()
            .zip(self.wires_outputs())
            .map(|(con, out)| {
                builder.sub_extension(vars.local_constants[con], vars.local_wires[out])
            })
            .collect()
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = ConstantGenerator {
            gate_index,
            gate: *self,
            constants: local_constants[self.consts_inputs()].to_vec(),
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        self.num_consts
    }

    fn num_constants(&self) -> usize {
        self.num_consts
    }

    fn degree(&self) -> usize {
        1
    }

    fn num_constraints(&self) -> usize {
        self.num_consts
    }
}

impl<F: RichField + Extendable<D>, const D: usize> MultiOpsGate<F, D> for ConstantGate {
    fn num_ops(&self) -> usize {
        self.num_consts
    }

    fn dependencies_ith_op(&self, gate_index: usize, i: usize) -> Vec<Target> {
        vec![]
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D> for ConstantGate {
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        vars: EvaluationVarsBasePacked<P>,
        mut yield_constr: StridedConstraintConsumer<P>,
    ) {
        yield_constr.many(
            self.consts_inputs()
                .zip(self.wires_outputs())
                .map(|(con, out)| vars.local_constants[con] - vars.local_wires[out]),
        );
    }
}

#[derive(Debug)]
struct ConstantGenerator<F: Field> {
    gate_index: usize,
    gate: ConstantGate,
    constants: Vec<F>,
}

impl<F: Field> SimpleGenerator<F> for ConstantGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        Vec::new()
    }

    fn run_once(&self, _witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        for (con, out) in self.gate.consts_inputs().zip(self.gate.wires_outputs()) {
            let wire = Wire {
                gate: self.gate_index,
                input: out,
            };
            out_buffer.set_wire(wire, self.constants[con]);
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::goldilocks_field::GoldilocksField;

    use crate::gates::constant::ConstantGate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    #[test]
    fn low_degree() {
        let num_consts = CircuitConfig::standard_recursion_config().constant_gate_size;
        let gate = ConstantGate { num_consts };
        test_low_degree::<GoldilocksField, _, 2>(gate)
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let num_consts = CircuitConfig::standard_recursion_config().constant_gate_size;
        let gate = ConstantGate { num_consts };
        test_eval_fns::<F, C, _, D>(gate)
    }
}
