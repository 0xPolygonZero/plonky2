use std::marker::PhantomData;

use num::{BigUint, Zero};
use plonky2_field::{extension_field::Extendable, field_types::Field};
use plonky2_util::ceil_div_usize;

use crate::gadgets::arithmetic_u32::U32Target;
use crate::field::field_types::RichField;
use crate::gadgets::binary_arithmetic::BinaryTarget;
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

    pub fn add_nonnative<FF: Field>(
        &mut self,
        a: &NonNativeTarget<FF>,
        b: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        let sum = self.add_virtual_nonnative_target::<FF>();
        let overflow = self.add_virtual_bool_target();

        self.add_simple_generator(NonNativeAdditionGenerator::<F, D, FF> {
            a: a.clone(),
            b: b.clone(),
            sum: sum.clone(),
            overflow: overflow.clone(),
            _phantom: PhantomData,
        });

        let sum_expected = self.add_biguint(&a.value, &b.value);

        let modulus = self.constant_biguint(&FF::order());
        let mod_times_overflow = self.mul_biguint_by_bool(&modulus, overflow);
        let sum_actual = self.add_biguint(&sum.value, &mod_times_overflow);
        self.connect_biguint(&sum_expected, &sum_actual);

        sum
    }

    pub fn mul_nonnative_by_bool<FF: Field>(
        &mut self,
        a: &NonNativeTarget<FF>,
        b: BoolTarget,
    ) -> NonNativeTarget<FF> {
        let t = b.target;

        NonNativeTarget {
            value: BigUintTarget {
                limbs: a
                    .value
                    .limbs
                    .iter()
                    .map(|l| U32Target(self.mul(l.0, t)))
                    .collect(),
            },
            _phantom: PhantomData,
        }
    }

    pub fn add_many_nonnative<FF: Field>(
        &mut self,
        to_add: &[NonNativeTarget<FF>],
    ) -> NonNativeTarget<FF> {
        if to_add.len() == 1 {
            return to_add[0].clone();
        }

        let sum = self.add_virtual_nonnative_target::<FF>();
        let overflow = self.add_virtual_u32_target();
        let summands = to_add.to_vec();

        self.add_simple_generator(NonNativeMultipleAddsGenerator::<F, D, FF> {
            summands: summands.clone(),
            sum: sum.clone(),
            overflow: overflow.clone(),
            _phantom: PhantomData,
        });

        let sum_expected = summands
            .iter()
            .fold(self.zero_biguint(), |a, b| self.add_biguint(&a, &b.value));

        let modulus = self.constant_biguint(&FF::order());
        let overflow_biguint = BigUintTarget {
            limbs: vec![overflow],
        };
        let mod_times_overflow = self.mul_biguint(&modulus, &overflow_biguint);
        let sum_actual = self.add_biguint(&sum.value, &mod_times_overflow);
        self.connect_biguint(&sum_expected, &sum_actual);

        sum
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
        let modulus = FF::order();
        let order_target = self.constant_biguint(&modulus);
        let value = self.rem_biguint(x, &order_target);

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
struct NonNativeAdditionGenerator<F: RichField + Extendable<D>, const D: usize, FF: Field> {
    a: NonNativeTarget<FF>,
    b: NonNativeTarget<FF>,
    sum: NonNativeTarget<FF>,
    overflow: BoolTarget,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize, FF: Field> SimpleGenerator<F>
    for NonNativeAdditionGenerator<F, D, FF>
{
    fn dependencies(&self) -> Vec<Target> {
        self.a
            .value
            .limbs
            .iter()
            .cloned()
            .chain(self.b.value.limbs.clone())
            .map(|l| l.0)
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let a = witness.get_nonnative_target(self.a.clone());
        let b = witness.get_nonnative_target(self.b.clone());
        let a_biguint = a.to_biguint();
        let b_biguint = b.to_biguint();
        let sum_biguint = a_biguint + b_biguint;
        let modulus = FF::order();
        let (overflow, sum_reduced) = if sum_biguint > modulus {
            (true, sum_biguint - modulus)
        } else {
            (false, sum_biguint)
        };

        out_buffer.set_biguint_target(self.sum.value.clone(), sum_reduced);
        out_buffer.set_bool_target(self.overflow, overflow);
    }
}

#[derive(Debug)]
struct NonNativeMultipleAddsGenerator<F: RichField + Extendable<D>, const D: usize, FF: Field> {
    summands: Vec<NonNativeTarget<FF>>,
    sum: NonNativeTarget<FF>,
    overflow: U32Target,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize, FF: Field> SimpleGenerator<F>
    for NonNativeMultipleAddsGenerator<F, D, FF>
{
    fn dependencies(&self) -> Vec<Target> {
        self.summands
            .iter()
            .map(|summand| summand.value.limbs.iter().map(|limb| limb.0))
            .flatten()
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let summands: Vec<_> = self
            .summands
            .iter()
            .map(|summand| witness.get_nonnative_target(summand.clone()))
            .collect();
        let summand_biguints: Vec<_> = summands
            .iter()
            .map(|summand| summand.to_biguint())
            .collect();

        let sum_biguint = summand_biguints
            .iter()
            .fold(BigUint::zero(), |a, b| a + b.clone());

        let modulus = FF::order();
        let (overflow_biguint, sum_reduced) = sum_biguint.div_rem(&modulus);
        let overflow = overflow_biguint.to_u64_digits()[0] as u32;

        out_buffer.set_biguint_target(self.sum.value.clone(), sum_reduced);
        out_buffer.set_u32_target(self.overflow, overflow);
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

        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant_nonnative(x_ff);
        let y = builder.constant_nonnative(y_ff);
        println!("LIMBS LIMBS LIMBS {}", y.value.limbs.len());
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

        let ffs: Vec<_> = (0..num).map(|_| FF::rand()).collect();

        let op_targets: Vec<_> = ffs
            .iter()
            .map(|&x| op_builder.constant_nonnative(x))
            .collect();
        op_builder.mul_many_nonnative(&op_targets);

        let unop_targets: Vec<_> = ffs
            .iter()
            .map(|&x| unop_builder.constant_nonnative(x))
            .collect();
        let mut result = unop_targets[0].clone();
        for i in 1..unop_targets.len() {
            result = unop_builder.mul_nonnative(&result, &unop_targets[i]);
        }
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
