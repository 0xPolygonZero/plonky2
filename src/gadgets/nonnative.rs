use std::marker::PhantomData;

use crate::field::field_types::RichField;
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gadgets::arithmetic_u32::U32Target;
use crate::plonk::circuit_builder::CircuitBuilder;

pub struct ForeignFieldTarget<FF: Field> {
    limbs: Vec<U32Target>,
    _phantom: PhantomData<FF>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn order_u32_limbs<FF: Field>(&mut self) -> Vec<U32Target> {
        let modulus = FF::order();
        let limbs = modulus.to_u32_digits();
        limbs
            .iter()
            .map(|&limb| self.constant_u32(F::from_canonical_u32(limb)))
            .collect()
    }

    // Add two `ForeignFieldTarget`s.
    pub fn add_nonnative<FF: Field>(
        &mut self,
        a: ForeignFieldTarget<FF>,
        b: ForeignFieldTarget<FF>,
    ) -> ForeignFieldTarget<FF> {
        let num_limbs = a.limbs.len();
        debug_assert!(b.limbs.len() == num_limbs);

        let mut combined_limbs = self.add_virtual_u32_targets(num_limbs + 1);
        let mut carry = self.zero_u32();
        for i in 0..num_limbs {
            let (new_limb, new_carry) =
                self.add_three_u32(carry.clone(), a.limbs[i].clone(), b.limbs[i].clone());
            carry = new_carry;
            combined_limbs[i] = new_limb;
        }
        combined_limbs[num_limbs] = carry;

        let reduced_limbs = self.reduce_add_result::<FF>(combined_limbs);
        ForeignFieldTarget {
            limbs: reduced_limbs,
            _phantom: PhantomData,
        }
    }

    /// Reduces the result of a non-native addition.
    pub fn reduce_add_result<FF: Field>(&mut self, limbs: Vec<U32Target>) -> Vec<U32Target> {
        let num_limbs = limbs.len();

        let mut modulus_limbs = self.order_u32_limbs::<FF>();
        modulus_limbs.push(self.zero_u32());

        let needs_reduce = self.list_le_u32(modulus_limbs, limbs);

        let mut to_subtract = vec![];
        for i in 0..num_limbs {
            let (low, _high) = self.mul_u32(modulus_limbs[i], U32Target(needs_reduce.target));
            to_subtract.push(low);
        }

        let mut reduced_limbs = vec![];

        let mut borrow = self.zero_u32();
        for i in 0..num_limbs {
            let (result, new_borrow) = self.sub_u32(limbs[i], to_subtract[i], borrow);
            reduced_limbs[i] = result;
            borrow = new_borrow;
        }
        // Borrow should be zero here.

        reduced_limbs
    }

    // Subtract two `ForeignFieldTarget`s. We assume that the first is larger than the second.
    pub fn sub_nonnative<FF: Field>(
        &mut self,
        a: ForeignFieldTarget<FF>,
        b: ForeignFieldTarget<FF>,
    ) -> ForeignFieldTarget<FF> {
        let num_limbs = a.limbs.len();
        debug_assert!(b.limbs.len() == num_limbs);

        let mut result_limbs = vec![];

        let mut borrow = self.zero_u32();
        for i in 0..num_limbs {
            let (result, new_borrow) = self.sub_u32(a.limbs[i], b.limbs[i], borrow);
            result_limbs[i] = result;
            borrow = new_borrow;
        }
        // Borrow should be zero here.

        ForeignFieldTarget {
            limbs: result_limbs,
            _phantom: PhantomData,
        }
    }

    pub fn mul_nonnative<FF: Field>(
        &mut self,
        a: ForeignFieldTarget<FF>,
        b: ForeignFieldTarget<FF>,
    ) -> ForeignFieldTarget<FF> {
        let num_limbs = a.limbs.len();
        debug_assert!(b.limbs.len() == num_limbs);

        let mut combined_limbs = self.add_virtual_u32_targets(2 * num_limbs - 1);
        let mut to_add = vec![vec![]; 2 * num_limbs];
        for i in 0..num_limbs {
            for j in 0..num_limbs {
                let (product, carry) = self.mul_u32(a.limbs[i], b.limbs[j]);
                to_add[i + j].push(product);
                to_add[i + j + 1].push(carry);
            }
        }

        let mut combined_limbs = vec![];
        let mut carry = self.zero_u32();
        for i in 0..2 * num_limbs {
            to_add[i].push(carry);
            let (new_result, new_carry) = self.add_many_u32(to_add[i]);
            combined_limbs.push(new_result);
            carry = new_carry;
        }
        combined_limbs.push(carry);

        let reduced_limbs = self.reduce_mul_result::<FF>(combined_limbs);

        ForeignFieldTarget {
            limbs: reduced_limbs,
            _phantom: PhantomData,
        }
    }

    pub fn reduce_mul_result<FF: Field>(&mut self, limbs: Vec<U32Target>) -> Vec<U32Target> {
        todo!()
    }
}
