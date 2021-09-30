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
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Number of arithmetic operations performed by an arithmetic gate.
pub const NUM_U32_ARITHMETIC_OPS: usize = 3;

/// A gate to perform a basic mul-add on 32-bit values (we assume they are range-checked beforehand).
#[derive(Debug)]
pub struct U32ArithmeticGate<F: RichField + Extendable<D>, const D: usize> {
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> U32ArithmeticGate<F, D> {
    pub fn wire_ith_multiplicand_0(i: usize) -> usize {
        debug_assert!(i < NUM_U32_ARITHMETIC_OPS);
        5 * i
    }
    pub fn wire_ith_multiplicand_1(i: usize) -> usize {
        debug_assert!(i < NUM_U32_ARITHMETIC_OPS);
        5 * i + 1
    }
    pub fn wire_ith_addend(i: usize) -> usize {
        debug_assert!(i < NUM_U32_ARITHMETIC_OPS);
        5 * i + 2
    }

    pub fn wire_ith_output_low_half(i: usize) -> usize {
        debug_assert!(i < NUM_U32_ARITHMETIC_OPS);
        5 * i + 3
    }
    pub fn wire_ith_output_high_half(i: usize) -> usize {
        debug_assert!(i < NUM_U32_ARITHMETIC_OPS);
        5 * i + 4
    }

    pub fn limb_bits() -> usize {
        2
    }
    pub fn num_limbs() -> usize {
        64 / Self::limb_bits()
    }

    pub fn wire_ith_output_jth_limb(i: usize, j: usize) -> usize {
        debug_assert!(i < NUM_U32_ARITHMETIC_OPS);
        debug_assert!(j < Self::num_limbs());
        5 * NUM_U32_ARITHMETIC_OPS + Self::num_limbs() * i + j
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for U32ArithmeticGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..NUM_U32_ARITHMETIC_OPS {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];

            let computed_output = multiplicand_0 * multiplicand_1 + addend;

            let output_low = vars.local_wires[Self::wire_ith_output_low_half(i)];
            let output_high = vars.local_wires[Self::wire_ith_output_high_half(i)];

            let base = F::Extension::from_canonical_u64(1 << 32u64);
            let combined_output = output_high * base + output_low;

            constraints.push(combined_output - computed_output);

            let mut combined_low_limbs = F::Extension::ZERO;
            let mut combined_high_limbs = F::Extension::ZERO;
            let midpoint = Self::num_limbs() / 2;
            for j in 0..Self::num_limbs() {
                let this_limb = vars.local_wires[Self::wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::Extension::from_canonical_usize(x))
                    .product();
                constraints.push(product);

                if j < midpoint {
                    let base = F::Extension::from_canonical_u64(1u64 << (j * Self::limb_bits()));
                    combined_low_limbs += base * this_limb;
                } else {
                    let base = F::Extension::from_canonical_u64(
                        1u64 << ((j - midpoint) * Self::limb_bits()),
                    );
                    combined_high_limbs += base * this_limb;
                }
            }
            constraints.push(combined_low_limbs - output_low);
            constraints.push(combined_high_limbs - output_high);
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..NUM_U32_ARITHMETIC_OPS {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];

            let computed_output = multiplicand_0 * multiplicand_1 + addend;

            let output_low = vars.local_wires[Self::wire_ith_output_low_half(i)];
            let output_high = vars.local_wires[Self::wire_ith_output_high_half(i)];

            let base = F::from_canonical_u64(1 << 32u64);
            let combined_output = output_high * base + output_low;

            constraints.push(combined_output - computed_output);

            let mut combined_low_limbs = F::ZERO;
            let mut combined_high_limbs = F::ZERO;
            let midpoint = Self::num_limbs() / 2;
            for j in 0..Self::num_limbs() {
                let this_limb = vars.local_wires[Self::wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::from_canonical_usize(x))
                    .product();
                constraints.push(product);

                if j < midpoint {
                    let base = F::from_canonical_u64(1u64 << (j * Self::limb_bits()));
                    combined_low_limbs += base * this_limb;
                } else {
                    let base = F::from_canonical_u64(1u64 << ((j - midpoint) * Self::limb_bits()));
                    combined_high_limbs += base * this_limb;
                }
            }
            constraints.push(combined_low_limbs - output_low);
            constraints.push(combined_high_limbs - output_high);
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        for i in 0..NUM_U32_ARITHMETIC_OPS {
            let multiplicand_0 = vars.local_wires[Self::wire_ith_multiplicand_0(i)];
            let multiplicand_1 = vars.local_wires[Self::wire_ith_multiplicand_1(i)];
            let addend = vars.local_wires[Self::wire_ith_addend(i)];

            let computed_output = builder.mul_add_extension(multiplicand_0, multiplicand_1, addend);

            let output_low = vars.local_wires[Self::wire_ith_output_low_half(i)];
            let output_high = vars.local_wires[Self::wire_ith_output_high_half(i)];

            let base: F::Extension = F::from_canonical_u64(1 << 32u64).into();
            let base_target = builder.constant_extension(base);
            let combined_output = builder.mul_add_extension(output_high, base_target, output_low);

            constraints.push(builder.sub_extension(combined_output, computed_output));

            let mut combined_low_limbs = builder.zero_extension();
            let mut combined_high_limbs = builder.zero_extension();
            let midpoint = Self::num_limbs() / 2;
            for j in 0..Self::num_limbs() {
                let this_limb = vars.local_wires[Self::wire_ith_output_jth_limb(i, j)];
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
                    let base = builder.constant_extension(F::Extension::from_canonical_u64(
                        1u64 << (j * Self::limb_bits()),
                    ));
                    combined_low_limbs =
                        builder.mul_add_extension(base, this_limb, combined_low_limbs);
                } else {
                    let base = builder.constant_extension(F::Extension::from_canonical_u64(
                        1u64 << ((j - midpoint) * Self::limb_bits()),
                    ));
                    combined_high_limbs =
                        builder.mul_add_extension(base, this_limb, combined_high_limbs);
                }
            }

            constraints.push(builder.sub_extension(combined_low_limbs, output_low));
            constraints.push(builder.sub_extension(combined_high_limbs, output_high));
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..NUM_U32_ARITHMETIC_OPS)
            .map(|i| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    U32ArithmeticGenerator {
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
        NUM_U32_ARITHMETIC_OPS * (5 + Self::num_limbs())
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1 << Self::limb_bits()
    }

    fn num_constraints(&self) -> usize {
        NUM_U32_ARITHMETIC_OPS * (3 + Self::num_limbs())
    }
}

#[derive(Clone, Debug)]
struct U32ArithmeticGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    i: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for U32ArithmeticGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let mut deps = Vec::with_capacity(3);
        deps.push(local_target(
            U32ArithmeticGate::<F, D>::wire_ith_multiplicand_0(self.i),
        ));
        deps.push(local_target(
            U32ArithmeticGate::<F, D>::wire_ith_multiplicand_1(self.i),
        ));
        deps.push(local_target(U32ArithmeticGate::<F, D>::wire_ith_addend(
            self.i,
        )));
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let multiplicand_0 =
            get_local_wire(U32ArithmeticGate::<F, D>::wire_ith_multiplicand_0(self.i));
        let multiplicand_1 =
            get_local_wire(U32ArithmeticGate::<F, D>::wire_ith_multiplicand_1(self.i));
        let addend = get_local_wire(U32ArithmeticGate::<F, D>::wire_ith_addend(self.i));

        let output = multiplicand_0 * multiplicand_1 + addend;
        let mut output_u64 = output.to_canonical_u64();

        let output_high_u64 = output_u64 >> 32;
        let output_low_u64 = output_u64 & ((1 << 32) - 1);

        let output_high = F::from_canonical_u64(output_high_u64);
        let output_low = F::from_canonical_u64(output_low_u64);

        let output_high_wire =
            local_wire(U32ArithmeticGate::<F, D>::wire_ith_output_high_half(self.i));
        let output_low_wire =
            local_wire(U32ArithmeticGate::<F, D>::wire_ith_output_low_half(self.i));

        out_buffer.set_wire(output_high_wire, output_high);
        out_buffer.set_wire(output_low_wire, output_low);

        let limb_base = 1 << U32ArithmeticGate::<F, D>::limb_bits();
        let output_limbs_u64: Vec<_> = unfold((), move |_| {
            if output_u64 == 0 {
                return None;
            }
            let ret = output_u64 % limb_base;
            output_u64 /= limb_base;
            Some(ret)
        })
        .collect();
        let output_limbs_F: Vec<_> = output_limbs_u64
            .iter()
            .cloned()
            .map(F::from_canonical_u64)
            .collect();

        for j in 0..output_limbs_F.len() {
            let wire = local_wire(U32ArithmeticGate::<F, D>::wire_ith_output_jth_limb(
                self.i, j,
            ));
            out_buffer.set_wire(wire, output_limbs_F[j]);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use itertools::{izip, unfold};
    use rand::Rng;

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::field_types::Field;
    use crate::gates::arithmetic_u32::{NUM_U32_ARITHMETIC_OPS, U32ArithmeticGate};
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::hash::hash_types::HashOut;
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(U32ArithmeticGate::<CrandallField, 4> {
            _phantom: PhantomData,
        })
    }

    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<CrandallField, _, 4>(U32ArithmeticGate::<CrandallField, 4> {
            _phantom: PhantomData,
        })
    }

