use std::collections::BTreeMap;
use std::marker::PhantomData;

use crate::field::field_types::RichField;
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gates::switch::SwitchGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::bimap::bimap_from_lists;

pub struct U32Target(Target);

pub struct NonNativeTarget {
    /// The modulus of the field F' being represented.
    modulus: BigUInt,
    /// These F elements are assumed to contain 32-bit values.
    limbs: Vec<U32Target>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn add_mul_u32(&mut self, x: U32Target, y: U32Target, z:U32Target) -> (U32Target, U32Target) {
        
    }

    pub fn add_u32(&mut self, a: U32Target, b: U32Target) -> (U32Target, U32Target) {
        
    }

    pub fn mul_u32(&mut self, a: U32Target, b: U32Target) -> (U32Target, U32Target) {
        
    }

    pub fn add_nonnative(&mut self, a: NonNativeTarget, b: NonNativeTarget) -> NonNativeTarget {
        let modulus = a.modulus;
        let num_limbs = a.limbs.len();
        debug_assert!(b.modulus == modulus);
        debug_assert!(b.limbs.len() == num_limbs);

        let mut combined_limbs = self.add_virtual_targets(num_limbs + 1);
        let mut carry = self.zero();
        for i in 0..num_limbs {
            let gate = ComparisonGate::new(bits, num_chunks);
            let gate_index = self.add_gate(gate.clone(), vec![]);

            self.connect(Target::wire(gate_index, gate.wire_first_input()), lhs);
            self.connect(Target::wire(gate_index, gate.wire_second_input()), rhs);
        }
    }

    pub fn reduce_add_result(&mut self, limbs: Vec<Target>, modulus: BigUInt) -> Vec<Target> {
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
                let sum = builder.add(a.limbs[i], b.limbs[j]);
                combined_limbs[i + j] = builder.add(combined_limbs[i + j], sum);
            }
        }

        let reduced_limbs = self.reduce(combined_limbs, modulus);
        
        NonNativeTarget {
            modulus,
            limbs: reduced_limbs,
        }
    }

    pub fn reduce_mul_result(&mut self, limbs: Vec<Target>, modulus: BigUInt) -> Vec<Target> {
        todo!()
    }
}
