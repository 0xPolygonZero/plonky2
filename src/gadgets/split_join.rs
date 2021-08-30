use crate::field::extension_field::Extendable;
use crate::field::field_types::Field;
use crate::gates::base_sum::BaseSumGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::ceil_div_usize;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Split the given integer into a list of wires, where each one represents a
    /// bit of the integer, with little-endian ordering.
    /// Verifies that the decomposition is correct by using `k` `BaseSum<2>` gates
    /// with `k` such that `k * num_routed_wires >= num_bits`.
    pub(crate) fn split_le(&mut self, integer: Target, num_bits: usize) -> Vec<BoolTarget> {
        if num_bits == 0 {
            return Vec::new();
        }
        let bits_per_gate = self.config.num_routed_wires - BaseSumGate::<2>::START_LIMBS;
        let k = ceil_div_usize(num_bits, bits_per_gate);
        let gates = (0..k)
            .map(|_| self.add_gate(BaseSumGate::<2>::new(bits_per_gate), vec![]))
            .collect::<Vec<_>>();

        let mut bits = Vec::with_capacity(num_bits);
        for &gate in &gates {
            let start_limbs = BaseSumGate::<2>::START_LIMBS;
            for limb_input in start_limbs..start_limbs + bits_per_gate {
                // `new_unsafe` is safe here because BaseSumGate::<2> forces it to be in `{0, 1}`.
                bits.push(BoolTarget::new_unsafe(Target::wire(gate, limb_input)));
            }
        }
        bits.drain(num_bits..);

        let zero = self.zero();
        let one = self.one();
        let mut acc = zero;
        for &gate in gates.iter().rev() {
            let sum = Target::wire(gate, BaseSumGate::<2>::WIRE_SUM);
            acc = self.arithmetic(
                F::from_canonical_usize(1 << bits_per_gate),
                acc,
                one,
                F::ONE,
                sum,
            );
        }
        self.connect(acc, integer);

        self.add_generator(WireSplitGenerator {
            integer,
            gates,
            num_limbs: bits_per_gate,
        });

        bits
    }
}

#[derive(Debug)]
struct SplitGenerator {
    integer: Target,
    bits: Vec<Target>,
}

impl<F: Field> SimpleGenerator<F> for SplitGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.integer]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let mut integer_value = witness.get_target(self.integer).to_canonical_u64();

        for &b in &self.bits {
            let b_value = integer_value & 1;
            out_buffer.set_target(b, F::from_canonical_u64(b_value));
            integer_value >>= 1;
        }

        debug_assert_eq!(
            integer_value, 0,
            "Integer too large to fit in given number of bits"
        );
    }
}

#[derive(Debug)]
struct WireSplitGenerator {
    integer: Target,
    gates: Vec<usize>,
    num_limbs: usize,
}

impl<F: Field> SimpleGenerator<F> for WireSplitGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.integer]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let mut integer_value = witness.get_target(self.integer).to_canonical_u64();

        for &gate in &self.gates {
            let sum = Target::wire(gate, BaseSumGate::<2>::WIRE_SUM);
            out_buffer.set_target(
                sum,
                F::from_canonical_u64(integer_value & ((1 << self.num_limbs) - 1)),
            );
            integer_value >>= self.num_limbs;
        }

        debug_assert_eq!(
            integer_value,
            0,
            "Integer too large to fit in {} many `BaseSumGate`s",
            self.gates.len()
        );
    }
}
