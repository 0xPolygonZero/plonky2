use std::collections::BTreeMap;
use std::marker::PhantomData;

use num::bigint::BigUint;

use crate::field::field_types::RichField;
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gadgets::arithmetic_u32::U32Target;
use crate::gates::arithmetic_u32::U32ArithmeticGate;
use crate::gates::switch::SwitchGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::bimap::bimap_from_lists;
 
pub struct ForeignFieldTarget<FF: Field> {
    /// These F elements are assumed to contain 32-bit values.
    limbs: Vec<U32Target>,
    _phantom: PhantomData<FF>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn order_u32_limbs<FF: Field>(&self) -> Vec<U32Target> {
        let modulus = FF::order();
        let limbs = modulus.to_u32_digits();
        limbs.iter().map(|&limb| self.constant_u32(F::from_canonical_u32(limb))).collect()
    }

    // Add two `ForeignFieldTarget`s, which we assume are both normalized.
    pub fn add_nonnative<FF: Field>(&mut self, a: ForeignFieldTarget<FF>, b: ForeignFieldTarget<FF>) -> ForeignFieldTarget<FF> {
        let num_limbs = a.limbs.len();
        debug_assert!(b.limbs.len() == num_limbs);

        let mut combined_limbs = self.add_virtual_u32_targets(num_limbs + 1);
        let mut carry = self.zero_u32();
        for i in 0..num_limbs {
            let (new_limb, carry) = self.add_three_u32(carry, a.limbs[i], b.limbs[i]);
            combined_limbs[i] = new_limb;
        }
        combined_limbs[num_limbs] = carry;

        let reduced_limbs = self.reduce_add_result::<FF>(combined_limbs);
        ForeignFieldTarget {
            limbs: reduced_limbs,
            _phantom: PhantomData,
        }
    }

    pub fn reduce_add_result<FF: Field>(&mut self, limbs: Vec<U32Target>) -> Vec<U32Target> {
        let modulus = FF::order();

        todo!()
    }

    pub fn mul_nonnative<FF: Field>(&mut self, a: ForeignFieldTarget<FF>, b: ForeignFieldTarget<FF>) -> ForeignFieldTarget<FF> {
        let num_limbs = a.limbs.len();
        debug_assert!(b.limbs.len() == num_limbs);

        let mut combined_limbs = self.add_virtual_targets(2 * num_limbs - 1);
        for i in 0..num_limbs {
            for j in 0..num_limbs {
                let sum = self.add_u32(a.limbs[i], b.limbs[j]);
                combined_limbs[i + j] = self.add(combined_limbs[i + j], sum);
            }
        }

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
