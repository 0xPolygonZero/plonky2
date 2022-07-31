use std::marker::PhantomData;

use itertools::unfold;
use plonky2::gates::gate::Gate;
use plonky2::gates::packed_util::PackedEvaluableBase;
use plonky2::gates::util::StridedConstraintConsumer;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::wire::Wire;
use plonky2::iop::witness::{PartitionWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};
use plonky2_field::extension::Extendable;
use plonky2_field::packed::PackedField;
use plonky2_field::types::Field;

/// A gate to perform a basic mul-add on 32-bit values (we assume they are range-checked beforehand).
#[derive(Copy, Clone, Debug)]
pub struct U32ArithmeticGate<F: RichField + Extendable<D>, const D: usize> {
    pub num_ops: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> U32ArithmeticGate<F, D> {
    pub fn new_from_config(config: &CircuitConfig) -> Self {
        Self {
            num_ops: Self::num_ops(config),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn num_ops(config: &CircuitConfig) -> usize {
        let wires_per_op = Self::routed_wires_per_op() + Self::num_limbs();
        (config.num_wires / wires_per_op).min(config.num_routed_wires / Self::routed_wires_per_op())
    }

    pub fn wire_ith_multiplicand_0(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        Self::routed_wires_per_op() * i
    }
    pub fn wire_ith_multiplicand_1(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        Self::routed_wires_per_op() * i + 1
    }
    pub fn wire_ith_addend(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        Self::routed_wires_per_op() * i + 2
    }

    pub fn wire_ith_output_low_half(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        Self::routed_wires_per_op() * i + 3
    }

    pub fn wire_ith_output_high_half(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        Self::routed_wires_per_op() * i + 4
    }

    pub fn wire_ith_inverse(&self, i: usize) -> usize {
        debug_assert!(i < self.num_ops);
        Self::routed_wires_per_op() * i + 5
    }

    pub fn limb_bits() -> usize {
        2
    }
    pub fn num_limbs() -> usize {
        64 / Self::limb_bits()
    }
    pub fn routed_wires_per_op() -> usize {
        6
    }
    pub fn wire_ith_output_jth_limb(&self, i: usize, j: usize) -> usize {
        debug_assert!(i < self.num_ops);
        debug_assert!(j < Self::num_limbs());
        Self::routed_wires_per_op() * self.num_ops + Self::num_limbs() * i + j
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for U32ArithmeticGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.local_wires[self.wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[self.wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[self.wire_ith_addend(i)];

            let computed_output = multiplicand_0 * multiplicand_1 + addend;

            let output_low = vars.local_wires[self.wire_ith_output_low_half(i)];
            let output_high = vars.local_wires[self.wire_ith_output_high_half(i)];
            let inverse = vars.local_wires[self.wire_ith_inverse(i)];

            // Check canonicity of combined_output = output_high * 2^32 + output_low
            let combined_output = {
                let base = F::Extension::from_canonical_u64(1 << 32u64);
                let one = F::Extension::ONE;
                let u32_max = F::Extension::from_canonical_u32(u32::MAX);

                // This is zero if and only if the high limb is `u32::MAX`.
                // u32::MAX - output_high
                let diff = u32_max - output_high;
                // If this is zero, the diff is invertible, so the high limb is not `u32::MAX`.
                // inverse * diff - 1
                let hi_not_max = inverse * diff - one;
                // If this is zero, either the high limb is not `u32::MAX`, or the low limb is zero.
                // hi_not_max * limb_0_u32
                let hi_not_max_or_lo_zero = hi_not_max * output_low;

                constraints.push(hi_not_max_or_lo_zero);

                output_high * base + output_low
            };

            constraints.push(combined_output - computed_output);

            let mut combined_low_limbs = F::Extension::ZERO;
            let mut combined_high_limbs = F::Extension::ZERO;
            let midpoint = Self::num_limbs() / 2;
            let base = F::Extension::from_canonical_u64(1u64 << Self::limb_bits());
            for j in (0..Self::num_limbs()).rev() {
                let this_limb = vars.local_wires[self.wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::Extension::from_canonical_usize(x))
                    .product();
                constraints.push(product);

                if j < midpoint {
                    combined_low_limbs = base * combined_low_limbs + this_limb;
                } else {
                    combined_high_limbs = base * combined_high_limbs + this_limb;
                }
            }
            constraints.push(combined_low_limbs - output_low);
            constraints.push(combined_high_limbs - output_high);
        }

        constraints
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

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for i in 0..self.num_ops {
            let multiplicand_0 = vars.local_wires[self.wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[self.wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[self.wire_ith_addend(i)];

            let computed_output = builder.mul_add_extension(multiplicand_0, multiplicand_1, addend);

            let output_low = vars.local_wires[self.wire_ith_output_low_half(i)];
            let output_high = vars.local_wires[self.wire_ith_output_high_half(i)];
            let inverse = vars.local_wires[self.wire_ith_inverse(i)];

            // Check canonicity of combined_output = output_high * 2^32 + output_low
            let combined_output = {
                let base: F::Extension = F::from_canonical_u64(1 << 32u64).into();
                let base_target = builder.constant_extension(base);
                let one = builder.one_extension();
                let u32_max =
                    builder.constant_extension(F::Extension::from_canonical_u32(u32::MAX));

                // This is zero if and only if the high limb is `u32::MAX`.
                let diff = builder.sub_extension(u32_max, output_high);
                // If this is zero, the diff is invertible, so the high limb is not `u32::MAX`.
                let hi_not_max = builder.mul_sub_extension(inverse, diff, one);
                // If this is zero, either the high limb is not `u32::MAX`, or the low limb is zero.
                let hi_not_max_or_lo_zero = builder.mul_extension(hi_not_max, output_low);

                constraints.push(hi_not_max_or_lo_zero);

                builder.mul_add_extension(output_high, base_target, output_low)
            };

            constraints.push(builder.sub_extension(combined_output, computed_output));

            let mut combined_low_limbs = builder.zero_extension();
            let mut combined_high_limbs = builder.zero_extension();
            let midpoint = Self::num_limbs() / 2;
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

                if j < midpoint {
                    combined_low_limbs =
                        builder.mul_add_extension(base, combined_low_limbs, this_limb);
                } else {
                    combined_high_limbs =
                        builder.mul_add_extension(base, combined_high_limbs, this_limb);
                }
            }

            constraints.push(builder.sub_extension(combined_low_limbs, output_low));
            constraints.push(builder.sub_extension(combined_high_limbs, output_high));
        }

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..self.num_ops)
            .map(|i| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    U32ArithmeticGenerator {
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
        self.num_ops * (Self::routed_wires_per_op() + Self::num_limbs())
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1 << Self::limb_bits()
    }

    fn num_constraints(&self) -> usize {
        self.num_ops * (4 + Self::num_limbs())
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D>
    for U32ArithmeticGate<F, D>
{
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        vars: EvaluationVarsBasePacked<P>,
        mut yield_constr: StridedConstraintConsumer<P>,
    ) {
        for i in 0..self.num_ops {
            let multiplicand_0 = vars.local_wires[self.wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[self.wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[self.wire_ith_addend(i)];

            let computed_output = multiplicand_0 * multiplicand_1 + addend;

            let output_low = vars.local_wires[self.wire_ith_output_low_half(i)];
            let output_high = vars.local_wires[self.wire_ith_output_high_half(i)];
            let inverse = vars.local_wires[self.wire_ith_inverse(i)];

            let combined_output = {
                let base = P::from(F::from_canonical_u64(1 << 32u64));
                let one = P::ONES;
                let u32_max = P::from(F::from_canonical_u32(u32::MAX));

                // This is zero if and only if the high limb is `u32::MAX`.
                // u32::MAX - output_high
                let diff = u32_max - output_high;
                // If this is zero, the diff is invertible, so the high limb is not `u32::MAX`.
                // inverse * diff - 1
                let hi_not_max = inverse * diff - one;
                // If this is zero, either the high limb is not `u32::MAX`, or the low limb is zero.
                // hi_not_max * limb_0_u32
                let hi_not_max_or_lo_zero = hi_not_max * output_low;

                yield_constr.one(hi_not_max_or_lo_zero);

                output_high * base + output_low
            };

            yield_constr.one(combined_output - computed_output);

            let mut combined_low_limbs = P::ZEROS;
            let mut combined_high_limbs = P::ZEROS;
            let midpoint = Self::num_limbs() / 2;
            let base = F::from_canonical_u64(1u64 << Self::limb_bits());
            for j in (0..Self::num_limbs()).rev() {
                let this_limb = vars.local_wires[self.wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::from_canonical_usize(x))
                    .product();
                yield_constr.one(product);

                if j < midpoint {
                    combined_low_limbs = combined_low_limbs * base + this_limb;
                } else {
                    combined_high_limbs = combined_high_limbs * base + this_limb;
                }
            }
            yield_constr.one(combined_low_limbs - output_low);
            yield_constr.one(combined_high_limbs - output_high);
        }
    }
}

#[derive(Clone, Debug)]
struct U32ArithmeticGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate: U32ArithmeticGate<F, D>,
    row: usize,
    i: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for U32ArithmeticGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |column| Target::wire(self.row, column);

        vec![
            local_target(self.gate.wire_ith_multiplicand_0(self.i)),
            local_target(self.gate.wire_ith_multiplicand_1(self.i)),
            local_target(self.gate.wire_ith_addend(self.i)),
        ]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |column| Wire {
            row: self.row,
            column,
        };

        let get_local_wire = |column| witness.get_wire(local_wire(column));

        let multiplicand_0 = get_local_wire(self.gate.wire_ith_multiplicand_0(self.i));
        let multiplicand_1 = get_local_wire(self.gate.wire_ith_multiplicand_1(self.i));
        let addend = get_local_wire(self.gate.wire_ith_addend(self.i));

        let output = multiplicand_0 * multiplicand_1 + addend;
        let mut output_u64 = output.to_canonical_u64();

        let output_high_u64 = output_u64 >> 32;
        let output_low_u64 = output_u64 & ((1 << 32) - 1);

        let output_high = F::from_canonical_u64(output_high_u64);
        let output_low = F::from_canonical_u64(output_low_u64);

        let output_high_wire = local_wire(self.gate.wire_ith_output_high_half(self.i));
        let output_low_wire = local_wire(self.gate.wire_ith_output_low_half(self.i));

        out_buffer.set_wire(output_high_wire, output_high);
        out_buffer.set_wire(output_low_wire, output_low);

        let diff = u32::MAX as u64 - output_high_u64;
        let inverse = if diff == 0 {
            F::ZERO
        } else {
            F::from_canonical_u64(diff).inverse()
        };
        let inverse_wire = local_wire(self.gate.wire_ith_inverse(self.i));
        out_buffer.set_wire(inverse_wire, inverse);

        let num_limbs = U32ArithmeticGate::<F, D>::num_limbs();
        let limb_base = 1 << U32ArithmeticGate::<F, D>::limb_bits();
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
    use plonky2::gates::gate::Gate;
    use plonky2::gates::gate_testing::{test_eval_fns, test_low_degree};
    use plonky2::hash::hash_types::HashOut;
    use plonky2::hash::hash_types::RichField;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::plonk::vars::EvaluationVars;
    use plonky2_field::extension::Extendable;
    use plonky2_field::goldilocks_field::GoldilocksField;
    use plonky2_field::types::Field;
    use rand::Rng;

    use crate::gates::arithmetic_u32::U32ArithmeticGate;

    #[test]
    fn low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(U32ArithmeticGate::<GoldilocksField, 4> {
            num_ops: 3,
            _phantom: PhantomData,
        })
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        test_eval_fns::<F, C, _, D>(U32ArithmeticGate::<GoldilocksField, D> {
            num_ops: 3,
            _phantom: PhantomData,
        })
    }

    fn get_wires<
        F: RichField + Extendable<D>,
        FF: From<F>,
        const D: usize,
        const NUM_U32_ARITHMETIC_OPS: usize,
    >(
        multiplicands_0: Vec<u64>,
        multiplicands_1: Vec<u64>,
        addends: Vec<u64>,
    ) -> Vec<FF> {
        let mut v0 = Vec::new();
        let mut v1 = Vec::new();

        let limb_bits = U32ArithmeticGate::<F, D>::limb_bits();
        let num_limbs = U32ArithmeticGate::<F, D>::num_limbs();
        let limb_base = 1 << limb_bits;
        for c in 0..NUM_U32_ARITHMETIC_OPS {
            let m0 = multiplicands_0[c];
            let m1 = multiplicands_1[c];
            let a = addends[c];

            let mut output = m0 * m1 + a;
            let output_low = output & ((1 << 32) - 1);
            let output_high = output >> 32;
            let diff = u32::MAX as u64 - output_high;
            let inverse = if diff == 0 {
                F::ZERO
            } else {
                F::from_canonical_u64(diff).inverse()
            };

            let mut output_limbs = Vec::with_capacity(num_limbs);
            for _i in 0..num_limbs {
                output_limbs.push(output % limb_base);
                output /= limb_base;
            }
            let mut output_limbs_f: Vec<_> = output_limbs
                .into_iter()
                .map(F::from_canonical_u64)
                .collect();

            v0.push(F::from_canonical_u64(m0));
            v0.push(F::from_canonical_u64(m1));
            v0.push(F::from_noncanonical_u64(a));
            v0.push(F::from_canonical_u64(output_low));
            v0.push(F::from_canonical_u64(output_high));
            v0.push(inverse);
            v1.append(&mut output_limbs_f);
        }

        v0.iter().chain(v1.iter()).map(|&x| x.into()).collect()
    }

    #[test]
    fn test_gate_constraint() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;
        const NUM_U32_ARITHMETIC_OPS: usize = 3;

        let mut rng = rand::thread_rng();
        let multiplicands_0: Vec<_> = (0..NUM_U32_ARITHMETIC_OPS)
            .map(|_| rng.gen::<u32>() as u64)
            .collect();
        let multiplicands_1: Vec<_> = (0..NUM_U32_ARITHMETIC_OPS)
            .map(|_| rng.gen::<u32>() as u64)
            .collect();
        let addends: Vec<_> = (0..NUM_U32_ARITHMETIC_OPS)
            .map(|_| rng.gen::<u32>() as u64)
            .collect();

        let gate = U32ArithmeticGate::<F, D> {
            num_ops: NUM_U32_ARITHMETIC_OPS,
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires::<F, FF, D, NUM_U32_ARITHMETIC_OPS>(
                multiplicands_0,
                multiplicands_1,
                addends,
            ),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }

    #[test]
    fn test_canonicity() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;
        const NUM_U32_ARITHMETIC_OPS: usize = 3;

        let multiplicands_0 = vec![0; NUM_U32_ARITHMETIC_OPS];
        let multiplicands_1 = vec![0; NUM_U32_ARITHMETIC_OPS];
        // A non-canonical addend will produce a non-canonical output using
        // get_wires.
        let addends = vec![0xFFFFFFFF00000001; NUM_U32_ARITHMETIC_OPS];

        let gate = U32ArithmeticGate::<F, D> {
            num_ops: NUM_U32_ARITHMETIC_OPS,
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires::<F, FF, D, NUM_U32_ARITHMETIC_OPS>(
                multiplicands_0,
                multiplicands_1,
                addends,
            ),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            !gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Non-canonical output should not pass constraints."
        );
    }
}
