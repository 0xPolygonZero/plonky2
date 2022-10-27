use std::marker::PhantomData;

use itertools::unfold;
use plonky2::gates::gate::Gate;
use plonky2::gates::util::StridedConstraintConsumer;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::wire::Wire;
use plonky2::iop::witness::{PartitionWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use plonky2_field::extension::Extendable;
use plonky2_field::types::Field;
use plonky2_util::ceil_div_usize;

const LOG2_MAX_NUM_ADDENDS: usize = 4;
const MAX_NUM_ADDENDS: usize = 16;

/// A gate to perform addition on `num_addends` different 32-bit values, plus a small carry
#[derive(Copy, Clone, Debug)]
pub struct U32AddManyGate<F: RichField + Extendable<D>, const D: usize> {
    pub num_addends: usize,
    pub num_ops: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> U32AddManyGate<F, D> {
    pub fn new_from_config(config: &CircuitConfig, num_addends: usize) -> Self {
        Self {
            num_addends,
            num_ops: Self::num_ops(num_addends, config),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn num_ops(num_addends: usize, config: &CircuitConfig) -> usize {
        debug_assert!(num_addends <= MAX_NUM_ADDENDS);
        let wires_per_op = (num_addends + 3) + Self::num_limbs();
        let routed_wires_per_op = num_addends + 3;
        (config.num_wires / wires_per_op).min(config.num_routed_wires / routed_wires_per_op)
    }

    pub fn wire_ith_op_jth_addend(&self, i: usize, j: usize) -> usize {
        debug_assert!(i < self.num_ops);
        debug_assert!(j < self.num_addends);
        (self.num_addends + 3) * i + j
    }
    pub fn wire_ith_carry(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        (self.num_addends + 3) * i + self.num_addends
    }

    pub fn wire_ith_output_result(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        (self.num_addends + 3) * i + self.num_addends + 1
    }
    pub fn wire_ith_output_carry(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        (self.num_addends + 3) * i + self.num_addends + 2
    }

    pub fn limb_bits() -> usize {
        2
    }
    pub fn num_result_limbs() -> usize {
        ceil_div_usize(32, Self::limb_bits())
    }
    pub fn num_carry_limbs() -> usize {
        ceil_div_usize(LOG2_MAX_NUM_ADDENDS, Self::limb_bits())
    }
    pub fn num_limbs() -> usize {
        Self::num_result_limbs() + Self::num_carry_limbs()
    }

    pub fn wire_ith_output_jth_limb(&self, i: usize, j: usize) -> usize {
        debug_assert!(i < self.num_ops);
        debug_assert!(j < Self::num_limbs());
        (self.num_addends + 3) * self.num_ops + Self::num_limbs() * i + j
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for U32AddManyGate<F, D> {
    fn id(&self) -> String {
        format!("{self:?}")
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..self.num_ops {
            let addends: Vec<F::Extension> = (0..self.num_addends)
                .map(|j| vars.local_wires[self.wire_ith_op_jth_addend(i, j)])
                .collect();
            let carry = vars.local_wires[self.wire_ith_carry(i)];

            let computed_output = addends.iter().fold(F::Extension::ZERO, |x, &y| x + y) + carry;

            let output_result = vars.local_wires[self.wire_ith_output_result(i)];
            let output_carry = vars.local_wires[self.wire_ith_output_carry(i)];

            let base = F::Extension::from_canonical_u64(1 << 32u64);
            let combined_output = output_carry * base + output_result;

            constraints.push(combined_output - computed_output);

            let mut combined_result_limbs = F::Extension::ZERO;
            let mut combined_carry_limbs = F::Extension::ZERO;
            let base = F::Extension::from_canonical_u64(1u64 << Self::limb_bits());
            for j in (0..Self::num_limbs()).rev() {
                let this_limb = vars.local_wires[self.wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::Extension::from_canonical_usize(x))
                    .product();
                constraints.push(product);

                if j < Self::num_result_limbs() {
                    combined_result_limbs = base * combined_result_limbs + this_limb;
                } else {
                    combined_carry_limbs = base * combined_carry_limbs + this_limb;
                }
            }
            constraints.push(combined_result_limbs - output_result);
            constraints.push(combined_carry_limbs - output_carry);
        }

        constraints
    }

    fn eval_unfiltered_base_one(
        &self,
        vars: EvaluationVarsBase<F>,
        mut yield_constr: StridedConstraintConsumer<F>,
    ) {
        for i in 0..self.num_ops {
            let addends: Vec<F> = (0..self.num_addends)
                .map(|j| vars.local_wires[self.wire_ith_op_jth_addend(i, j)])
                .collect();
            let carry = vars.local_wires[self.wire_ith_carry(i)];

            let computed_output = addends.iter().fold(F::ZERO, |x, &y| x + y) + carry;

            let output_result = vars.local_wires[self.wire_ith_output_result(i)];
            let output_carry = vars.local_wires[self.wire_ith_output_carry(i)];

            let base = F::from_canonical_u64(1 << 32u64);
            let combined_output = output_carry * base + output_result;

            yield_constr.one(combined_output - computed_output);

            let mut combined_result_limbs = F::ZERO;
            let mut combined_carry_limbs = F::ZERO;
            let base = F::from_canonical_u64(1u64 << Self::limb_bits());
            for j in (0..Self::num_limbs()).rev() {
                let this_limb = vars.local_wires[self.wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::from_canonical_usize(x))
                    .product();
                yield_constr.one(product);

                if j < Self::num_result_limbs() {
                    combined_result_limbs = base * combined_result_limbs + this_limb;
                } else {
                    combined_carry_limbs = base * combined_carry_limbs + this_limb;
                }
            }
            yield_constr.one(combined_result_limbs - output_result);
            yield_constr.one(combined_carry_limbs - output_carry);
        }
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for i in 0..self.num_ops {
            let addends: Vec<ExtensionTarget<D>> = (0..self.num_addends)
                .map(|j| vars.local_wires[self.wire_ith_op_jth_addend(i, j)])
                .collect();
            let carry = vars.local_wires[self.wire_ith_carry(i)];

            let mut computed_output = carry;
            for addend in addends {
                computed_output = builder.add_extension(computed_output, addend);
            }

            let output_result = vars.local_wires[self.wire_ith_output_result(i)];
            let output_carry = vars.local_wires[self.wire_ith_output_carry(i)];

            let base: F::Extension = F::from_canonical_u64(1 << 32u64).into();
            let base_target = builder.constant_extension(base);
            let combined_output =
                builder.mul_add_extension(output_carry, base_target, output_result);

            constraints.push(builder.sub_extension(combined_output, computed_output));

            let mut combined_result_limbs = builder.zero_extension();
            let mut combined_carry_limbs = builder.zero_extension();
            let base = builder
                .constant_extension(F::Extension::from_canonical_u64(1u64 << Self::limb_bits()));
            for j in (0..Self::num_limbs()).rev() {
                let this_limb = vars.local_wires[self.wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();

                let mut product = builder.one_extension();
                for x in 0..max_limb {
                    let x_target =
                        builder.constant_extension(F::Extension::from_canonical_usize(x));
                    let diff = builder.sub_extension(this_limb, x_target);
                    product = builder.mul_extension(product, diff);
                }
                constraints.push(product);

                if j < Self::num_result_limbs() {
                    combined_result_limbs =
                        builder.mul_add_extension(base, combined_result_limbs, this_limb);
                } else {
                    combined_carry_limbs =
                        builder.mul_add_extension(base, combined_carry_limbs, this_limb);
                }
            }
            constraints.push(builder.sub_extension(combined_result_limbs, output_result));
            constraints.push(builder.sub_extension(combined_carry_limbs, output_carry));
        }

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..self.num_ops)
            .map(|i| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    U32AddManyGenerator {
                        gate: *self,
                        row,
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
        (self.num_addends + 3) * self.num_ops + Self::num_limbs() * self.num_ops
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1 << Self::limb_bits()
    }

    fn num_constraints(&self) -> usize {
        self.num_ops * (3 + Self::num_limbs())
    }
}

#[derive(Clone, Debug)]
struct U32AddManyGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate: U32AddManyGate<F, D>,
    row: usize,
    i: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for U32AddManyGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |column| Target::wire(self.row, column);

        (0..self.gate.num_addends)
            .map(|j| local_target(self.gate.wire_ith_op_jth_addend(self.i, j)))
            .chain([local_target(self.gate.wire_ith_carry(self.i))])
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |column| Wire {
            row: self.row,
            column,
        };

        let get_local_wire = |column| witness.get_wire(local_wire(column));

        let addends: Vec<_> = (0..self.gate.num_addends)
            .map(|j| get_local_wire(self.gate.wire_ith_op_jth_addend(self.i, j)))
            .collect();
        let carry = get_local_wire(self.gate.wire_ith_carry(self.i));

        let output = addends.iter().fold(F::ZERO, |x, &y| x + y) + carry;
        let output_u64 = output.to_canonical_u64();

        let output_carry_u64 = output_u64 >> 32;
        let output_result_u64 = output_u64 & ((1 << 32) - 1);

        let output_carry = F::from_canonical_u64(output_carry_u64);
        let output_result = F::from_canonical_u64(output_result_u64);

        let output_carry_wire = local_wire(self.gate.wire_ith_output_carry(self.i));
        let output_result_wire = local_wire(self.gate.wire_ith_output_result(self.i));

        out_buffer.set_wire(output_carry_wire, output_carry);
        out_buffer.set_wire(output_result_wire, output_result);

        let num_result_limbs = U32AddManyGate::<F, D>::num_result_limbs();
        let num_carry_limbs = U32AddManyGate::<F, D>::num_carry_limbs();
        let limb_base = 1 << U32AddManyGate::<F, D>::limb_bits();

        let split_to_limbs = |mut val, num| {
            unfold((), move |_| {
                let ret = val % limb_base;
                val /= limb_base;
                Some(ret)
            })
            .take(num)
            .map(F::from_canonical_u64)
        };

        let result_limbs = split_to_limbs(output_result_u64, num_result_limbs);
        let carry_limbs = split_to_limbs(output_carry_u64, num_carry_limbs);

        for (j, limb) in result_limbs.chain(carry_limbs).enumerate() {
            let wire = local_wire(self.gate.wire_ith_output_jth_limb(self.i, j));
            out_buffer.set_wire(wire, limb);
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
    use rand::Rng;

    use crate::gates::add_many_u32::U32AddManyGate;

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(U32AddManyGate::<GoldilocksField, 4> {
            num_addends: 4,
            num_ops: 3,
            _phantom: PhantomData,
        })
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(U32AddManyGate::<GoldilocksField, D> {
            num_addends: 4,
            num_ops: 3,
            _phantom: PhantomData,
        })
    }

    #[test]
    fn test_gate_constraint() {
        type F = GoldilocksField;
        type FF = QuarticExtension<GoldilocksField>;
        const D: usize = 4;
        const NUM_ADDENDS: usize = 10;
        const NUM_U32_ADD_MANY_OPS: usize = 3;

        fn get_wires(addends: Vec<Vec<u64>>, carries: Vec<u64>) -> Vec<FF> {
            let mut v0 = Vec::new();
            let mut v1 = Vec::new();

            let num_result_limbs = U32AddManyGate::<F, D>::num_result_limbs();
            let num_carry_limbs = U32AddManyGate::<F, D>::num_carry_limbs();
            let limb_base = 1 << U32AddManyGate::<F, D>::limb_bits();
            for op in 0..NUM_U32_ADD_MANY_OPS {
                let adds = &addends[op];
                let ca = carries[op];

                let output = adds.iter().sum::<u64>() + ca;
                let output_result = output & ((1 << 32) - 1);
                let output_carry = output >> 32;

                let split_to_limbs = |mut val, num| {
                    unfold((), move |_| {
                        let ret = val % limb_base;
                        val /= limb_base;
                        Some(ret)
                    })
                    .take(num)
                    .map(F::from_canonical_u64)
                };

                let mut result_limbs: Vec<_> =
                    split_to_limbs(output_result, num_result_limbs).collect();
                let mut carry_limbs: Vec<_> =
                    split_to_limbs(output_carry, num_carry_limbs).collect();

                for a in adds {
                    v0.push(F::from_canonical_u64(*a));
                }
                v0.push(F::from_canonical_u64(ca));
                v0.push(F::from_canonical_u64(output_result));
                v0.push(F::from_canonical_u64(output_carry));
                v1.append(&mut result_limbs);
                v1.append(&mut carry_limbs);
            }

            v0.iter().chain(v1.iter()).map(|&x| x.into()).collect()
        }

        let mut rng = rand::thread_rng();
        let addends: Vec<Vec<_>> = (0..NUM_U32_ADD_MANY_OPS)
            .map(|_| (0..NUM_ADDENDS).map(|_| rng.gen::<u32>() as u64).collect())
            .collect();
        let carries: Vec<_> = (0..NUM_U32_ADD_MANY_OPS)
            .map(|_| rng.gen::<u32>() as u64)
            .collect();

        let gate = U32AddManyGate::<F, D> {
            num_addends: NUM_ADDENDS,
            num_ops: NUM_U32_ADD_MANY_OPS,
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(addends, carries),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
