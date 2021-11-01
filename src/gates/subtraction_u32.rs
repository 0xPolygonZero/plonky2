use std::marker::PhantomData;

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

/// Maximum number of subtractions operations performed by a single gate.
pub const NUM_U32_SUBTRACTION_OPS: usize = 3;

/// A gate to perform a subtraction on 32-bit limbs: given `x`, `y`, and `borrow`, it returns
/// the result `x - y - borrow` and, if this underflows, a new `borrow`. Inputs are not range-checked.
#[derive(Clone, Debug)]
pub struct U32SubtractionGate<F: RichField + Extendable<D>, const D: usize> {
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> U32SubtractionGate<F, D> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    pub fn wire_ith_input_x(i: usize) -> usize {
        debug_assert!(i < NUM_U32_SUBTRACTION_OPS);
        5 * i
    }
    pub fn wire_ith_input_y(i: usize) -> usize {
        debug_assert!(i < NUM_U32_SUBTRACTION_OPS);
        5 * i + 1
    }
    pub fn wire_ith_input_borrow(i: usize) -> usize {
        debug_assert!(i < NUM_U32_SUBTRACTION_OPS);
        5 * i + 2
    }

    pub fn wire_ith_output_result(i: usize) -> usize {
        debug_assert!(i < NUM_U32_SUBTRACTION_OPS);
        5 * i + 3
    }
    pub fn wire_ith_output_borrow(i: usize) -> usize {
        debug_assert!(i < NUM_U32_SUBTRACTION_OPS);
        5 * i + 4
    }

    // We have limbs ony for the first half of the output.
    pub fn limb_bits() -> usize {
        2
    }
    pub fn num_limbs() -> usize {
        32 / Self::limb_bits()
    }

