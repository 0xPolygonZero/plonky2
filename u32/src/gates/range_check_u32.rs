use std::marker::PhantomData;

use plonky2::gates::gate::Gate;
use plonky2::gates::util::StridedConstraintConsumer;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartitionWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::plonk_common::{reduce_with_powers, reduce_with_powers_ext_circuit};
use plonky2::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use plonky2_field::extension::Extendable;
use plonky2_field::types::Field;
use plonky2_util::ceil_div_usize;

/// A gate which can decompose a number into base B little-endian limbs.
#[derive(Copy, Clone, Debug)]
pub struct U32RangeCheckGate<F: RichField + Extendable<D>, const D: usize> {
    pub num_input_limbs: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> U32RangeCheckGate<F, D> {
    pub fn new(num_input_limbs: usize) -> Self {
        Self {
            num_input_limbs,
            _phantom: PhantomData,
        }
    }

    pub const AUX_LIMB_BITS: usize = 2;
    pub const BASE: usize = 1 << Self::AUX_LIMB_BITS;

    fn aux_limbs_per_input_limb(&self) -> usize {
        ceil_div_usize(32, Self::AUX_LIMB_BITS)
    }
    pub fn wire_ith_input_limb(&self, i: usize) -> usize {
        debug_assert!(i < self.num_input_limbs);
        i
    }
    pub fn wire_ith_input_limb_jth_aux_limb(&self, i: usize, j: usize) -> usize {
        debug_assert!(i < self.num_input_limbs);
        debug_assert!(j < self.aux_limbs_per_input_limb());
        self.num_input_limbs + self.aux_limbs_per_input_limb() * i + j
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for U32RangeCheckGate<F, D> {
    fn id(&self) -> String {
        format!("{self:?}")
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let base = F::Extension::from_canonical_usize(Self::BASE);
        for i in 0..self.num_input_limbs {
            let input_limb = vars.local_wires[self.wire_ith_input_limb(i)];
            let aux_limbs: Vec<_> = (0..self.aux_limbs_per_input_limb())
                .map(|j| vars.local_wires[self.wire_ith_input_limb_jth_aux_limb(i, j)])
                .collect();
            let computed_sum = reduce_with_powers(&aux_limbs, base);

            constraints.push(computed_sum - input_limb);
            for aux_limb in aux_limbs {
                constraints.push(
                    (0..Self::BASE)
                        .map(|i| aux_limb - F::Extension::from_canonical_usize(i))
                        .product(),
                );
            }
        }

        constraints
    }

    fn eval_unfiltered_base_one(
        &self,
        vars: EvaluationVarsBase<F>,
        mut yield_constr: StridedConstraintConsumer<F>,
    ) {
        let base = F::from_canonical_usize(Self::BASE);
        for i in 0..self.num_input_limbs {
            let input_limb = vars.local_wires[self.wire_ith_input_limb(i)];
            let aux_limbs: Vec<_> = (0..self.aux_limbs_per_input_limb())
                .map(|j| vars.local_wires[self.wire_ith_input_limb_jth_aux_limb(i, j)])
                .collect();
            let computed_sum = reduce_with_powers(&aux_limbs, base);

            yield_constr.one(computed_sum - input_limb);
            for aux_limb in aux_limbs {
                yield_constr.one(
                    (0..Self::BASE)
                        .map(|i| aux_limb - F::from_canonical_usize(i))
                        .product(),
                );
            }
        }
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        let base = builder.constant(F::from_canonical_usize(Self::BASE));
        for i in 0..self.num_input_limbs {
            let input_limb = vars.local_wires[self.wire_ith_input_limb(i)];
            let aux_limbs: Vec<_> = (0..self.aux_limbs_per_input_limb())
                .map(|j| vars.local_wires[self.wire_ith_input_limb_jth_aux_limb(i, j)])
                .collect();
            let computed_sum = reduce_with_powers_ext_circuit(builder, &aux_limbs, base);

            constraints.push(builder.sub_extension(computed_sum, input_limb));
            for aux_limb in aux_limbs {
                constraints.push({
                    let mut acc = builder.one_extension();
                    (0..Self::BASE).for_each(|i| {
                        // We update our accumulator as:
                        // acc' = acc (x - i)
                        //      = acc x + (-i) acc
                        // Since -i is constant, we can do this in one arithmetic_extension call.
                        let neg_i = -F::from_canonical_usize(i);
                        acc = builder.arithmetic_extension(F::ONE, neg_i, acc, aux_limb, acc)
                    });
                    acc
                });
            }
        }

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = U32RangeCheckGenerator { gate: *self, row };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        self.num_input_limbs * (1 + self.aux_limbs_per_input_limb())
    }

    fn num_constants(&self) -> usize {
        0
    }

    // Bounded by the range-check (x-0)*(x-1)*...*(x-BASE+1).
    fn degree(&self) -> usize {
        Self::BASE
    }

    // 1 for checking the each sum of aux limbs, plus a range check for each aux limb.
    fn num_constraints(&self) -> usize {
        self.num_input_limbs * (1 + self.aux_limbs_per_input_limb())
    }
}

#[derive(Debug)]
pub struct U32RangeCheckGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate: U32RangeCheckGate<F, D>,
    row: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for U32RangeCheckGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let num_input_limbs = self.gate.num_input_limbs;
        (0..num_input_limbs)
            .map(|i| Target::wire(self.row, self.gate.wire_ith_input_limb(i)))
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let num_input_limbs = self.gate.num_input_limbs;
        for i in 0..num_input_limbs {
            let sum_value = witness
                .get_target(Target::wire(self.row, self.gate.wire_ith_input_limb(i)))
                .to_canonical_u64() as u32;

            let base = U32RangeCheckGate::<F, D>::BASE as u32;
            let limbs = (0..self.gate.aux_limbs_per_input_limb())
                .map(|j| Target::wire(self.row, self.gate.wire_ith_input_limb_jth_aux_limb(i, j)));
            let limbs_value = (0..self.gate.aux_limbs_per_input_limb())
                .scan(sum_value, |acc, _| {
                    let tmp = *acc % base;
                    *acc /= base;
                    Some(F::from_canonical_u32(tmp))
                })
                .collect::<Vec<_>>();

            for (b, b_value) in limbs.zip(limbs_value) {
                out_buffer.set_target(b, b_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use itertools::unfold;
    use plonky2::gates::gate::Gate;
    use plonky2::gates::gate_testing::{test_eval_fns, test_low_degree};
    use plonky2::hash::hash_types::HashOut;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::plonk::vars::EvaluationVars;
    use plonky2_field::extension::quartic::QuarticExtension;
    use plonky2_field::goldilocks_field::GoldilocksField;
    use plonky2_field::types::Field;
    use plonky2_util::ceil_div_usize;
    use rand::Rng;

    use crate::gates::range_check_u32::U32RangeCheckGate;

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(U32RangeCheckGate::new(8))
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(U32RangeCheckGate::new(8))
    }

    fn test_gate_constraint(input_limbs: Vec<u64>) {
        type F = GoldilocksField;
        type FF = QuarticExtension<GoldilocksField>;
        const D: usize = 4;
        const AUX_LIMB_BITS: usize = 2;
        const BASE: usize = 1 << AUX_LIMB_BITS;
        const AUX_LIMBS_PER_INPUT_LIMB: usize = ceil_div_usize(32, AUX_LIMB_BITS);

        fn get_wires(input_limbs: Vec<u64>) -> Vec<FF> {
            let num_input_limbs = input_limbs.len();
            let mut v = Vec::new();

            for i in 0..num_input_limbs {
                let input_limb = input_limbs[i];

                let split_to_limbs = |mut val, num| {
                    unfold((), move |_| {
                        let ret = val % (BASE as u64);
                        val /= BASE as u64;
                        Some(ret)
                    })
                    .take(num)
                    .map(F::from_canonical_u64)
                };

                let mut aux_limbs: Vec<_> =
                    split_to_limbs(input_limb, AUX_LIMBS_PER_INPUT_LIMB).collect();

                v.append(&mut aux_limbs);
            }

            input_limbs
                .iter()
                .cloned()
                .map(F::from_canonical_u64)
                .chain(v.iter().cloned())
                .map(|x| x.into())
                .collect()
        }

        let gate = U32RangeCheckGate::<F, D> {
            num_input_limbs: 8,
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(input_limbs),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }

    #[test]
    fn test_gate_constraint_good() {
        let mut rng = rand::thread_rng();
        let input_limbs: Vec<_> = (0..8).map(|_| rng.gen::<u32>() as u64).collect();

        test_gate_constraint(input_limbs);
    }

    #[test]
    #[should_panic]
    fn test_gate_constraint_bad() {
        let mut rng = rand::thread_rng();
        let input_limbs: Vec<_> = (0..8).map(|_| rng.gen()).collect();

        test_gate_constraint(input_limbs);
    }
}
