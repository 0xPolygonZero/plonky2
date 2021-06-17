use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::gates::base_sum::BaseSumGate;
use crate::target::Target;
use crate::util::ceil_div_usize;

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

    pub(crate) fn reverse_bits<const B: usize>(&mut self, x: Target, num_limbs: usize) -> Target {
        let gate = self.add_gate(BaseSumGate::<B>::new(num_limbs), vec![]);
        let sum = Target::wire(gate, BaseSumGate::<B>::WIRE_SUM);
        self.route(x, sum);

        Target::wire(gate, BaseSumGate::<B>::WIRE_REVERSED_SUM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field::Field;
    use crate::witness::PartialWitness;

    #[test]
    fn test_split_base() {
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
        let revt = builder.reverse_bits::<2>(xt, 9);
        builder.route(revt, rev);

        builder.assert_leading_zeros(xt, 64 - 9);
        let data = builder.build();

        let proof = data.prove(PartialWitness::new());
    }
}