    pub fn wire_ith_output_jth_limb(i: usize, j: usize) -> usize {
        debug_assert!(i < NUM_U32_SUBTRACTION_OPS);
        debug_assert!(j < Self::num_limbs());
        5 * NUM_U32_SUBTRACTION_OPS + Self::num_limbs() * i + j
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for U32SubtractionGate<F, D> {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..NUM_U32_SUBTRACTION_OPS {
            let input_x = vars.local_wires[Self::wire_ith_input_x(i)];
            let input_y = vars.local_wires[Self::wire_ith_input_y(i)];
            let input_borrow = vars.local_wires[Self::wire_ith_input_borrow(i)];

            let result_initial = input_x - input_y - input_borrow;
            let base = F::Extension::from_canonical_u64(1 << 32u64);

            let output_result = vars.local_wires[Self::wire_ith_output_result(i)];
            let output_borrow = vars.local_wires[Self::wire_ith_output_borrow(i)];

            constraints.push(output_result - (result_initial + base * output_borrow));

            // Range-check output_result to be at most 32 bits.
            let mut combined_limbs = F::Extension::ZERO;
            let limb_base = F::Extension::from_canonical_u64(1u64 << Self::limb_bits());
            for j in (0..Self::num_limbs()).rev() {
                let this_limb = vars.local_wires[Self::wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::Extension::from_canonical_usize(x))
                    .product();
                constraints.push(product);

                combined_limbs = limb_base * combined_limbs + this_limb;
            }
            constraints.push(combined_limbs - output_result);

            // Range-check output_borrow to be one bit.
            constraints.push(output_borrow * (F::Extension::ONE - output_borrow));
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..NUM_U32_SUBTRACTION_OPS {
            let input_x = vars.local_wires[Self::wire_ith_input_x(i)];
            let input_y = vars.local_wires[Self::wire_ith_input_y(i)];
            let input_borrow = vars.local_wires[Self::wire_ith_input_borrow(i)];

            let result_initial = input_x - input_y - input_borrow;
            let base = F::from_canonical_u64(1 << 32u64);

            let output_result = vars.local_wires[Self::wire_ith_output_result(i)];
            let output_borrow = vars.local_wires[Self::wire_ith_output_borrow(i)];

            constraints.push(output_result - (result_initial + base * output_borrow));

            // Range-check output_result to be at most 32 bits.
            let mut combined_limbs = F::ZERO;
            let limb_base = F::from_canonical_u64(1u64 << Self::limb_bits());
            for j in (0..Self::num_limbs()).rev() {
                let this_limb = vars.local_wires[Self::wire_ith_output_jth_limb(i, j)];
                let max_limb = 1 << Self::limb_bits();
                let product = (0..max_limb)
                    .map(|x| this_limb - F::from_canonical_usize(x))
                    .product();
                constraints.push(product);

                combined_limbs = limb_base * combined_limbs + this_limb;
            }
            constraints.push(combined_limbs - output_result);

            // Range-check output_borrow to be one bit.
            constraints.push(output_borrow * (F::ONE - output_borrow));
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let mut constraints = Vec::with_capacity(self.num_constraints());
        for i in 0..NUM_U32_SUBTRACTION_OPS {
            let input_x = vars.local_wires[Self::wire_ith_input_x(i)];
            let input_y = vars.local_wires[Self::wire_ith_input_y(i)];
            let input_borrow = vars.local_wires[Self::wire_ith_input_borrow(i)];

            let diff = builder.sub_extension(input_x, input_y);
            let result_initial = builder.sub_extension(diff, input_borrow);
            let base = builder.constant_extension(F::Extension::from_canonical_u64(1 << 32u64));

            let output_result = vars.local_wires[Self::wire_ith_output_result(i)];
            let output_borrow = vars.local_wires[Self::wire_ith_output_borrow(i)];

            let computed_output = builder.mul_add_extension(base, output_borrow, result_initial);
            constraints.push(builder.sub_extension(output_result, computed_output));

            // Range-check output_result to be at most 32 bits.
            let mut combined_limbs = builder.zero_extension();
            let limb_base = builder
                .constant_extension(F::Extension::from_canonical_u64(1u64 << Self::limb_bits()));
            for j in (0..Self::num_limbs()).rev() {
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

                combined_limbs = builder.mul_add_extension(limb_base, combined_limbs, this_limb);
            }
            constraints.push(builder.sub_extension(combined_limbs, output_result));

            // Range-check output_borrow to be one bit.
            let one = builder.one_extension();
            let not_borrow = builder.sub_extension(one, output_borrow);
            constraints.push(builder.mul_extension(output_borrow, not_borrow));
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        (0..NUM_U32_SUBTRACTION_OPS)
            .map(|i| {
                let g: Box<dyn WitnessGenerator<F>> = Box::new(
                    U32SubtractionGenerator {
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
        NUM_U32_SUBTRACTION_OPS * (5 + Self::num_limbs())
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1 << Self::limb_bits()
    }

    fn num_constraints(&self) -> usize {
        NUM_U32_SUBTRACTION_OPS * (3 + Self::num_limbs())
    }
}

#[derive(Clone, Debug)]
struct U32SubtractionGenerator<F: RichField + Extendable<D>, const D: usize> {
    gate_index: usize,
    i: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for U32SubtractionGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let local_target = |input| Target::wire(self.gate_index, input);

        let mut deps = Vec::with_capacity(3);
        deps.push(local_target(U32SubtractionGate::<F, D>::wire_ith_input_x(
            self.i,
        )));
        deps.push(local_target(U32SubtractionGate::<F, D>::wire_ith_input_y(
            self.i,
        )));
        deps.push(local_target(
            U32SubtractionGate::<F, D>::wire_ith_input_borrow(self.i),
        ));
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let get_local_wire = |input| witness.get_wire(local_wire(input));

        let input_x = get_local_wire(U32SubtractionGate::<F, D>::wire_ith_input_x(self.i));
        let input_y = get_local_wire(U32SubtractionGate::<F, D>::wire_ith_input_y(self.i));
        let input_borrow =
            get_local_wire(U32SubtractionGate::<F, D>::wire_ith_input_borrow(self.i));

        let result_initial = input_x - input_y - input_borrow;
        let result_initial_u64 = result_initial.to_canonical_u64();
        let output_borrow = if result_initial_u64 > 1 << 32u64 {
            F::ONE
        } else {
            F::ZERO
        };

        let base = F::from_canonical_u64(1 << 32u64);
        let output_result = result_initial + base * output_borrow;

        let output_result_wire =
            local_wire(U32SubtractionGate::<F, D>::wire_ith_output_result(self.i));
        let output_borrow_wire =
            local_wire(U32SubtractionGate::<F, D>::wire_ith_output_borrow(self.i));

        out_buffer.set_wire(output_result_wire, output_result);
        out_buffer.set_wire(output_borrow_wire, output_borrow);

        let output_result_u64 = output_result.to_canonical_u64();

        let num_limbs = U32SubtractionGate::<F, D>::num_limbs();
        let limb_base = 1 << U32SubtractionGate::<F, D>::limb_bits();
        let output_limbs: Vec<_> = (0..num_limbs)
            .scan(output_result_u64, |acc, _| {
                let tmp = *acc % limb_base;
                *acc /= limb_base;
                Some(F::from_canonical_u64(tmp))
            })
            .collect();

        for j in 0..num_limbs {
            let wire = local_wire(U32SubtractionGate::<F, D>::wire_ith_output_jth_limb(
                self.i, j,
            ));
            out_buffer.set_wire(wire, output_limbs[j]);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;
    use rand::Rng;

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::field_types::{Field, PrimeField};
    use crate::gates::gate::Gate;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::subtraction_u32::{U32SubtractionGate, NUM_U32_SUBTRACTION_OPS};
    use crate::hash::hash_types::HashOut;
    use crate::plonk::vars::EvaluationVars;

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(U32SubtractionGate::<CrandallField, 4> {
            _phantom: PhantomData,
        })
    }

    #[test]
    fn eval_fns() -> Result<()> {
        test_eval_fns::<CrandallField, _, 4>(U32SubtractionGate::<CrandallField, 4> {
            _phantom: PhantomData,
        })
    }

    #[test]
    fn test_gate_constraint() {
        type F = CrandallField;
        type FF = QuarticExtension<CrandallField>;
        const D: usize = 4;

        fn get_wires(inputs_x: Vec<u64>, inputs_y: Vec<u64>, borrows: Vec<u64>) -> Vec<FF> {
            let mut v0 = Vec::new();
            let mut v1 = Vec::new();

            let limb_bits = U32SubtractionGate::<F, D>::limb_bits();
            let num_limbs = U32SubtractionGate::<F, D>::num_limbs();
            let limb_base = 1 << limb_bits;
            for c in 0..NUM_U32_SUBTRACTION_OPS {
                let input_x = F::from_canonical_u64(inputs_x[c]);
                let input_y = F::from_canonical_u64(inputs_y[c]);
                let input_borrow = F::from_canonical_u64(borrows[c]);

                let result_initial = input_x - input_y - input_borrow;
                let result_initial_u64 = result_initial.to_canonical_u64();
                let output_borrow = if result_initial_u64 > 1 << 32u64 {
                    F::ONE
                } else {
                    F::ZERO
                };

                let base = F::from_canonical_u64(1 << 32u64);
                let output_result = result_initial + base * output_borrow;

                let output_result_u64 = output_result.to_canonical_u64();

                let mut output_limbs: Vec<_> = (0..num_limbs)
                    .scan(output_result_u64, |acc, _| {
                        let tmp = *acc % limb_base;
                        *acc /= limb_base;
                        Some(F::from_canonical_u64(tmp))
                    })
                    .collect();

                v0.push(input_x);
                v0.push(input_y);
                v0.push(input_borrow);
                v0.push(output_result);
                v0.push(output_borrow);
                v1.append(&mut output_limbs);
            }

            v0.iter()
                .chain(v1.iter())
                .map(|&x| x.into())
                .collect::<Vec<_>>()
        }

        let mut rng = rand::thread_rng();
        let inputs_x = (0..NUM_U32_SUBTRACTION_OPS)
            .map(|_| rng.gen::<u32>() as u64)
            .collect();
        let inputs_y = (0..NUM_U32_SUBTRACTION_OPS)
            .map(|_| rng.gen::<u32>() as u64)
            .collect();
        let borrows = (0..NUM_U32_SUBTRACTION_OPS)
            .map(|_| (rng.gen::<u32>() % 2) as u64)
            .collect();

        let gate = U32SubtractionGate::<F, D> {
            _phantom: PhantomData,
        };

        let vars = EvaluationVars {
            local_constants: &[],
            local_wires: &get_wires(inputs_x, inputs_y, borrows),
            public_inputs_hash: &HashOut::rand(),
        };

        assert!(
            gate.eval_unfiltered(vars).iter().all(|x| x.is_zero()),
            "Gate constraints are not satisfied."
        );
    }
}
