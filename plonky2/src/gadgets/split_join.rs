#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use anyhow::Result;

use crate::field::extension::Extendable;
use crate::gates::base_sum::BaseSumGate;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness, WitnessWrite};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::util::serialization::{Buffer, IoResult, Read, Write};

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Split the given integer into a list of wires, where each one represents a
    /// bit of the integer, with little-endian ordering.
    /// Verifies that the decomposition is correct by using `k` `BaseSum<2>` gates
    /// with `k` such that `k * num_routed_wires >= num_bits`.
    pub fn split_le(&mut self, integer: Target, num_bits: usize) -> Vec<BoolTarget> {
        if num_bits == 0 {
            return Vec::new();
        }
        let gate_type = BaseSumGate::<2>::new_from_config::<F>(&self.config);
        let k = num_bits.div_ceil(gate_type.num_limbs);
        let gates = (0..k)
            .map(|_| self.add_gate(gate_type, vec![]))
            .collect::<Vec<_>>();

        let mut bits = Vec::with_capacity(num_bits);
        for &gate in &gates {
            for limb_column in gate_type.limbs() {
                // `new_unsafe` is safe here because BaseSumGate::<2> forces it to be in `{0, 1}`.
                bits.push(BoolTarget::new_unsafe(Target::wire(gate, limb_column)));
            }
        }
        for b in bits.drain(num_bits..) {
            self.assert_zero(b.target);
        }

        let zero = self.zero();
        let base = F::TWO.exp_u64(gate_type.num_limbs as u64);
        let mut acc = zero;
        for &gate in gates.iter().rev() {
            let sum = Target::wire(gate, BaseSumGate::<2>::WIRE_SUM);
            acc = self.mul_const_add(base, acc, sum);
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

#[derive(Debug, Default)]
pub struct SplitGenerator {
    integer: Target,
    bits: Vec<Target>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F, D> for SplitGenerator {
    fn id(&self) -> String {
        "SplitGenerator".to_string()
    }

    fn dependencies(&self) -> Vec<Target> {
        vec![self.integer]
    }

    fn run_once(
        &self,
        witness: &PartitionWitness<F>,
        out_buffer: &mut GeneratedValues<F>,
    ) -> Result<()> {
        let mut integer_value = witness.get_target(self.integer).to_canonical_u64();

        for &b in &self.bits {
            let b_value = integer_value & 1;
            out_buffer.set_target(b, F::from_canonical_u64(b_value))?;
            integer_value >>= 1;
        }

        debug_assert_eq!(
            integer_value, 0,
            "Integer too large to fit in given number of bits"
        );

        Ok(())
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_target(self.integer)?;
        dst.write_target_vec(&self.bits)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let integer = src.read_target()?;
        let bits = src.read_target_vec()?;
        Ok(Self { integer, bits })
    }
}

#[derive(Debug, Default)]
pub struct WireSplitGenerator {
    integer: Target,
    gates: Vec<usize>,
    num_limbs: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F, D> for WireSplitGenerator {
    fn id(&self) -> String {
        "WireSplitGenerator".to_string()
    }

    fn dependencies(&self) -> Vec<Target> {
        vec![self.integer]
    }

    fn run_once(
        &self,
        witness: &PartitionWitness<F>,
        out_buffer: &mut GeneratedValues<F>,
    ) -> Result<()> {
        let mut integer_value = witness.get_target(self.integer).to_canonical_u64();

        for &gate in &self.gates {
            let sum = Target::wire(gate, BaseSumGate::<2>::WIRE_SUM);

            // If num_limbs >= 64, we don't need to truncate since `integer_value` is already
            // limited to 64 bits, and trying to do so would cause overflow. Hence the conditional.
            let mut truncated_value = integer_value;
            if self.num_limbs < 64 {
                truncated_value = integer_value & ((1 << self.num_limbs) - 1);
                integer_value >>= self.num_limbs;
            } else {
                integer_value = 0;
            };

            out_buffer.set_target(sum, F::from_canonical_u64(truncated_value))?;
        }

        debug_assert_eq!(
            integer_value,
            0,
            "Integer too large to fit in {} many `BaseSumGate`s",
            self.gates.len()
        );

        Ok(())
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_target(self.integer)?;
        dst.write_usize_vec(&self.gates)?;
        dst.write_usize(self.num_limbs)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let integer = src.read_target()?;
        let gates = src.read_usize_vec()?;
        let num_limbs = src.read_usize()?;
        Ok(Self {
            integer,
            gates,
            num_limbs,
        })
    }
}
