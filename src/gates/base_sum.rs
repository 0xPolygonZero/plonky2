use std::ops::Range;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::plonk_common::{reduce_with_powers, reduce_with_powers_recursive};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use crate::witness::PartialWitness;

/// A gate which can decompose a number into base B little-endian limbs,
/// and compute the limb-reversed (i.e. big-endian) sum.
#[derive(Clone, Debug)]
pub struct BaseSumGate<const B: usize> {
    num_limbs: usize,
}

impl<const B: usize> BaseSumGate<B> {
    pub fn new(num_limbs: usize) -> Self {
        Self { num_limbs }
    }

    pub const WIRE_SUM: usize = 0;
    pub const WIRE_REVERSED_SUM: usize = 1;
    pub const START_LIMBS: usize = 2;

    /// Returns the index of the `i`th limb wire.
    pub fn limbs(&self) -> Range<usize> {
        Self::START_LIMBS..Self::START_LIMBS + self.num_limbs
    }
}

impl<F: Extendable<D>, const D: usize, const B: usize> Gate<F, D> for BaseSumGate<B> {
    fn id(&self) -> String {
        format!("{:?} + Base: {}", self, B)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let sum = vars.local_wires[Self::WIRE_SUM];
        let reversed_sum = vars.local_wires[Self::WIRE_REVERSED_SUM];
        let mut limbs = vars.local_wires[self.limbs()].to_vec();
        let computed_sum = reduce_with_powers(&limbs, F::Extension::from_canonical_usize(B));
        limbs.reverse();
        let computed_reversed_sum =
            reduce_with_powers(&limbs, F::Extension::from_canonical_usize(B));
        let mut constraints = vec![computed_sum - sum, computed_reversed_sum - reversed_sum];
        for limb in limbs {
            constraints.push(
                (0..B)
                    .map(|i| limb - F::Extension::from_canonical_usize(i))
                    .product(),
            );
        }
        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let sum = vars.local_wires[Self::WIRE_SUM];
        let reversed_sum = vars.local_wires[Self::WIRE_REVERSED_SUM];
        let mut limbs = vars.local_wires[self.limbs()].to_vec();
        let computed_sum = reduce_with_powers(&limbs, F::from_canonical_usize(B));
        limbs.reverse();
        let computed_reversed_sum = reduce_with_powers(&limbs, F::from_canonical_usize(B));
        let mut constraints = vec![computed_sum - sum, computed_reversed_sum - reversed_sum];
        for limb in limbs {
            constraints.push((0..B).map(|i| limb - F::from_canonical_usize(i)).product());
        }
        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        let base = builder.constant(F::from_canonical_usize(B));
        let sum = vars.local_wires[Self::WIRE_SUM];
        let reversed_sum = vars.local_wires[Self::WIRE_REVERSED_SUM];
        let mut limbs = vars.local_wires[self.limbs()].to_vec();
        let computed_sum = reduce_with_powers_recursive(builder, &limbs, base);
        limbs.reverse();
        let computed_reversed_sum = reduce_with_powers_recursive(builder, &limbs, base);
        let mut constraints = vec![
            builder.sub_extension(computed_sum, sum),
            builder.sub_extension(computed_reversed_sum, reversed_sum),
        ];
        for limb in limbs {
            constraints.push({
                let mut acc = builder.one_extension();
                (0..B).for_each(|i| {
                    let it = builder.constant_extension(F::from_canonical_usize(i).into());
                    let diff = builder.sub_extension(limb, it);
                    acc = builder.mul_extension(acc, diff);
                });
                acc
            });
        }
        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = BaseSplitGenerator::<B> {
            gate_index,
            num_limbs: self.num_limbs,
        };
        vec![Box::new(gen)]
    }

    // 2 for the sum and reversed sum, then `num_limbs` for the limbs.
    fn num_wires(&self) -> usize {
        self.num_limbs + 2
    }

    fn num_constants(&self) -> usize {
        0
    }

    // Bounded by the range-check (x-0)*(x-1)*...*(x-B+1).
    fn degree(&self) -> usize {
        B
    }

    // 2 for checking the sum and reversed sum, then `num_limbs` for range-checking the limbs.
    fn num_constraints(&self) -> usize {
        2 + self.num_limbs
    }
}

#[derive(Debug)]
pub struct BaseSplitGenerator<const B: usize> {
    gate_index: usize,
    num_limbs: usize,
}

impl<F: Field, const B: usize> SimpleGenerator<F> for BaseSplitGenerator<B> {
    fn dependencies(&self) -> Vec<Target> {
        vec![Target::wire(self.gate_index, BaseSumGate::<B>::WIRE_SUM)]
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> GeneratedValues<F> {
        let sum_value = witness
            .get_target(Target::wire(self.gate_index, BaseSumGate::<B>::WIRE_SUM))
            .to_canonical_u64() as usize;
        debug_assert_eq!(
            (0..self.num_limbs).fold(sum_value, |acc, _| acc / B),
            0,
            "Integer too large to fit in given number of limbs"
        );

        let limbs = (BaseSumGate::<B>::START_LIMBS..BaseSumGate::<B>::START_LIMBS + self.num_limbs)
            .map(|i| Target::wire(self.gate_index, i));
        let limbs_value = (0..self.num_limbs)
            .scan(sum_value, |acc, _| {
                let tmp = *acc % B;
                *acc /= B;
                Some(F::from_canonical_usize(tmp))
            })
            .collect::<Vec<_>>();

        let b_field = F::from_canonical_usize(B);
        let reversed_sum = limbs_value
            .iter()
            .fold(F::ZERO, |acc, &x| acc * b_field + x);

        let mut result = GeneratedValues::with_capacity(self.num_limbs + 1);
        result.set_target(
            Target::wire(self.gate_index, BaseSumGate::<B>::WIRE_REVERSED_SUM),
            reversed_sum,
        );
        for (b, b_value) in limbs.zip(limbs_value) {
            result.set_target(b, b_value);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::gates::base_sum::BaseSumGate;
    use crate::gates::gate::GateRef;
    use crate::gates::gate_testing::test_low_degree;

    #[test]
    fn low_degree() {
        test_low_degree::<CrandallField, _, 4>(BaseSumGate::<6>::new(11))
    }
}
