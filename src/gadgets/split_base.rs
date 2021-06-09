use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::base_sum::{BaseSplitGenerator, BaseSumGate};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::util::{ceil_div_usize, log2_strict};
use crate::wire::Wire;
use crate::witness::PartialWitness;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Split the given element into a list of targets, where each one represents a
    /// base-B limb of the element, with little-endian ordering.
    pub(crate) fn split_le_base<const B: usize>(&mut self, x: Target) -> Vec<Target> {
        let num_limbs = num_limbs_to_check(64, B);
        let gate = self.add_gate(BaseSumGate::<B>::new(num_limbs), vec![]);
        let sum = Target::wire(gate, BaseSumGate::<B>::WIRE_SUM);
        self.route(x, sum);

        Target::wires_from_range(
            gate,
            BaseSumGate::<B>::WIRE_LIMBS_START..BaseSumGate::<B>::WIRE_LIMBS_START + num_limbs,
        )
    }

    /// Asserts that `x`'s bit representation has at least `trailing_zeros` trailing zeros.
    pub(crate) fn assert_trailing_zeros<const B: usize>(&mut self, x: Target, trailing_zeros: u32) {
        let limbs = self.split_le_base::<B>(x);
        let num_limbs_to_check = num_limbs_to_check(trailing_zeros, B);
        assert!(
            num_limbs_to_check < self.config.num_routed_wires,
            "Not enough routed wires."
        );
        for i in 0..num_limbs_to_check {
            self.assert_zero(limbs[i]);
        }
    }

    pub(crate) fn reverse_bits<const B: usize>(&mut self, x: Target, num_limbs: usize) -> Target {
        let gate = self.add_gate(BaseSumGate::<B>::new(num_limbs), vec![]);
        let sum = Target::wire(gate, BaseSumGate::<B>::WIRE_SUM);
        self.route(x, sum);

        Target::wire(gate, BaseSumGate::<B>::WIRE_REVERSED_SUM)
    }
}

/// Returns `k` such that any number with `k` trailing zeros in base `base` has at least
/// `n` trailing zeros in base 2.
const fn num_limbs_to_check(n: u32, base: usize) -> usize {
    assert_eq!(base % 2, 0, "Base should be even.");
    ceil_div_usize(n as usize, base.trailing_zeros() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::fri::FriConfig;
    use crate::prover::PLONK_BLINDING;
    use crate::verifier::verify;
    use anyhow::Result;

    #[test]
    fn test_split_base() -> Result<()> {
        type F = CrandallField;
        let config = CircuitConfig {
            num_wires: 134,
            num_routed_wires: 12,
            security_bits: 128,
            rate_bits: 3,
            num_challenges: 3,
            fri_config: FriConfig {
                proof_of_work_bits: 1,
                rate_bits: 3,
                reduction_arity_bits: vec![1],
                num_query_rounds: 1,
                blinding: PLONK_BLINDING.to_vec(),
            },
        };
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let x = F::from_canonical_usize(0b10100000); // 160 =1120 in base 5.
        let xt = builder.constant(x);
        let limbs = builder.split_le_base::<5>(xt);
        assert_eq!(limbs.len(), 28); // 5^27 < 2^64 <= 5^28
        let zero = builder.zero();
        let one = builder.one();
        let two = builder.two();
        builder.assert_equal(limbs[0], zero);
        builder.assert_equal(limbs[1], two);
        builder.assert_equal(limbs[2], one);
        builder.assert_equal(limbs[3], one);

        builder.assert_trailing_zeros::<3>(xt, 4);
        builder.assert_trailing_zeros::<3>(xt, 5);
        builder.assert_trailing_zeros::<13>(xt, 5);
        let data = builder.build();

        let proof = data.prove(PartialWitness::new());
        verify(proof, &data.verifier_only, &data.common)
    }
}
