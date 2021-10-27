use std::marker::PhantomData;

use num::{BigUint, Integer};

use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gadgets::arithmetic_u32::U32Target;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;

#[derive(Clone, Debug)]
pub struct BigUintTarget {
    pub limbs: Vec<U32Target>,
}

impl BigUintTarget {
    pub fn num_limbs(&self) -> usize {
        self.limbs.len()
    }

    pub fn get_limb(&self, i: usize) -> U32Target {
        self.limbs[i]
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn constant_biguint(&mut self, value: &BigUint) -> BigUintTarget {
        let limb_values = value.to_u32_digits();
        let mut limbs = Vec::new();
        for i in 0..limb_values.len() {
            limbs.push(U32Target(
                self.constant(F::from_canonical_u32(limb_values[i])),
            ));
        }

        BigUintTarget { limbs }
    }

    pub fn connect_biguint(&mut self, lhs: &BigUintTarget, rhs: &BigUintTarget) {
        let min_limbs = lhs.num_limbs().min(rhs.num_limbs());
        for i in 0..min_limbs {
            self.connect_u32(lhs.get_limb(i), rhs.get_limb(i));
        }

        for i in min_limbs..lhs.num_limbs() {
            self.assert_zero_u32(lhs.get_limb(i));
        }
        for i in min_limbs..rhs.num_limbs() {
            self.assert_zero_u32(rhs.get_limb(i));
        }
    }

    pub fn pad_biguints(
        &mut self,
        a: BigUintTarget,
        b: BigUintTarget,
    ) -> (BigUintTarget, BigUintTarget) {
        if a.num_limbs() > b.num_limbs() {
            let mut padded_b_limbs = b.limbs.clone();
            padded_b_limbs.extend(self.add_virtual_u32_targets(a.num_limbs() - b.num_limbs()));
            let padded_b = BigUintTarget {
                limbs: padded_b_limbs,
            };
            (a, padded_b)
        } else {
            let mut padded_a_limbs = a.limbs.clone();
            padded_a_limbs.extend(self.add_virtual_u32_targets(b.num_limbs() - a.num_limbs()));
            let padded_a = BigUintTarget {
                limbs: padded_a_limbs,
            };
            (padded_a, b)
        }
    }

    pub fn cmp_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BoolTarget {
        let (padded_a, padded_b) = self.pad_biguints(a.clone(), b.clone());

        let a_vec = padded_a.limbs.iter().map(|&x| x.0).collect();
        let b_vec = padded_b.limbs.iter().map(|&x| x.0).collect();

        self.list_le(a_vec, b_vec, 32)
    }

    pub fn add_virtual_biguint_target(&mut self, num_limbs: usize) -> BigUintTarget {
        let limbs = (0..num_limbs)
            .map(|_| self.add_virtual_u32_target())
            .collect();

        BigUintTarget { limbs }
    }

    // Add two `BigUintTarget`s.
    pub fn add_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let num_limbs = a.num_limbs().max(b.num_limbs());

        let mut combined_limbs = vec![];
        let mut carry = self.zero_u32();
        for i in 0..num_limbs {
            let a_limb = if i < a.num_limbs() {
                a.limbs[i].clone()
            } else {
                self.zero_u32()
            };
            let b_limb = if i < b.num_limbs() {
                b.limbs[i].clone()
            } else {
                self.zero_u32()
            };

            let (new_limb, new_carry) = self.add_three_u32(carry.clone(), a_limb, b_limb);
            carry = new_carry;
            combined_limbs.push(new_limb);
        }
        combined_limbs.push(carry);

        BigUintTarget {
            limbs: combined_limbs,
        }
    }

    // Subtract two `BigUintTarget`s. We assume that the first is larger than the second.
    pub fn sub_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let num_limbs = a.limbs.len();
        debug_assert!(b.limbs.len() == num_limbs);

        let mut result_limbs = vec![];

        let mut borrow = self.zero_u32();
        for i in 0..num_limbs {
            let (result, new_borrow) = self.sub_u32(a.limbs[i], b.limbs[i], borrow);
            result_limbs.push(result);
            borrow = new_borrow;
        }
        // Borrow should be zero here.

