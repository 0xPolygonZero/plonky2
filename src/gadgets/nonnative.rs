use std::marker::PhantomData;

use num::{BigUint, One};

use crate::field::field_types::RichField;
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gadgets::biguint::BigUintTarget;
use crate::plonk::circuit_builder::CircuitBuilder;

pub struct ForeignFieldTarget<FF: Field> {
    value: BigUintTarget,
    _phantom: PhantomData<FF>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn biguint_to_ff<FF: Field>(&mut self, x: &BigUintTarget) -> ForeignFieldTarget<FF> {
        ForeignFieldTarget {
            value: x.clone(),
            _phantom: PhantomData,
        }
    }

    pub fn ff_to_biguint<FF: Field>(&mut self, x: &ForeignFieldTarget<FF>) -> BigUintTarget {
        x.value.clone()
    }

    pub fn constant_ff<FF: Field>(&mut self, x: FF) -> ForeignFieldTarget<FF> {
        let x_biguint = self.constant_biguint(&x.to_biguint());
        self.biguint_to_ff(&x_biguint)
    }

    // Assert that two ForeignFieldTarget's, both assumed to be in reduced form, are equal.
    pub fn connect_ff_reduced<FF: Field>(
        &mut self,
        lhs: &ForeignFieldTarget<FF>,
        rhs: &ForeignFieldTarget<FF>,
    ) {
        self.connect_biguint(&lhs.value, &rhs.value);
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

    pub fn neg_nonnative<FF: Field>(
        &mut self,
        x: &ForeignFieldTarget<FF>,
    ) -> ForeignFieldTarget<FF> {
        let neg_one = FF::order() - BigUint::one();
        let neg_one_target = self.constant_biguint(&neg_one);
        let neg_one_ff = self.biguint_to_ff(&neg_one_target);

        self.mul_nonnative(&neg_one_ff, x)
    }

    /// Returns `x % |FF|` as a `ForeignFieldTarget`.
    fn reduce<FF: Field>(&mut self, x: &BigUintTarget) -> ForeignFieldTarget<FF> {
        let modulus = FF::order();
        let order_target = self.constant_biguint(&modulus);
        let value = self.rem_biguint(x, &order_target);

        ForeignFieldTarget {
            value,
            _phantom: PhantomData,
        }
    }

    fn reduce_ff<FF: Field>(&mut self, x: &ForeignFieldTarget<FF>) -> ForeignFieldTarget<FF> {
        let x_biguint = self.ff_to_biguint(x);
        self.reduce(&x_biguint)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::field::secp256k1::Secp256K1Base;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_nonnative_add() -> Result<()> {
        type FF = Secp256K1Base;
        let x_ff = FF::rand();
        let y_ff = FF::rand();
        let sum_ff = x_ff + y_ff;

        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_ff(x_ff);
        let y = builder.constant_ff(y_ff);
        let sum = builder.add_nonnative(&x, &y);

        let sum_expected = builder.constant_ff(sum_ff);
        builder.connect_ff_reduced(&sum, &sum_expected);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_nonnative_sub() -> Result<()> {
        type FF = Secp256K1Base;
        let x_ff = FF::rand();
        let mut y_ff = FF::rand();
        while y_ff.to_biguint() > x_ff.to_biguint() {
            y_ff = FF::rand();
        }
        let diff_ff = x_ff - y_ff;

        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_ff(x_ff);
        let y = builder.constant_ff(y_ff);
        let diff = builder.sub_nonnative(&x, &y);

        let diff_expected = builder.constant_ff(diff_ff);
        builder.connect_ff_reduced(&diff, &diff_expected);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_nonnative_mul() -> Result<()> {
        type FF = Secp256K1Base;
        let x_ff = FF::rand();
        let y_ff = FF::rand();
        let product_ff = x_ff * y_ff;

        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_ff(x_ff);
        let y = builder.constant_ff(y_ff);
        let product = builder.mul_nonnative(&x, &y);

        let product_expected = builder.constant_ff(product_ff);
        builder.connect_ff_reduced(&product, &product_expected);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_nonnative_neg() -> Result<()> {
        type FF = Secp256K1Base;
        let x_ff = FF::rand();
        let neg_x_ff = -x_ff;

        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_ff(x_ff);
        let neg_x = builder.neg_nonnative(&x);

        let neg_x_expected = builder.constant_ff(neg_x_ff);
        builder.connect_ff_reduced(&neg_x, &neg_x_expected);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }
}
