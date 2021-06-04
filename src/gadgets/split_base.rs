use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::base_sum::{BaseSplitGenerator, BaseSumGate};
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::util::ceil_div_usize;
use crate::wire::Wire;
use crate::witness::PartialWitness;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Split the given element into a list of 11 targets, where each one represents a
    /// base-64 limb of the element, with little-endian ordering.
    pub(crate) fn split_le_base64(&mut self, x: Target) -> Vec<Target> {
        let gate = self.add_gate(BaseSumGate::<64>::new(11), vec![]);
        let sum = Target::Wire(Wire {
            gate,
            input: BaseSumGate::<64>::WIRE_SUM,
        });
        self.route(x, sum);
        (BaseSumGate::<64>::WIRE_LIMBS_START..BaseSumGate::<64>::WIRE_LIMBS_START + 11)
            .map(|i| Target::Wire(Wire { gate, input: i }))
            .collect()
    }

    /// Asserts that `x`'s bit representation has at least `trailing_zeros` trailing zeros.
    pub(crate) fn assert_trailing_zeros(&mut self, x: Target, trailing_zeros: u32) {
        let limbs = self.split_le_base64(x);
        for i in 0..ceil_div_usize(trailing_zeros as usize, 6) {
            self.assert_zero(limbs[i]);
        }
    }
}