        BigUintTarget {
            limbs: result_limbs,
        }
    }

    pub fn mul_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let total_limbs = a.limbs.len() + b.limbs.len();

        let mut to_add = vec![vec![]; total_limbs];
        for i in 0..a.limbs.len() {
            for j in 0..b.limbs.len() {
                let (product, carry) = self.mul_u32(a.limbs[i], b.limbs[j]);
                to_add[i + j].push(product);
                to_add[i + j + 1].push(carry);
            }
        }

        let mut combined_limbs = vec![];
        let mut carry = self.zero_u32();
        for i in 0..total_limbs {
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

    pub fn div_rem_biguint(
        &mut self,
        a: &BigUintTarget,
        b: &BigUintTarget,
    ) -> (BigUintTarget, BigUintTarget) {
        let num_limbs = a.limbs.len();
        let div = self.add_virtual_biguint_target(num_limbs);
        let rem = self.add_virtual_biguint_target(num_limbs);

        self.add_simple_generator(BigUintDivRemGenerator::<F, D> {
            a: a.clone(),
            b: b.clone(),
            div: div.clone(),
            rem: rem.clone(),
            _phantom: PhantomData,
        });

        let div_b = self.mul_biguint(&div, &b);
        let div_b_plus_rem = self.add_biguint(&div_b, &rem);
        self.connect_biguint(&a, &div_b_plus_rem);

        let cmp_rem_b = self.cmp_biguint(&rem, b);
        self.assert_one(cmp_rem_b.target);

        (div, rem)
    }

    pub fn div_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let (div, _rem) = self.div_rem_biguint(a, b);
        div
    }

    pub fn rem_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let (_div, rem) = self.div_rem_biguint(a, b);
        rem
    }
}

#[derive(Debug)]
struct BigUintDivRemGenerator<F: RichField + Extendable<D>, const D: usize> {
    a: BigUintTarget,
    b: BigUintTarget,
    div: BigUintTarget,
    rem: BigUintTarget,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for BigUintDivRemGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        self.a
            .limbs
            .iter()
            .map(|&l| l.0)
            .chain(self.b.limbs.iter().map(|&l| l.0))
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let a = witness.get_biguint_target(self.a.clone());
        let b = witness.get_biguint_target(self.b.clone());
        let (div, rem) = a.div_rem(&b);

        out_buffer.set_biguint_target(self.div.clone(), div);
        out_buffer.set_biguint_target(self.rem.clone(), rem);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use num::{BigUint, FromPrimitive, Integer};

    use crate::{
        field::crandall_field::CrandallField,
        iop::witness::PartialWitness,
        plonk::{circuit_builder::CircuitBuilder, circuit_data::CircuitConfig, verifier::verify},
    };

    #[test]
    fn test_biguint_add() -> Result<()> {
        let x_value = BigUint::from_u128(22222222222222222222222222222222222222).unwrap();
        let y_value = BigUint::from_u128(33333333333333333333333333333333333333).unwrap();
        let expected_z_value = &x_value + &y_value;

        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_biguint(&x_value);
        let y = builder.constant_biguint(&y_value);
        let z = builder.add_biguint(&x, &y);
        let expected_z = builder.constant_biguint(&expected_z_value);

        builder.connect_biguint(&z, &expected_z);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_biguint_sub() -> Result<()> {
        let x_value = BigUint::from_u128(33333333333333333333333333333333333333).unwrap();
        let y_value = BigUint::from_u128(22222222222222222222222222222222222222).unwrap();
        let expected_z_value = &x_value - &y_value;

        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_biguint(&x_value);
        let y = builder.constant_biguint(&y_value);
        let z = builder.sub_biguint(&x, &y);
        let expected_z = builder.constant_biguint(&expected_z_value);

        builder.connect_biguint(&z, &expected_z);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_biguint_mul() -> Result<()> {
        let x_value = BigUint::from_u128(123123123123123123123123123123123123).unwrap();
        let y_value = BigUint::from_u128(456456456456456456456456456456456456).unwrap();
        let expected_z_value = &x_value * &y_value;

        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_biguint(&x_value);
        let y = builder.constant_biguint(&y_value);
        let z = builder.mul_biguint(&x, &y);
        let expected_z = builder.constant_biguint(&expected_z_value);

        builder.connect_biguint(&z, &expected_z);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_biguint_cmp() -> Result<()> {
        let x_value = BigUint::from_u128(33333333333333333333333333333333333333).unwrap();
        let y_value = BigUint::from_u128(22222222222222222222222222222222222222).unwrap();

        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_biguint(&x_value);
        let y = builder.constant_biguint(&y_value);
        let cmp = builder.cmp_biguint(&x, &y);
        let expected_cmp = builder.constant_bool(false);

        builder.connect(cmp.target, expected_cmp.target);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_biguint_div_rem() -> Result<()> {
        let x_value = BigUint::from_u128(456456456456456456456456456456456456).unwrap();
        let y_value = BigUint::from_u128(123123123123123123123123123123123123).unwrap();
        let (expected_div_value, expected_rem_value) = x_value.div_rem(&y_value);

        type F = CrandallField;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let x = builder.constant_biguint(&x_value);
        let y = builder.constant_biguint(&y_value);
        let (div, rem) = builder.div_rem_biguint(&x, &y);

        let expected_div = builder.constant_biguint(&expected_div_value);
        let expected_rem = builder.constant_biguint(&expected_rem_value);

        builder.connect_biguint(&div, &expected_div);
        builder.connect_biguint(&rem, &expected_rem);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }
}
