use std::marker::PhantomData;

use crate::gadgets::biguint::BigUintTarget;
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

    pub fn ff_to_biguint<FF: Field>(&mut self, x: &ForeignFieldTarget<FF>) -> BigUintTarget {
        BigUintTarget {
            limbs: x.limbs.clone(),
        }
    }

    // Add two `ForeignFieldTarget`s.
    pub fn add_nonnative<FF: Field>(
        &mut self,
        a: &ForeignFieldTarget<FF>,
        b: &ForeignFieldTarget<FF>,
    ) -> ForeignFieldTarget<FF> {
        let a_biguint = self.ff_to_biguint(a);
        let b_biguint = self.ff_to_biguint(b);
        let result = self.add_biguint(&a_biguint, &b_biguint);

        self.reduce(&result)
    }

    // Subtract two `ForeignFieldTarget`s. We assume that the first is larger than the second.
    pub fn sub_nonnative<FF: Field>(
        &mut self,
        a: &ForeignFieldTarget<FF>,
        b: &ForeignFieldTarget<FF>,
    ) -> ForeignFieldTarget<FF> {
        let a_biguint = self.ff_to_biguint(a);
        let b_biguint = self.ff_to_biguint(b);
        let result = self.sub_biguint(&a_biguint, &b_biguint);

        self.reduce(&result)
    }

    pub fn mul_nonnative<FF: Field>(
        &mut self,
        a: &ForeignFieldTarget<FF>,
        b: &ForeignFieldTarget<FF>,
    ) -> ForeignFieldTarget<FF> {
        let a_biguint = self.ff_to_biguint(a);
        let b_biguint = self.ff_to_biguint(b);
        let result = self.mul_biguint(&a_biguint, &b_biguint);

        self.reduce(&result)
    }

    /// Returns `x % |FF|` as a `ForeignFieldTarget`.
    fn reduce<FF: Field>(
        &mut self,
        x: &BigUintTarget,
    ) -> ForeignFieldTarget<FF> {
        let modulus = FF::order();
        let order_target = self.constant_biguint(&modulus);
        let value = self.rem_biguint(x, &order_target);

        ForeignFieldTarget {
            limbs: value.limbs,
            _phantom: PhantomData,
        }
    }

    fn reduce_ff<FF: Field>(&mut self, x: &ForeignFieldTarget<FF>) -> ForeignFieldTarget<FF> {
        let x_biguint = self.ff_to_biguint(x);
        self.reduce(&x_biguint)
    }
}
