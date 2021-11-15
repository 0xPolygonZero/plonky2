use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::base_sum::BaseSumGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::ceil_div_usize;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Split the given integer into a list of wires, where each one represents a
    /// bit of the integer, with little-endian ordering.
    /// Verifies that the decomposition is correct by using `k` `BaseSum<2>` gates
    /// with `k` such that `k * num_routed_wires >= num_bits`.
    pub(crate) fn split_le(&mut self, integer: Target, num_bits: usize) -> Vec<BoolTarget> {
        if num_bits == 0 {
            return Vec::new();
        }
        let gate_type = BaseSumGate::<2>::new_from_config::<F>(&self.config);
        let k = ceil_div_usize(num_bits, gate_type.num_limbs);
        dbg!(num_bits, gate_type.num_limbs);
        let gates = (0..k)
            .map(|_| self.add_gate(gate_type, vec![]))
            .collect::<Vec<_>>();
        dbg!(&gates);

        let mut bits = Vec::with_capacity(num_bits);
        for &gate in &gates {
            let start_limbs = BaseSumGate::<2>::START_LIMBS;
            for limb_input in start_limbs..start_limbs + gate_type.num_limbs {
                // `new_unsafe` is safe here because BaseSumGate::<2> forces it to be in `{0, 1}`.
                bits.push(BoolTarget::new_unsafe(Target::wire(gate, limb_input)));
            }
        }
        for b in bits.drain(num_bits..) {
            self.assert_zero(b.target);
        }

        let zero = self.zero();
        let mut gatesi = gates.iter().rev();
        let mut acc = Target::wire(*gatesi.next().unwrap(), BaseSumGate::<2>::WIRE_SUM);
        for &gate in gatesi {
            let sum = Target::wire(gate, BaseSumGate::<2>::WIRE_SUM);
            acc = self.mul_const_add(F::from_canonical_usize(1 << gate_type.num_limbs), acc, sum);
        }
        self.connect(acc, integer);

        self.add_simple_generator(WireSplitGenerator {
            integer,
            gates,
            num_limbs: gate_type.num_limbs,
        });

        bits
    }
}

#[derive(Debug)]
struct SplitGenerator {
    integer: Target,
    bits: Vec<Target>,
}

impl<F: RichField> SimpleGenerator<F> for SplitGenerator {
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

impl<F: RichField> SimpleGenerator<F> for WireSplitGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.integer]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let mut integer_value = witness.get_target(self.integer).to_canonical_u64();

        for &gate in &self.gates {
            let sum = Target::wire(gate, BaseSumGate::<2>::WIRE_SUM);
            out_buffer.set_target(
                sum,
                F::from_canonical_u64(
                    integer_value & ((1u128 << self.num_limbs as u128) - 1u128) as u64,
                ),
            );
            if self.gates.len() > 1 {
                integer_value >>= self.num_limbs;
            }
        }

        // debug_assert_eq!(
        //     integer_value,
        //     0,
        //     "Integer too large to fit in {} many `BaseSumGate`s",
        //     self.gates.len()
        // );
    }
}

#[cfg(test)]
mod tests {
    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_mul_algebra() {
        type F = GoldilocksField;
        type FF = QuarticExtension<GoldilocksField>;
        const D: usize = 4;

        let config = CircuitConfig::standard_recursion_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant(F::from_canonical_u64(14743424468522423903));
        let bits = builder.split_le(x, 64);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common).unwrap();
    }
}
