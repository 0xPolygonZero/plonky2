use std::borrow::Borrow;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::base_sum::BaseSumGate;
use crate::generator::{GeneratedValues, SimpleGenerator};
use crate::target::Target;
use crate::witness::PartialWitness;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Split the given element into a list of targets, where each one represents a
    /// base-B limb of the element, with little-endian ordering.
    pub(crate) fn split_le_base<const B: usize>(
        &mut self,
        x: Target,
        num_limbs: usize,
    ) -> Vec<Target> {
        let gate = self.add_gate(BaseSumGate::<B>::new(num_limbs), vec![]);
        let sum = Target::wire(gate, BaseSumGate::<B>::WIRE_SUM);
        self.route(x, sum);

        Target::wires_from_range(
            gate,
            BaseSumGate::<B>::START_LIMBS..BaseSumGate::<B>::START_LIMBS + num_limbs,
        )
    }

    /// Asserts that `x`'s big-endian bit representation has at least `leading_zeros` leading zeros.
    pub(crate) fn assert_leading_zeros(&mut self, x: Target, leading_zeros: u32) {
        self.range_check(x, (64 - leading_zeros) as usize);
    }

    pub(crate) fn reverse_limbs<const B: usize>(&mut self, x: Target, num_limbs: usize) -> Target {
        let gate = self.add_gate(BaseSumGate::<B>::new(num_limbs), vec![]);
        let sum = Target::wire(gate, BaseSumGate::<B>::WIRE_SUM);
        self.route(x, sum);

        Target::wire(gate, BaseSumGate::<B>::WIRE_REVERSED_SUM)
    }

    pub(crate) fn base_sum(
        &mut self,
        limbs: impl ExactSizeIterator<Item = impl Borrow<Target>> + Clone,
    ) -> Target {
        let num_limbs = limbs.len();
        debug_assert!(
            BaseSumGate::<2>::START_LIMBS + num_limbs <= self.config.num_routed_wires,
            "Not enough routed wires."
        );
        let gate_index = self.add_gate(BaseSumGate::<2>::new(num_limbs), vec![]);
        for (limb, wire) in limbs
            .clone()
            .zip(BaseSumGate::<2>::START_LIMBS..BaseSumGate::<2>::START_LIMBS + num_limbs)
        {
            self.route(*limb.borrow(), Target::wire(gate_index, wire));
        }

        self.add_generator(BaseSumGenerator::<2> {
            gate_index,
            limbs: limbs.map(|l| *l.borrow()).collect(),
        });

        Target::wire(gate_index, BaseSumGate::<2>::WIRE_SUM)
    }
}

#[derive(Debug)]
struct BaseSumGenerator<const B: usize> {
    gate_index: usize,
    limbs: Vec<Target>,
}

impl<F: Field, const B: usize> SimpleGenerator<F> for BaseSumGenerator<B> {
    fn dependencies(&self) -> Vec<Target> {
        self.limbs.clone()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> GeneratedValues<F> {
        let sum = self
            .limbs
            .iter()
            .map(|&t| witness.get_target(t))
            .rev()
            .fold(F::ZERO, |acc, limb| acc * F::from_canonical_usize(B) + limb);

        GeneratedValues::singleton_target(
            Target::wire(self.gate_index, BaseSumGate::<B>::WIRE_SUM),
            sum,
        )
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field::Field;
    use crate::verifier::verify;
    use crate::witness::PartialWitness;

    #[test]
    fn test_split_base() -> Result<()> {
        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let x = F::from_canonical_usize(0b110100000); // 416 = 1532 in base 6.
        let xt = builder.constant(x);
        let limbs = builder.split_le_base::<6>(xt, 24);
        let one = builder.one();
        let two = builder.two();
        let three = builder.constant(F::from_canonical_u64(3));
        let five = builder.constant(F::from_canonical_u64(5));
        builder.route(limbs[0], two);
        builder.route(limbs[1], three);
        builder.route(limbs[2], five);
        builder.route(limbs[3], one);
        let rev = builder.constant(F::from_canonical_u64(11));
        let revt = builder.reverse_limbs::<2>(xt, 9);
        builder.route(revt, rev);

        builder.assert_leading_zeros(xt, 64 - 9);
        let data = builder.build();

        let proof = data.prove(PartialWitness::new())?;

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_base_sum() -> Result<()> {
        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let n = thread_rng().gen_range(0, 1 << 10);
        let x = builder.constant(F::from_canonical_usize(n));

        let zero = builder.zero();
        let one = builder.one();

        let y = builder.base_sum(
            (0..10)
                .scan(n, |acc, _| {
                    let tmp = *acc % 2;
                    *acc /= 2;
                    Some(if tmp == 1 { one } else { zero })
                })
                .collect::<Vec<_>>()
                .iter(),
        );

        builder.assert_equal(x, y);

        let data = builder.build();

        let proof = data.prove(PartialWitness::new())?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
