#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec, vec::Vec};
use core::borrow::Borrow;

use anyhow::Result;
use itertools::Itertools;

use crate::field::extension::Extendable;
use crate::gates::base_sum::BaseSumGate;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness, WitnessWrite};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::util::log_floor;
use crate::util::serialization::{Buffer, IoResult, Read, Write};

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
    pub fn le_sum(&mut self, bits: impl Iterator<Item = impl Borrow<BoolTarget>>) -> Target {
        let bits = bits.map(|b| *b.borrow()).collect_vec();
        let num_bits = bits.len();
        assert!(
            num_bits <= log_floor(F::ORDER, 2),
            "{} bits may overflow the field",
            num_bits
        );
        if num_bits == 0 {
            return self.zero();
        }

        // Check if it's cheaper to just do this with arithmetic operations.
        let arithmetic_ops = num_bits - 1;
        if arithmetic_ops <= self.num_base_arithmetic_ops_per_gate() {
            let two = self.two();
            let mut rev_bits = bits.iter().rev();
            let mut sum = rev_bits.next().unwrap().target;
            for &bit in rev_bits {
                sum = self.mul_add(two, sum, bit.target);
            }
            return sum;
        }

        debug_assert!(
            BaseSumGate::<2>::START_LIMBS + num_bits <= self.config.num_routed_wires,
            "Not enough routed wires."
        );
        let gate_type = BaseSumGate::<2>::new_from_config::<F>(&self.config);
        let row = self.add_gate(gate_type, vec![]);
        for (limb, wire) in bits
            .iter()
            .zip(BaseSumGate::<2>::START_LIMBS..BaseSumGate::<2>::START_LIMBS + num_bits)
        {
            self.connect(limb.target, Target::wire(row, wire));
        }
        for l in gate_type.limbs().skip(num_bits) {
            self.assert_zero(Target::wire(row, l));
        }

        self.add_simple_generator(BaseSumGenerator::<2> { row, limbs: bits });

        Target::wire(row, BaseSumGate::<2>::WIRE_SUM)
    }
}

#[derive(Debug, Default)]
pub struct BaseSumGenerator<const B: usize> {
    row: usize,
    limbs: Vec<BoolTarget>,
}

impl<F: RichField + Extendable<D>, const B: usize, const D: usize> SimpleGenerator<F, D>
    for BaseSumGenerator<B>
{
    fn id(&self) -> String {
        format!("BaseSumGenerator + Base: {B}")
    }

    fn dependencies(&self) -> Vec<Target> {
        self.limbs.iter().map(|b| b.target).collect()
    }

    fn run_once(
        &self,
        witness: &PartitionWitness<F>,
        out_buffer: &mut GeneratedValues<F>,
    ) -> Result<()> {
        let sum = self
            .limbs
            .iter()
            .map(|&t| witness.get_bool_target(t))
            .rev()
            .fold(F::ZERO, |acc, limb| {
                acc * F::from_canonical_usize(B) + F::from_bool(limb)
            });

        out_buffer.set_target(Target::wire(self.row, BaseSumGate::<B>::WIRE_SUM), sum)
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_usize(self.row)?;
        dst.write_target_bool_vec(&self.limbs)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let row = src.read_usize()?;
        let limbs = src.read_target_bool_vec()?;
        Ok(Self { row, limbs })
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::types::Field;
    use rand::rngs::OsRng;
    use rand::Rng;

    use super::*;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_split_base() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let config = CircuitConfig::standard_recursion_config();
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
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let n = OsRng.gen_range(0..(1 << 30));
        let x = builder.constant(F::from_canonical_usize(n));

        let zero = builder._false();
        let one = builder._true();

        let y = builder.le_sum(
            (0..30)
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
