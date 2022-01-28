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
pub struct Secp256k1BaseBoolGate<F: RichField + Extendable<D>, const D: usize> {
    pub num_ops: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> Secp256k1BaseBoolGate<F, D> {
    const LIMBS: [usize; 3] = [0xfffffc2f, 0xfffffffe, 0xffffffff];

    pub fn new_from_config(config: &CircuitConfig) -> Self {
        Self {
            num_ops: Self::num_ops(config),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn num_ops(config: &CircuitConfig) -> usize {
        config.num_routed_wires / 4
    }

    pub fn wire_bool(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        4 * i
    }
    pub fn wire_ith_op_jth_output(&self, i: usize, j: usize) -> usize {
        debug_assert!(i < self.num_ops);
        debug_assert!(j < 3);
        4 * i + 1 + j
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for Secp256k1BaseBoolGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        (0..self.num_ops)
            .flat_map(|i| {
                let b = vars.local_wires[self.wire_bool(i)];
                (0..3).map(move |j| {
                    b * F::Extension::from_canonical_usize(Self::LIMBS[j])
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
            (0..3).for_each(|j| {
                yield_constr.one(
                    b * F::from_canonical_usize(Self::LIMBS[j])
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
                (0..3)
                    .map(|j| {
                        let c = builder
                            .constant_extension(F::Extension::from_canonical_usize(Self::LIMBS[j]));
                        builder.mul_sub_extension(
                            b,
                            c,
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
                    Secp256k1BaseBoolGenerator {
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
        4 * self.num_ops
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1
    }

    fn num_constraints(&self) -> usize {
        self.num_ops * 3
    }
}

#[derive(Clone, Debug)]
struct Secp256k1BaseBoolGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate: Secp256k1BaseBoolGate<F, D>,
    gate_index: usize,
    i: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for Secp256k1BaseBoolGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        vec![Target::wire(self.gate_index, self.gate.wire_bool(self.i))]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));
        let b = get_local_wire(self.gate.wire_bool(self.i));
        for j in 0..3 {
            out_buffer.set_wire(
                local_wire(self.gate.wire_ith_op_jth_output(self.i, j)),
                b * F::from_canonical_usize(Secp256k1BaseBoolGate::<F, D>::LIMBS[j]),
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
    use crate::gates::secp256k1_base_bool::Secp256k1BaseBoolGate;
    use crate::hash::hash_types::HashOut;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(Secp256k1BaseBoolGate::<GoldilocksField, 4> {
            num_ops: 3,
            _phantom: PhantomData,
        })
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(Secp256k1BaseBoolGate::<GoldilocksField, D> {
            num_ops: 4,
            _phantom: PhantomData,
        })
    }
}
