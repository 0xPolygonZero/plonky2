use std::marker::PhantomData;

use num::{BigUint, One, Zero};

use crate::field::field_types::RichField;
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gadgets::biguint::BigUintTarget;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::Target;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;

#[derive(Clone, Debug)]
pub struct NonNativeTarget<FF: Field> {
    pub(crate) value: BigUintTarget,
    _phantom: PhantomData<FF>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    fn num_nonnative_limbs<FF: Field>() -> usize {
        let ff_size = FF::order();
        let f_size = F::order();
        let num_limbs = ((ff_size + f_size.clone() - BigUint::one()) / f_size).to_u32_digits()[0];

        num_limbs as usize
    }

    pub fn biguint_to_nonnative<FF: Field>(&mut self, x: &BigUintTarget) -> NonNativeTarget<FF> {
        NonNativeTarget {
            value: x.clone(),
            _phantom: PhantomData,
        }
    }

    pub fn nonnative_to_biguint<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> BigUintTarget {
        x.value.clone()
    }

    pub fn constant_nonnative<FF: Field>(&mut self, x: FF) -> NonNativeTarget<FF> {
        let x_biguint = self.constant_biguint(&x.to_biguint());
        self.biguint_to_nonnative(&x_biguint)
    }

    // Assert that two NonNativeTarget's, both assumed to be in reduced form, are equal.
    pub fn connect_nonnative<FF: Field>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) {
        self.connect_biguint(&lhs.value, &rhs.value);
    }

    pub fn add_virtual_nonnative_target<FF: Field>(&mut self) -> NonNativeTarget<FF> {
        let num_limbs = Self::num_nonnative_limbs::<FF>();
        let value = self.add_virtual_biguint_target(num_limbs);

        NonNativeTarget {
            value,
            _phantom: PhantomData,
        }
    }

    // Add two `NonNativeTarget`s.
    pub fn add_nonnative<FF: Field>(
        &mut self,
        a: &NonNativeTarget<FF>,
        b: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        let result = self.add_biguint(&a.value, &b.value);

        // TODO: reduce add result with only one conditional subtraction
        self.reduce(&result)
    }

    // Subtract two `NonNativeTarget`s.
    pub fn sub_nonnative<FF: Field>(
        &mut self,
        a: &NonNativeTarget<FF>,
        b: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        let order = self.constant_biguint(&FF::order());
        let a_plus_order = self.add_biguint(&order, &a.value);
        let result = self.sub_biguint(&a_plus_order, &b.value);

        // TODO: reduce sub result with only one conditional addition?
        self.reduce(&result)
    }

    pub fn mul_nonnative<FF: Field>(
        &mut self,
        a: &NonNativeTarget<FF>,
        b: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        let result = self.mul_biguint(&a.value, &b.value);

        self.reduce(&result)
    }

    pub fn neg_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF> {
        let zero_target = self.constant_biguint(&BigUint::zero());
        let zero_ff = self.biguint_to_nonnative(&zero_target);

        self.sub_nonnative(&zero_ff, x)
    }

    pub fn inv_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF> {
        let num_limbs = x.value.num_limbs();
        let inv_biguint = self.add_virtual_biguint_target(num_limbs);
        let inv = NonNativeTarget::<FF> {
            value: inv_biguint,
            _phantom: PhantomData,
        };

        self.add_simple_generator(NonNativeInverseGenerator::<F, D, FF> {
            x: x.clone(),
            inv: inv.clone(),
            _phantom: PhantomData,
        });

        let product = self.mul_nonnative(&x, &inv);
        let one = self.constant_nonnative(FF::ONE);
        self.connect_nonnative(&product, &one);

        inv
    }

    pub fn div_rem_nonnative<FF: Field>(
        &mut self,
        x: &NonNativeTarget<FF>,
        y: &NonNativeTarget<FF>,
    ) -> (NonNativeTarget<FF>, NonNativeTarget<FF>) {
        let x_biguint = self.nonnative_to_biguint(x);
        let y_biguint = self.nonnative_to_biguint(y);

        let (div_biguint, rem_biguint) = self.div_rem_biguint(&x_biguint, &y_biguint);
        let div = self.biguint_to_nonnative(&div_biguint);
        let rem = self.biguint_to_nonnative(&rem_biguint);
        (div, rem)
    }

    /// Returns `x % |FF|` as a `NonNativeTarget`.
    fn reduce<FF: Field>(&mut self, x: &BigUintTarget) -> NonNativeTarget<FF> {
        let modulus = FF::order();
        let order_target = self.constant_biguint(&modulus);
        let value = self.rem_biguint(x, &order_target);

        NonNativeTarget {
            value,
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    fn reduce_nonnative<FF: Field>(
        &mut self,
        x: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        let x_biguint = self.nonnative_to_biguint(x);
        self.reduce(&x_biguint)
    }
}

#[derive(Debug)]
struct NonNativeInverseGenerator<F: RichField + Extendable<D>, const D: usize, FF: Field> {
    x: NonNativeTarget<FF>,
    inv: NonNativeTarget<FF>,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize, FF: Field> SimpleGenerator<F>
    for NonNativeInverseGenerator<F, D, FF>
{
    fn dependencies(&self) -> Vec<Target> {
        self.x.value.limbs.iter().map(|&l| l.0).collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let x = witness.get_nonnative_target(self.x.clone());
        let inv = x.inverse();

        out_buffer.set_nonnative_target(self.inv.clone(), inv);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::field::secp256k1_base::Secp256K1Base;
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

        type F = GoldilocksField;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let y = builder.constant_nonnative(y_ff);
        let sum = builder.add_nonnative(&x, &y);

        let sum_expected = builder.constant_nonnative(sum_ff);
        builder.connect_nonnative(&sum, &sum_expected);

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

        type F = GoldilocksField;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let y = builder.constant_nonnative(y_ff);
        let diff = builder.sub_nonnative(&x, &y);

        let diff_expected = builder.constant_nonnative(diff_ff);
        builder.connect_nonnative(&diff, &diff_expected);

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

        type F = GoldilocksField;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let y = builder.constant_nonnative(y_ff);
        let product = builder.mul_nonnative(&x, &y);

        let product_expected = builder.constant_nonnative(product_ff);
        builder.connect_nonnative(&product, &product_expected);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_nonnative_neg() -> Result<()> {
        type FF = Secp256K1Base;
        let x_ff = FF::rand();
        let neg_x_ff = -x_ff;

        type F = GoldilocksField;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let neg_x = builder.neg_nonnative(&x);

        let neg_x_expected = builder.constant_nonnative(neg_x_ff);
        builder.connect_nonnative(&neg_x, &neg_x_expected);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_nonnative_inv() -> Result<()> {
        type FF = Secp256K1Base;
        let x_ff = FF::rand();
        let inv_x_ff = x_ff.inverse();

        type F = GoldilocksField;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let inv_x = builder.inv_nonnative(&x);

        let inv_x_expected = builder.constant_nonnative(inv_x_ff);
        builder.connect_nonnative(&inv_x, &inv_x_expected);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }
}