    #[test]
    fn test_gate_constraint() {
        type F = CrandallField;
        type FF = QuarticExtension<CrandallField>;
        const D: usize = 4;

        fn get_wires(multiplicands_0: Vec<u64>, multiplicands_1: Vec<u64>, addends: Vec<u64>)  -> Vec<FF> {
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

                let mut output_limbs = Vec::with_capacity(num_limbs);
                for i in 0..num_limbs {
                    output_limbs.push(output % limb_base);
                    output /= limb_base;
                }
                let mut output_limbs_F: Vec<_> = output_limbs.iter().cloned().map(F::from_canonical_u64).collect();

                v0.push(F::from_canonical_u64(m0));
                v0.push(F::from_canonical_u64(m1));
                v0.push(F::from_canonical_u64(a));
                v0.push(F::from_canonical_u64(output_low));
                v0.push(F::from_canonical_u64(output_high));
                v1.append(&mut output_limbs_F);
            }

            v0.iter().chain(v1.iter()).map(|&x| x.into()).collect::<Vec<_>>()
        }

        let mut rng = rand::thread_rng();
        let multiplicands_0: Vec<_> = (0..NUM_U32_ARITHMETIC_OPS).map(|_| rng.gen::<u32>() as u64).collect();
        let multiplicands_1: Vec<_> = (0..NUM_U32_ARITHMETIC_OPS).map(|_| rng.gen::<u32>() as u64).collect();
        let addends: Vec<_> = (0..NUM_U32_ARITHMETIC_OPS).map(|_| rng.gen::<u32>() as u64).collect();

        let gate = U32ArithmeticGate::<F, D> {
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(multiplicands_0, multiplicands_1, addends),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
