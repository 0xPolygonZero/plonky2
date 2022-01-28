use std::marker::PhantomData;

use itertools::unfold;
use plonky2_util::ceil_div_usize;

use crate::field::extension_field::Extendable;
use crate::field::field_types::Field;
use crate::gates::gate::Gate;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CircuitConfig;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

const LOG2_MAX_NUM_ADDENDS: usize = 4;
const MAX_NUM_ADDENDS: usize = 16;

/// A gate to perform addition on `num_addends` different 32-bit values, plus a small carry
#[derive(Copy, Clone, Debug)]
pub struct MulU32BoolGate<F: RichField + Extendable<D>, const D: usize> {
    pub num_ops: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> MulU32BoolGate<F, D> {
    pub fn new_from_config(config: &CircuitConfig) -> Self {
        Self {
            num_ops: Self::num_ops(config),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn num_ops(config: &CircuitConfig) -> usize {
        config.num_routed_wires / (1 + 2 * 8)
    }

    pub fn wire_bool(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        17 * i
    }
    pub fn wire_ith_op_jth_input(&self, i: usize, j: usize) -> usize {
        debug_assert!(i < self.num_ops);
        17 * i + 1 + 2 * j
    }

    pub fn wire_ith_op_jth_output(&self, i: usize, j: usize) -> usize {
        debug_assert!(i < self.num_ops);
        17 * i + 2 + 2 * j
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for MulU32BoolGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        (0..self.num_ops)
            .flat_map(|i| {
                let b = vars.local_wires[self.wire_bool(i)];
                (0..8).map(move |j| {
                    b * vars.local_wires[self.wire_ith_op_jth_input(i, j)]
                        - vars.local_wires[self.wire_ith_op_jth_output(i, j)]
                })
            })
            .collect()
    }

    fn eval_unfiltered_base_one(
        &self,
        vars: EvaluationVarsBase<F>,
        mut yield_constr: StridedConstraintConsumer<F>,
    ) {
        (0..self.num_ops).for_each(|i| {
            let b = vars.local_wires[self.wire_bool(i)];
            (0..8).for_each(|j| {
                yield_constr.one(
                    b * vars.local_wires[self.wire_ith_op_jth_input(i, j)]
                        - vars.local_wires[self.wire_ith_op_jth_output(i, j)],
                );
            })
        });
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        (0..self.num_ops)
            .flat_map(|i| {
                let b = vars.local_wires[self.wire_bool(i)];
                (0..8)
                    .map(|j| {
                        builder.mul_sub_extension(
                            b,
                            vars.local_wires[self.wire_ith_op_jth_input(i, j)],
                            vars.local_wires[self.wire_ith_op_jth_output(i, j)],
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..self.num_ops)
            .map(|i| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    MulU32BoolGenerator {
                        gate: *self,
                        gate_index,
                        i,
                        _phantom: PhantomData,
                    }
                    .adapter(),
                );
                g
            })
            .collect()
    }

    fn num_wires(&self) -> usize {
        17 * self.num_ops
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        2
    }

    fn num_constraints(&self) -> usize {
        self.num_ops * 8
    }
}

#[derive(Clone, Debug)]
struct MulU32BoolGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate: MulU32BoolGate<F, D>,
    gate_index: usize,
    i: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for MulU32BoolGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);
        (0..8)
            .map(|j| local_target(self.gate.wire_ith_op_jth_input(self.i, j)))
            .chain(Some(local_target(self.gate.wire_bool(self.i))))
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));
        let b = get_local_wire(self.gate.wire_bool(self.i));
        for j in 0..8 {
            let input = get_local_wire(self.gate.wire_ith_op_jth_input(self.i, j));
            out_buffer.set_wire(
                local_wire(self.gate.wire_ith_op_jth_output(self.i, j)),
                b * input,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use itertools::unfold;
    use rand::Rng;

    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::mul_biguint_bool::MulU32BoolGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(MulU32BoolGate::<GoldilocksField, 4> {
            num_ops: 3,
            _phantom: PhantomData,
        })
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(MulU32BoolGate::<GoldilocksField, D> {
            num_ops: 4,
            _phantom: PhantomData,
        })
    }
}
