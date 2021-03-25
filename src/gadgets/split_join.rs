use std::iter;

use crate::constraint_polynomial::ConstraintPolynomial;
use crate::field::field::Field;
use crate::generator::{SimpleGenerator, WitnessGenerator};
use crate::target::Target;
use crate::witness::PartialWitness;
use crate::wire::Wire;

/// Constraints for a little-endian split.
pub fn split_le_constraints<F: Field>(
    integer: ConstraintPolynomial<F>,
    bits: &[ConstraintPolynomial<F>],
) -> Vec<ConstraintPolynomial<F>> {
    let weighted_sum = bits.iter()
        .fold(ConstraintPolynomial::zero(), |acc, b| acc.double() + b);
    bits.iter()
        .rev()
        .map(|b| b * (b - 1))
        .chain(iter::once(weighted_sum - integer))
        .collect()
}

/// Generator for a little-endian split.
pub fn split_le_generator<F: Field>(
    integer: Target,
    bits: Vec<Target>,
) -> Box<dyn WitnessGenerator<F>> {
    Box::new(SplitGenerator { integer, bits })
}

/// Generator for a little-endian split.
pub fn split_le_generator_local_wires<F: Field>(
    gate: usize,
    integer_input_index: usize,
    bit_input_indices: &[usize],
) -> Box<dyn WitnessGenerator<F>> {
    let integer = Target::Wire(
        Wire { gate, input: integer_input_index });
    let bits = bit_input_indices.iter()
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

        debug_assert_eq!(integer_value, 0,
                         "Integer too large to fit in given number of bits");

        result
    }
}
