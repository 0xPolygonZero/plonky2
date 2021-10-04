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

pub struct NonNativeTarget {
    /// The modulus of the field F' being represented.
    modulus: BigUint,
    /// These F elements are assumed to contain 32-bit values.
    limbs: Vec<U32Target>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn add_nonnative(&mut self, a: NonNativeTarget, b: NonNativeTarget) -> NonNativeTarget {
        let modulus = a.modulus;
        let num_limbs = a.limbs.len();
        debug_assert!(b.modulus == modulus);
        debug_assert!(b.limbs.len() == num_limbs);

        let mut combined_limbs = self.add_virtual_u32_targets(num_limbs + 1);
        let mut carry = self.zero_u32();
        for i in 0..num_limbs {
            let (new_limb, carry) = self.add_three_u32(carry, a.limbs[i], b.limbs[i]);
            combined_limbs[i] = new_limb;
        }
        combined_limbs[num_limbs] = carry;

        let reduced_limbs = self.reduce_add_result(combined_limbs, modulus);
        NonNativeTarget {
            modulus,
            limbs: reduced_limbs,
        }
    }

    pub fn reduce_add_result(&mut self, limbs: Vec<Target>, modulus: BigUint) -> Vec<Target> {
        todo!()
    }

    pub fn mul_nonnative(&mut self, a: NonNativeTarget, b: NonNativeTarget) -> NonNativeTarget {
        let modulus = a.modulus;
        let num_limbs = a.limbs.len();
        debug_assert!(b.modulus == modulus);
        debug_assert!(b.limbs.len() == num_limbs);

        let mut combined_limbs = self.add_virtual_targets(2 * num_limbs - 1);
        for i in 0..num_limbs {
            for j in 0..num_limbs {
                let sum = self.add(a.limbs[i], b.limbs[j]);
                combined_limbs[i + j] = self.add(combined_limbs[i + j], sum);
            }
        }

        let reduced_limbs = self.reduce_mul_result(combined_limbs, modulus);

        NonNativeTarget {
            modulus,
            limbs: reduced_limbs,
        }
    }

    pub fn reduce_mul_result(&mut self, limbs: Vec<Target>, modulus: BigUint) -> Vec<Target> {
        todo!()
    }
}
