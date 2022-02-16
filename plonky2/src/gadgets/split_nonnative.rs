use itertools::Itertools;
use plonky2_field::extension_field::Extendable;
use plonky2_field::field_types::Field;

use crate::gadgets::arithmetic_u32::U32Target;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::hash::hash_types::RichField;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn split_u32_to_4_bit_limbs(&mut self, val: U32Target) -> Vec<Target> {
        let two_bit_limbs = self.split_le_base::<2>(val.0, 16);
        let four = self.constant(F::from_canonical_usize(4));
        let combined_limbs = two_bit_limbs
            .iter()
            .tuples()
            .map(|(&a, &b)| self.mul_add(b, four, a))
            .collect();

        combined_limbs
    }

    pub fn split_nonnative_to_4_bit_limbs<FF: Field>(
        &mut self,
        val: &NonNativeTarget<FF>,
    ) -> Vec<Target> {
        val.value
            .limbs
            .iter()
            .flat_map(|&l| self.split_u32_to_4_bit_limbs(l))
            .collect()
    }
}
