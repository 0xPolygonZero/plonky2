use std::marker::PhantomData;

use num::{BigUint, Zero};
use plonky2_field::{extension_field::Extendable, field_types::Field};
use plonky2_util::ceil_div_usize;

use crate::gadgets::arithmetic_u32::U32Target;
use crate::gadgets::biguint::BigUintTarget;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;

#[derive(Clone, Debug)]
pub struct NonNativeTarget<FF: Field> {
    pub(crate) value: BigUintTarget,
    pub(crate) _phantom: PhantomData<FF>,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    fn num_nonnative_limbs<FF: Field>() -> usize {
        ceil_div_usize(FF::BITS, 32)
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

    pub fn zero_nonnative<FF: Field>(&mut self) -> NonNativeTarget<FF> {
        self.constant_nonnative(FF::ZERO)
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

    pub fn add_many_nonnative<FF: Field>(
        &mut self,
        to_add: &[NonNativeTarget<FF>],
    ) -> NonNativeTarget<FF> {
        if to_add.len() == 1 {
            return to_add[0].clone();
        }

        let mut result = self.add_biguint(&to_add[0].value, &to_add[1].value);
        for i in 2..to_add.len() {
            result = self.add_biguint(&result, &to_add[i].value);
        }

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

    pub fn mul_many_nonnative<FF: Field>(
        &mut self,
        to_mul: &[NonNativeTarget<FF>],
    ) -> NonNativeTarget<FF> {
        if to_mul.len() == 1 {
            return to_mul[0].clone();
        }

        let mut result = self.mul_biguint(&to_mul[0].value, &to_mul[1].value);
        for i in 2..to_mul.len() {
            result = self.mul_biguint(&result, &to_mul[i].value);
        }

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
        let div = self.add_virtual_biguint_target(num_limbs);
        
        self.add_simple_generator(NonNativeInverseGenerator::<F, D, FF> {
            x: x.clone(),
            inv: inv_biguint.clone(),
            div: div.clone(),
            _phantom: PhantomData,
        });

        let product = self.mul_biguint(&x.value, &inv_biguint);
        
        let modulus = self.constant_biguint(&FF::order());
        let mod_times_div = self.mul_biguint(&modulus, &div);
        let one = self.constant_biguint(&BigUint::one());
        let expected_product = self.add_biguint(&mod_times_div, &one);
        self.connect_biguint(&product, &expected_product);

        let inv = NonNativeTarget::<FF> {
            value: inv_biguint,
            _phantom: PhantomData,
        };
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
        println!("NUM LIMBS: {}", x.limbs.len());
        let before = self.num_gates();

        let modulus = FF::order();
        let order_target = self.constant_biguint(&modulus);
        let value = self.rem_biguint(x, &order_target);

        println!("NUMBER OF GATES: {}", self.num_gates() - before);
        println!("OUTPUT LIMBS: {}", value.limbs.len());

        NonNativeTarget {
            value,
            _phantom: PhantomData,
        }
    }

    /// Returns `x % |FF|` as a `NonNativeTarget`.
    fn reduce_by_bits<FF: Field>(&mut self, x: &BigUintTarget) -> NonNativeTarget<FF> {
        println!("NUM LIMBS: {}", x.limbs.len());
        let before = self.num_gates();

        let mut powers_of_two = Vec::new();
        let mut cur_power_of_two = FF::ONE;
        let two = FF::TWO;
        let mut max_num_limbs = 0;
        for _ in 0..(x.limbs.len() * 32) {
            let cur_power = self.constant_biguint(&cur_power_of_two.to_biguint());
            max_num_limbs = max_num_limbs.max(cur_power.limbs.len());
            powers_of_two.push(cur_power.limbs);

            cur_power_of_two *= two;
        }

        let mut result_limbs_unreduced = vec![self.zero(); max_num_limbs];
        for i in 0..x.limbs.len() {
            let this_limb = x.limbs[i];
            let bits = self.split_le(this_limb.0, 32);
            for b in 0..bits.len() {
                let this_power = powers_of_two[32 * i + b].clone();
                for x in 0..this_power.len() {
                    result_limbs_unreduced[x] = self.mul_add(bits[b].target, this_power[x].0, result_limbs_unreduced[x]);
                }
            }
        }

        let mut result_limbs_reduced = Vec::new();
        let mut carry = self.zero_u32();
        for i in 0..result_limbs_unreduced.len() {
            println!("{}", i);
            let (low, high) = self.split_to_u32(result_limbs_unreduced[i]);
            let (cur, overflow) = self.add_u32(carry, low);
            let (new_carry, _) = self.add_many_u32(&[overflow, high, carry]);
            result_limbs_reduced.push(cur);
            carry = new_carry;
        }
        result_limbs_reduced.push(carry);

        let value = BigUintTarget {
            limbs: result_limbs_reduced,
        };

        println!("NUMBER OF GATES: {}", self.num_gates() - before);
        println!("OUTPUT LIMBS: {}", value.limbs.len());

        NonNativeTarget {
            value,
            _phantom: PhantomData,
        }
    }

    #[allow(dead_code)]
    fn reduce_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF> {
        let x_biguint = self.nonnative_to_biguint(x);
        self.reduce(&x_biguint)
    }

    pub fn bool_to_nonnative<FF: Field>(&mut self, b: &BoolTarget) -> NonNativeTarget<FF> {
        let limbs = vec![U32Target(b.target)];
        let value = BigUintTarget { limbs };

        NonNativeTarget {
            value,
            _phantom: PhantomData,
        }
    }

    // Split a nonnative field element to bits.
    pub fn split_nonnative_to_bits<FF: Field>(
        &mut self,
        x: &NonNativeTarget<FF>,
    ) -> Vec<BoolTarget> {
        let num_limbs = x.value.num_limbs();
        let mut result = Vec::with_capacity(num_limbs * 32);

        for i in 0..num_limbs {
            let limb = x.value.get_limb(i);
            let bit_targets = self.split_le_base::<2>(limb.0, 32);
            let mut bits: Vec<_> = bit_targets
                .iter()
                .map(|&t| BoolTarget::new_unsafe(t))
                .collect();

            result.append(&mut bits);
        }

        result
    }
}

#[derive(Debug)]
struct NonNativeInverseGenerator<F: RichField + Extendable<D>, const D: usize, FF: Field> {
    x: NonNativeTarget<FF>,
    inv: BigUintTarget,
    div: BigUintTarget,
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

        let x_biguint = x.to_biguint();
        let inv_biguint = inv.to_biguint();
        let prod = x_biguint * &inv_biguint;
        let modulus = FF::order();
        let (div, _rem) = prod.div_rem(&modulus);

        out_buffer.set_biguint_target(self.div.clone(), div);
        out_buffer.set_biguint_target(self.inv.clone(), inv_biguint);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::field_types::Field;
    use plonky2_field::secp256k1_base::Secp256K1Base;

    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_nonnative_add() -> Result<()> {
        type FF = Secp256K1Base;
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let x_ff = FF::rand();
        let y_ff = FF::rand();
        let sum_ff = x_ff + y_ff;

        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let y = builder.constant_nonnative(y_ff);
        let sum = builder.add_nonnative(&x, &y);

        let sum_expected = builder.constant_nonnative(sum_ff);
        builder.connect_nonnative(&sum, &sum_expected);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_nonnative_many_adds() -> Result<()> {
        type FF = Secp256K1Base;
        let a_ff = FF::rand();
        let b_ff = FF::rand();
        let c_ff = FF::rand();
        let d_ff = FF::rand();
        let e_ff = FF::rand();
        let f_ff = FF::rand();
        let g_ff = FF::rand();
        let h_ff = FF::rand();
        let sum_ff = a_ff + b_ff + c_ff + d_ff + e_ff + f_ff + g_ff + h_ff;

        type F = GoldilocksField;
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, 4>::new(config);

        let a = builder.constant_nonnative(a_ff);
        let b = builder.constant_nonnative(b_ff);
        let c = builder.constant_nonnative(c_ff);
        let d = builder.constant_nonnative(d_ff);
        let e = builder.constant_nonnative(e_ff);
        let f = builder.constant_nonnative(f_ff);
        let g = builder.constant_nonnative(g_ff);
        let h = builder.constant_nonnative(h_ff);
        let all = [a, b, c, d, e, f, g, h];
        let sum = builder.add_many_nonnative(&all);

        let sum_expected = builder.constant_nonnative(sum_ff);
        builder.connect_nonnative(&sum, &sum_expected);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_nonnative_sub() -> Result<()> {
        type FF = Secp256K1Base;
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let x_ff = FF::rand();
        let mut y_ff = FF::rand();
        while y_ff.to_biguint() > x_ff.to_biguint() {
            y_ff = FF::rand();
        }
        let diff_ff = x_ff - y_ff;

        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let y = builder.constant_nonnative(y_ff);
        let diff = builder.sub_nonnative(&x, &y);

        let diff_expected = builder.constant_nonnative(diff_ff);
        builder.connect_nonnative(&diff, &diff_expected);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_nonnative_mul() -> Result<()> {
        type FF = Secp256K1Base;
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let x_ff = FF::rand();
        let y_ff = FF::rand();
        let product_ff = x_ff * y_ff;
        println!("PRODUCT FF: {:?}", product_ff);

        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let y = builder.constant_nonnative(y_ff);
        let product = builder.mul_nonnative(&x, &y);

        let product_expected = builder.constant_nonnative(product_ff);
        builder.connect_nonnative(&product, &product_expected);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    fn test_nonnative_many_muls_helper(num: usize) {
        type FF = Secp256K1Base;

        type F = GoldilocksField;
        let config = CircuitConfig::standard_recursion_config();
        let mut unop_builder = CircuitBuilder::<F, 4>::new(config.clone());
        let mut op_builder = CircuitBuilder::<F, 4>::new(config);

        println!("NUM: {}", num);

        let ffs: Vec<_> = (0..num).map(|_| FF::rand()).collect();

        let op_targets: Vec<_> = ffs.iter().map(|&x| op_builder.constant_nonnative(x)).collect();
        op_builder.mul_many_nonnative(&op_targets);
        println!("OPTIMIZED GATE COUNT: {}", op_builder.num_gates());

        let unop_targets: Vec<_> = ffs.iter().map(|&x| unop_builder.constant_nonnative(x)).collect();
        let mut result = unop_targets[0].clone();
        for i in 1..unop_targets.len() {
            result = unop_builder.mul_nonnative(&result, &unop_targets[i]);
        }

        println!("UNOPTIMIZED GATE COUNT: {}", unop_builder.num_gates());
    }

    #[test]
    fn test_nonnative_many_muls() {
        for num in 2..10 {
            test_nonnative_many_muls_helper(num);
        }
    }

    #[test]
    fn test_nonnative_neg() -> Result<()> {
        type FF = Secp256K1Base;
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let x_ff = FF::rand();
        let neg_x_ff = -x_ff;

        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let neg_x = builder.neg_nonnative(&x);

        let neg_x_expected = builder.constant_nonnative(neg_x_ff);
        builder.connect_nonnative(&neg_x, &neg_x_expected);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_nonnative_inv() -> Result<()> {
        type FF = Secp256K1Base;
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let x_ff = FF::rand();
        let inv_x_ff = x_ff.inverse();

        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let inv_x = builder.inv_nonnative(&x);

        let inv_x_expected = builder.constant_nonnative(inv_x_ff);
        builder.connect_nonnative(&inv_x, &inv_x_expected);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }
}
