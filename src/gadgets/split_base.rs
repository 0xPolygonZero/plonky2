use std::borrow::Borrow;

use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gates::base_sum::BaseSumGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Split the given element into a list of targets, where each one represents a
    /// base-B limb of the element, with little-endian ordering.
    pub fn split_le_base<const B: usize>(&mut self, x: Target, num_limbs: usize) -> Vec<Target> {
        let gate_type = BaseSumGate::<B>::new(num_limbs);
        let gate = self.add_gate(gate_type, vec![]);
        let sum = Target::wire(gate, BaseSumGate::<B>::WIRE_SUM);
        self.connect(x, sum);

        Target::wires_from_range(gate, gate_type.limbs())
    }

    /// Asserts that `x`'s big-endian bit representation has at least `leading_zeros` leading zeros.
    pub(crate) fn assert_leading_zeros(&mut self, x: Target, leading_zeros: u32) {
        self.range_check(x, (64 - leading_zeros) as usize);
    }

    /// Takes an iterator of bits `(b_i)` and returns `sum b_i * 2^i`, i.e.,
    /// the number with little-endian bit representation given by `bits`.
    pub(crate) fn le_sum(
        &mut self,
        bits: impl ExactSizeIterator<Item = impl Borrow<BoolTarget>> + Clone,
    ) -> Target {
        let num_bits = bits.len();
        if num_bits == 0 {
            return self.zero();
        } else if num_bits == 1 {
            let mut bits = bits;
            return bits.next().unwrap().borrow().target;
        } else if num_bits == 2 {
            let two = self.two();
            let mut bits = bits;
            let b0 = bits.next().unwrap().borrow().target;
            let b1 = bits.next().unwrap().borrow().target;
            return self.mul_add(two, b1, b0);
        }
        debug_assert!(
            BaseSumGate::<2>::START_LIMBS + num_bits <= self.config.num_routed_wires,
            "Not enough routed wires."
        );
        let gate_type = BaseSumGate::<2>::new_from_config::<F>(&self.config);
        let gate_index = self.add_gate(gate_type, vec![]);
        for (limb, wire) in bits
            .clone()
            .zip(BaseSumGate::<2>::START_LIMBS..BaseSumGate::<2>::START_LIMBS + num_bits)
        {
            self.connect(limb.borrow().target, Target::wire(gate_index, wire));
        }
        for l in gate_type.limbs().skip(num_bits) {
            self.assert_zero(Target::wire(gate_index, l));
        }

        self.add_simple_generator(BaseSumGenerator::<2> {
            gate_index,
            limbs: bits.map(|l| *l.borrow()).collect(),
        });

        Target::wire(gate_index, BaseSumGate::<2>::WIRE_SUM)
    }
}

#[derive(Debug)]
struct BaseSumGenerator<const B: usize> {
    gate_index: usize,
    limbs: Vec<BoolTarget>,
}

impl<F: Field, const B: usize> SimpleGenerator<F> for BaseSumGenerator<B> {
    fn dependencies(&self) -> Vec<Target> {
        self.limbs.iter().map(|b| b.target).collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let sum = self
            .limbs
            .iter()
            .map(|&t| witness.get_bool_target(t))
            .rev()
            .fold(F::ZERO, |acc, limb| {
                acc * F::from_canonical_usize(B) + F::from_bool(limb)
            });

        out_buffer.set_target(
            Target::wire(self.gate_index, BaseSumGate::<B>::WIRE_SUM),
            sum,
        );
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::{thread_rng, Rng};

    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_split_base() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let x = F::from_canonical_usize(0b110100000); // 416 = 1532 in base 6.
        let xt = builder.constant(x);
        let limbs = builder.split_le_base::<6>(xt, 24);
        let one = builder.one();
        let two = builder.two();
        let three = builder.constant(F::from_canonical_u64(3));
        let five = builder.constant(F::from_canonical_u64(5));
        builder.connect(limbs[0], two);
        builder.connect(limbs[1], three);
        builder.connect(limbs[2], five);
        builder.connect(limbs[3], one);

        builder.assert_leading_zeros(xt, 64 - 9);
        let data = builder.build::<C>();

        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_base_sum() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type FF = <C as GenericConfig<D>>::FE;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let n = thread_rng().gen_range(0..(1 << 10));
        let x = builder.constant(F::from_canonical_usize(n));

        let zero = builder._false();
        let one = builder._true();

        let y = builder.le_sum(
            (0..10)
                .scan(n, |acc, _| {
                    let tmp = *acc % 2;
                    *acc /= 2;
                    Some(if tmp == 1 { one } else { zero })
                })
                .collect::<Vec<_>>()
                .iter(),
        );

        builder.connect(x, y);

        let data = builder.build::<C>();

        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
