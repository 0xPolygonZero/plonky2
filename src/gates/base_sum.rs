use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::CircuitConfig;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::{Gate, GateRef};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::plonk_common::{reduce_with_powers, reduce_with_powers_recursive};
use crate::target::Target;
use crate::vars::{EvaluationTargets, EvaluationVars};
use crate::wire::Wire;
use crate::witness::PartialWitness;
use std::ops::Range;

/// A gate which can sum base W limbs.
#[derive(Debug)]
pub struct BaseSumGate<const B: usize> {
    num_limbs: usize,
}

impl<const B: usize> BaseSumGate<B> {
    pub fn new<F: Extendable<D>, const D: usize>(num_limbs: usize) -> GateRef<F, D> {
        GateRef::new(BaseSumGate::<B> { num_limbs })
    }

    pub const WIRE_SUM: usize = 0;
    pub const WIRE_LIMBS_START: usize = 1;

    /// Returns the index of the `i`th limb wire.
    pub fn limbs(&self) -> Range<usize> {
        Self::WIRE_LIMBS_START..Self::WIRE_LIMBS_START + self.num_limbs
    }
}

impl<F: Extendable<D>, const D: usize, const B: usize> Gate<F, D> for BaseSumGate<B> {
    fn id(&self) -> String {
        format!("{:?} + Base: {}", self, B)
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let sum = vars.local_wires[Self::WIRE_SUM];
        let limbs = vars.local_wires[self.limbs()].to_vec();
        let computed_sum = reduce_with_powers(&limbs, F::Extension::from_canonical_usize(B));
        let mut constraints = vec![computed_sum - sum];
        for limb in limbs {
            constraints.push(
                (0..B)
                    .map(|i| limb - F::Extension::from_canonical_usize(i))
                    .product(),
            );
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
        let limbs = vars.local_wires[self.limbs()].to_vec();
        let computed_sum =
            reduce_with_powers_recursive(builder, &vars.local_wires[self.limbs()], base);
        let mut constraints = vec![builder.sub_extension(computed_sum, sum)];
        for limb in limbs {
            constraints.push({
                let mut acc = builder.one_extension();
                (0..B).for_each(|i| {
                    let it = builder.constant_extension(F::from_canonical_usize(i).into());
                    let diff = builder.sub_extension(limb, it);
                    acc = builder.mul_extension_naive(acc, diff);
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

    fn num_wires(&self) -> usize {
        self.num_limbs + 1
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        B
    }

    fn num_constraints(&self) -> usize {
        1 + self.num_limbs
    }
}

#[derive(Debug)]
pub struct BaseSplitGenerator<const B: usize> {
    gate_index: usize,
    num_limbs: usize,
}

impl<F: Field, const B: usize> SimpleGenerator<F> for BaseSplitGenerator<B> {
    fn dependencies(&self) -> Vec<Target> {
        vec![Target::Wire(Wire {
            gate: self.gate_index,
            input: BaseSumGate::<B>::WIRE_SUM,
        })]
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut sum_value = witness
            .get_target(Target::Wire(Wire {
                gate: self.gate_index,
                input: BaseSumGate::<B>::WIRE_SUM,
            }))
            .to_canonical_u64() as usize;
        let limbs = (BaseSumGate::<B>::WIRE_LIMBS_START
            ..BaseSumGate::<B>::WIRE_LIMBS_START + self.num_limbs)
            .map(|i| {
                Target::Wire(Wire {
                    gate: self.gate_index,
                    input: i,
                })
            });

        let mut result = PartialWitness::new();
        for b in limbs {
            let b_value = sum_value % B;
            result.set_target(b, F::from_canonical_usize(b_value));
            sum_value /= B;
        }

        debug_assert_eq!(
            sum_value, 0,
            "Integer too large to fit in given number of bits"
        );

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::gates::base_sum::BaseSumGate;
    use crate::gates::gate_testing::test_low_degree;

    #[test]
    fn low_degree() {
        test_low_degree(BaseSumGate::<6>::new::<CrandallField, 4>(11))
    }
}
