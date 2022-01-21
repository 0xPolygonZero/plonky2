use std::marker::PhantomData;

use itertools::unfold;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gates::gate::Gate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CircuitConfig;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

const LOG2_MAX_NUM_ADDENDS: usize = 6;
const MAX_NUM_ADDENDS: usize = 1 << LOG2_MAX_NUM_ADDENDS;

/// A gate to perform addition on `num_addends` different 32-bit values, plus a small carry
#[derive(Copy, Clone, Debug)]
pub struct U32AddManyGate<F: RichField + Extendable<D>, const D: usize> {
    pub num_addends: usize,
    pub num_ops: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> U32AddManyGate<F, D> {
    pub fn new_from_config(num_addends: usize, config: &CircuitConfig) -> Self {
        Self {
            num_addends,
            num_ops: Self::num_ops(num_addends, config),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn num_ops(num_addends: usize, config: &CircuitConfig) -> usize {
        debug_assert!(num_addends < MAX_NUM_ADDENDS);
        let wires_per_op = (num_addends + 3) + Self::num_limbs();
        let routed_wires_per_op = 5;
        (config.num_wires / wires_per_op).min(config.num_routed_wires / routed_wires_per_op)
    }

    pub fn wire_ith_op_jth_addend(&self, i: usize, j: usize) -> usize {
        debug_assert!(i < self.num_ops);
        debug_assert!(i < self.num_addends);
        (self.num_addends + 3) * i + j
    }
    pub fn wire_ith_carry(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        (self.num_addends + 3) * i + self.num_addends
    }

    pub fn wire_ith_output_low_half(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        (self.num_addends + 3) * i + self.num_addends + 1
    }
    pub fn wire_ith_output_high_half(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        (self.num_addends + 3) * i + self.num_addends + 2
    }

    pub fn limb_bits() -> usize {
        2
    }
    pub fn num_limbs() -> usize {
        32 / Self::limb_bits()
    }

    pub fn wire_ith_output_jth_limb(&self, i: usize, j: usize) -> usize {
        debug_assert!(i < self.num_ops);
        debug_assert!(j < Self::num_limbs());
        (self.num_addends + 3) * self.num_ops + Self::num_limbs() * i + j
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for U32AddManyGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..self.num_ops {
            let addends: Vec<F::Extension> = (0..self.num_addends)
                .map(|j| vars.local_wires[self.wire_ith_op_jth_addend(i, j)])
                .collect();
            let borrow = vars.local_wires[self.wire_ith_carry(i)];

            let computed_output = addends.iter().fold(F::Extension::ZERO, |x, &y| x + y) + borrow;

            let output_low = vars.local_wires[self.wire_ith_output_low_half(i)];
            let output_high = vars.local_wires[self.wire_ith_output_high_half(i)];

            let base = F::Extension::from_canonical_u64(1 << 32u64);
            let combined_output = output_high * base + output_low;

            constraints.push(combined_output - computed_output);

            let mut combined_low_limbs = F::Extension::ZERO;
            let base = F::Extension::from_canonical_u64(1u64 << Self::limb_bits());
            for j in (0..Self::num_limbs()).rev() {
                let this_limb = vars.local_wires[self.wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::Extension::from_canonical_usize(x))
                    .product();
                constraints.push(product);

                combined_low_limbs = base * combined_low_limbs + this_limb;
            }
            constraints.push(combined_low_limbs - output_low);

            let max_overflow = self.num_addends;
            let product = (0..max_overflow)
                .map(|x| output_high - F::Extension::from_canonical_usize(x))
                .product();
            constraints.push(product);
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..self.num_ops {
            let addends: Vec<F> = (0..self.num_addends)
                .map(|j| vars.local_wires[self.wire_ith_op_jth_addend(i, j)])
                .collect();
            let borrow = vars.local_wires[self.wire_ith_carry(i)];

            let computed_output = addends.iter().fold(F::ZERO, |x, &y| x + y) + borrow;

            let output_low = vars.local_wires[self.wire_ith_output_low_half(i)];
            let output_high = vars.local_wires[self.wire_ith_output_high_half(i)];

            let base = F::from_canonical_u64(1 << 32u64);
            let combined_output = output_high * base + output_low;

            constraints.push(combined_output - computed_output);

            let mut combined_low_limbs = F::ZERO;
            let base = F::from_canonical_u64(1u64 << Self::limb_bits());
            for j in (0..Self::num_limbs()).rev() {
                let this_limb = vars.local_wires[self.wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::from_canonical_usize(x))
                    .product();
                constraints.push(product);

                combined_low_limbs = base * combined_low_limbs + this_limb;
            }
            constraints.push(combined_low_limbs - output_low);

            let max_overflow = self.num_addends;
            let product = (0..max_overflow)
                .map(|x| output_high - F::from_canonical_usize(x))
                .product();
            constraints.push(product);
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for i in 0..self.num_ops {
            let addends: Vec<ExtensionTarget<D>> = (0..self.num_addends)
                .map(|j| vars.local_wires[self.wire_ith_op_jth_addend(i, j)])
                .collect();
            let borrow = vars.local_wires[self.wire_ith_carry(i)];

            let mut computed_output = borrow;
            for addend in addends {
                computed_output = builder.add_extension(computed_output, addend);
            }

            let output_low = vars.local_wires[self.wire_ith_output_low_half(i)];
            let output_high = vars.local_wires[self.wire_ith_output_high_half(i)];

            let base: F::Extension = F::from_canonical_u64(1 << 32u64).into();
            let base_target = builder.constant_extension(base);
            let combined_output = builder.mul_add_extension(output_high, base_target, output_low);

            constraints.push(builder.sub_extension(combined_output, computed_output));

            let mut combined_low_limbs = builder.zero_extension();
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

                combined_low_limbs = builder.mul_add_extension(base, combined_low_limbs, this_limb);
            }
            constraints.push(builder.sub_extension(combined_low_limbs, output_low));

            let max_overflow = self.num_addends;
            let mut product = builder.one_extension();
            for x in 0..max_overflow {
                let x_target = builder.constant_extension(F::Extension::from_canonical_usize(x));
                let diff = builder.sub_extension(output_high, x_target);
                product = builder.mul_extension(product, diff);
            }
            constraints.push(product);
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..self.num_ops)
            .map(|i| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    U32AddManyGenerator {
                        gate: *self,
                        gate_index,
                        i,
                        _phantom: PhantomData,
                    }
                    .adapter(),
                );
                g
            })
            .collect::<Vec<_>>()
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
    gate_index: usize,
    i: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for U32AddManyGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        (0..self.gate.num_addends)
            .map(|j| local_target(self.gate.wire_ith_op_jth_addend(self.i, j)))
            .chain([local_target(self.gate.wire_ith_carry(self.i))])
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let addends: Vec<_> = (0..self.gate.num_addends).map(|j| get_local_wire(self.gate.wire_ith_output_jth_limb(self.i, j))).collect();
        let carry = get_local_wire(self.gate.wire_ith_carry(self.i));

        let output = addends.iter().fold(F::ZERO, |x, &y| x + y) + carry;
        let mut output_u64 = output.to_canonical_u64();

        let output_high_u64 = output_u64 >> 32;
        let output_low_u64 = output_u64 & ((1 << 32) - 1);

        let output_high = F::from_canonical_u64(output_high_u64);
        let output_low = F::from_canonical_u64(output_low_u64);

        let output_high_wire = local_wire(self.gate.wire_ith_output_high_half(self.i));
        let output_low_wire = local_wire(self.gate.wire_ith_output_low_half(self.i));

        out_buffer.set_wire(output_high_wire, output_high);
        out_buffer.set_wire(output_low_wire, output_low);

        let num_limbs = U32AddManyGate::<F, D>::num_limbs();
        let limb_base = 1 << U32AddManyGate::<F, D>::limb_bits();
        let output_limbs_u64 = unfold((), move |_| {
            let ret = output_u64 % limb_base;
            output_u64 /= limb_base;
            Some(ret)
        })
        .take(num_limbs);
        let output_limbs_f = output_limbs_u64.map(F::from_canonical_u64);

        for (j, output_limb) in output_limbs_f.enumerate() {
            let wire = local_wire(self.gate.wire_ith_output_jth_limb(self.i, j));
            out_buffer.set_wire(wire, output_limb);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use rand::Rng;

    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gates::add_many_u32::U32AddManyGate;
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::hash::hash_types::HashOut;
    use crate::plonk::vars::EvaluationVars;

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
        test_eval_fns::<GoldilocksField, _, 4>(U32AddManyGate::<GoldilocksField, 4> {
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
        const NUM_ADDENDS: usize = 4;
        const NUM_U32_ADD_MANY_OPS: usize = 3;

        fn get_wires(
            addends: Vec<Vec<u64>>,
            carries: Vec<u64>,
        ) -> Vec<FF> {
            let mut v0 = Vec::new();
            let mut v1 = Vec::new();

            let limb_bits = U32AddManyGate::<F, D>::limb_bits();
            let num_limbs = U32AddManyGate::<F, D>::num_limbs();
            let limb_base = 1 << limb_bits;
            for op in 0..NUM_U32_ADD_MANY_OPS {
                let adds = &addends[op];
                let ca = carries[op];

                let mut output = adds.iter().sum::<u64>() + ca;
                let output_low = output & ((1 << 32) - 1);
                let output_high = output >> 32;

                let mut output_limbs = Vec::with_capacity(num_limbs);
                for _i in 0..num_limbs {
                    output_limbs.push(output % limb_base);
                    output /= limb_base;
                }
                let mut output_limbs_f: Vec<_> = output_limbs
                    .into_iter()
                    .map(F::from_canonical_u64)
                    .collect();

                for a in adds {
                    v0.push(F::from_canonical_u64(*a));
                }
                v0.push(F::from_canonical_u64(ca));
                v0.push(F::from_canonical_u64(output_low));
                v0.push(F::from_canonical_u64(output_high));
                v1.append(&mut output_limbs_f);
            }

            v0.iter()
                .chain(v1.iter())
                .map(|&x| x.into())
                .collect::<Vec<_>>()
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
