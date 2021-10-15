use std::marker::PhantomData;

use num::BigUint;

use crate::field::field_types::RichField;
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gadgets::arithmetic_u32::U32Target;
use crate::plonk::circuit_builder::CircuitBuilder;

pub struct BigUintTarget {
    limbs: Vec<U32Target>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    // Add two `BigUintTarget`s.
    pub fn add_biguint(&mut self, a: BigUintTarget, b: BigUintTarget) -> BigUintTarget {
        let num_limbs = a.limbs.len();
        debug_assert!(b.limbs.len() == num_limbs);

        let mut combined_limbs = vec![];
        let mut carry = self.zero_u32();
        for i in 0..num_limbs {
            let (new_limb, new_carry) =
                self.add_three_u32(carry.clone(), a.limbs[i].clone(), b.limbs[i].clone());
            carry = new_carry;
            combined_limbs.push(new_limb);
        }
        combined_limbs[num_limbs] = carry;

        BigUintTarget {
            limbs: combined_limbs,
        }
    }

    // Subtract two `BigUintTarget`s. We assume that the first is larger than the second.
    pub fn sub_biguint(&mut self, a: BigUintTarget, b: BigUintTarget) -> BigUintTarget {
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

        BigUintTarget {
            limbs: result_limbs,
        }
    }

    pub fn mul_biguint(&mut self, a: BigUintTarget, b: BigUintTarget) -> BigUintTarget {
        let num_limbs = a.limbs.len();
        debug_assert!(b.limbs.len() == num_limbs);

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
            let (new_result, new_carry) = self.add_many_u32(to_add[i].clone());
            combined_limbs.push(new_result);
            carry = new_carry;
        }
        combined_limbs.push(carry);

        BigUintTarget {
            limbs: combined_limbs,
        }
    }
}
