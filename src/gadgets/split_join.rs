use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::wire::Wire;
use crate::witness::PartialWitness;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Split the given integer into a list of virtual advice targets, where each one represents a
    /// bit of the integer, with little-endian ordering.
    ///
    /// Note that this only handles witness generation; it does not enforce that the decomposition
    /// is correct. The output should be treated as a "purported" decomposition which must be
    /// enforced elsewhere.
    pub(crate) fn split_le_virtual(&mut self, integer: Target, num_bits: usize) -> Vec<Target> {
        let bit_targets = self.add_virtual_advice_targets(num_bits);
        self.add_generator(SplitGenerator {
            integer,
            bits: bit_targets.clone(),
        });
        bit_targets
    }
}

/// Generator for a little-endian split.
#[must_use]
pub fn split_le_generator<F: Field>(
    integer: Target,
    bits: Vec<Target>,
) -> Box<dyn WitnessGenerator<F>> {
    Box::new(SplitGenerator { integer, bits })
}

/// Generator for a little-endian split.
#[must_use]
pub fn split_le_generator_local_wires<F: Field>(
    gate: usize,
    integer_input_index: usize,
    bit_input_indices: &[usize],
) -> Box<dyn WitnessGenerator<F>> {
    let integer = Target::Wire(Wire {
        gate,
        input: integer_input_index,
    });
    let bits = bit_input_indices
        .iter()
        .map(|&input| Target::Wire(Wire { gate, input }))
        .collect();
    Box::new(SplitGenerator { integer, bits })
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

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut integer_value = witness.get_target(self.integer).to_canonical_u64();

        let mut result = PartialWitness::new();
        for &b in &self.bits {
            let b_value = integer_value & 1;
            result.set_target(b, F::from_canonical_u64(b_value));
            integer_value >>= 1;
        }

        debug_assert_eq!(
            integer_value, 0,
            "Integer too large to fit in given number of bits"
        );

        result
    }
}
