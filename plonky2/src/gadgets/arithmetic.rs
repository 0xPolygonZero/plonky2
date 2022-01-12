use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use plonky2_field::extension_field::Extendable;
use plonky2_field::field_types::PrimeField;

use crate::gates::arithmetic_base::ArithmeticGate;
use crate::gates::exponentiation::ExponentiationGate;
use crate::gates::gate::GateRef;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::operation::Operation;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Computes `-x`.
    pub fn neg(&mut self, x: Target) -> Target {
        let neg_one = self.neg_one();
        self.mul(x, neg_one)
    }

    /// Computes `x^2`.
    pub fn square(&mut self, x: Target) -> Target {
        self.mul(x, x)
    }

    /// Computes `x^3`.
    pub fn cube(&mut self, x: Target) -> Target {
        self.mul_many(&[x, x, x])
    }

    /// Computes `const_0 * multiplicand_0 * multiplicand_1 + const_1 * addend`.
    pub fn arithmetic(
        &mut self,
        const_0: F,
        const_1: F,
        multiplicand_0: Target,
        multiplicand_1: Target,
        addend: Target,
    ) -> Target {
        todo!()
    }

    /// Checks for special cases where the value of
    /// `const_0 * multiplicand_0 * multiplicand_1 + const_1 * addend`
    /// can be determined without adding an `ArithmeticGate`.
    fn arithmetic_special_cases(
        &mut self,
        const_0: F,
        const_1: F,
        multiplicand_0: Target,
        multiplicand_1: Target,
        addend: Target,
    ) -> Option<Target> {
        let zero = self.zero();

        let mul_0_const = self.target_as_constant(multiplicand_0);
        let mul_1_const = self.target_as_constant(multiplicand_1);
        let addend_const = self.target_as_constant(addend);

        let first_term_zero =
            const_0 == F::ZERO || multiplicand_0 == zero || multiplicand_1 == zero;
        let second_term_zero = const_1 == F::ZERO || addend == zero;

        // If both terms are constant, return their (constant) sum.
        let first_term_const = if first_term_zero {
            Some(F::ZERO)
        } else if let (Some(x), Some(y)) = (mul_0_const, mul_1_const) {
            Some(x * y * const_0)
        } else {
            None
        };
        let second_term_const = if second_term_zero {
            Some(F::ZERO)
        } else {
            addend_const.map(|x| x * const_1)
        };
        if let (Some(x), Some(y)) = (first_term_const, second_term_const) {
            return Some(self.constant(x + y));
        }

        if first_term_zero && const_1.is_one() {
            return Some(addend);
        }

        if second_term_zero {
            if let Some(x) = mul_0_const {
                if (x * const_0).is_one() {
                    return Some(multiplicand_1);
                }
            }
            if let Some(x) = mul_1_const {
                if (x * const_0).is_one() {
                    return Some(multiplicand_0);
                }
            }
        }

        None
    }

    /// Computes `x * y + z`.
    pub fn mul_add(&mut self, x: Target, y: Target, z: Target) -> Target {
        self.arithmetic(F::ONE, F::ONE, x, y, z)
    }

    /// Computes `x + C`.
    pub fn add_const(&mut self, x: Target, c: F) -> Target {
        let c = self.constant(c);
        self.add(x, c)
    }

    /// Computes `C * x`.
    pub fn mul_const(&mut self, c: F, x: Target) -> Target {
        let c = self.constant(c);
        self.mul(c, x)
    }

    /// Computes `C * x + y`.
    pub fn mul_const_add(&mut self, c: F, x: Target, y: Target) -> Target {
        let c = self.constant(c);
        self.mul_add(c, x, y)
    }

    /// Computes `x * y - z`.
    pub fn mul_sub(&mut self, x: Target, y: Target, z: Target) -> Target {
        self.arithmetic(F::ONE, F::NEG_ONE, x, y, z)
    }

    /// Computes `x + y`.
    pub fn add(&mut self, x: Target, y: Target) -> Target {
        let one = self.one();
        // x + y = 1 * x * 1 + 1 * y
        self.arithmetic(F::ONE, F::ONE, x, one, y)
    }

    /// Add `n` `Target`s.
    pub fn add_many(&mut self, terms: &[Target]) -> Target {
        terms.iter().fold(self.zero(), |acc, &t| self.add(acc, t))
    }

    /// Computes `x - y`.
    pub fn sub(&mut self, x: Target, y: Target) -> Target {
        let one = self.one();
        // x - y = 1 * x * 1 + (-1) * y
        self.arithmetic(F::ONE, F::NEG_ONE, x, one, y)
    }

    /// Computes `x * y`.
    pub fn mul(&mut self, x: Target, y: Target) -> Target {
        // x * y = 1 * x * y + 0 * x
        self.arithmetic(F::ONE, F::ZERO, x, y, x)
    }

    /// Multiply `n` `Target`s.
    pub fn mul_many(&mut self, terms: &[Target]) -> Target {
        terms
            .iter()
            .copied()
            .reduce(|acc, t| self.mul(acc, t))
            .unwrap_or_else(|| self.one())
    }

    /// Exponentiate `base` to the power of `2^power_log`.
    pub fn exp_power_of_2(&mut self, base: Target, power_log: usize) -> Target {
        if power_log > self.num_base_arithmetic_ops_per_gate() {
            // Cheaper to just use `ExponentiateGate`.
            return self.exp_u64(base, 1 << power_log);
        }

        let mut product = base;
        for _ in 0..power_log {
            product = self.square(product);
        }
        product
    }

    // TODO: Test
    /// Exponentiate `base` to the power of `exponent`, given by its little-endian bits.
    pub fn exp_from_bits(
        &mut self,
        base: Target,
        exponent_bits: impl IntoIterator<Item = impl Borrow<BoolTarget>>,
    ) -> Target {
        let mut targets: Vec<Target> = vec![base];
        let bits = exponent_bits.into_iter().map(|b| *b.borrow()).collect();
        let result = self.add_target();
        let gate = ExponentiationGate::new_from_config(&self.config);
        let exponentiation_operation = ExponentiationOperation {
            base,
            bits,
            result,
            intermediate_values: self.add_targets(gate.num_power_bits),
            gate,
        };
        self.add_operation(exponentiation_operation);
        result
    }

    // TODO: Test
    /// Exponentiate `base` to the power of `exponent`, where `exponent < 2^num_bits`.
    pub fn exp(&mut self, base: Target, exponent: Target, num_bits: usize) -> Target {
        let exponent_bits = self.split_le(exponent, num_bits);

        self.exp_from_bits(base, exponent_bits.iter())
    }

    /// Like `exp_from_bits` but with a constant base.
    pub fn exp_from_bits_const_base(
        &mut self,
        base: F,
        exponent_bits: impl IntoIterator<Item = impl Borrow<BoolTarget>>,
    ) -> Target {
        let base_t = self.constant(base);
        let exponent_bits: Vec<_> = exponent_bits.into_iter().map(|b| *b.borrow()).collect();

        if exponent_bits.len() > self.num_base_arithmetic_ops_per_gate() {
            // Cheaper to just use `ExponentiateGate`.
            return self.exp_from_bits(base_t, exponent_bits);
        }

        let mut product = self.one();
        for (i, bit) in exponent_bits.iter().enumerate() {
            let pow = 1 << i;
            // If the bit is on, we multiply product by base^pow.
            // We can arithmetize this as:
            //     product *= 1 + bit (base^pow - 1)
            //     product = (base^pow - 1) product bit + product
            product = self.arithmetic(
                base.exp_u64(pow as u64) - F::ONE,
                F::ONE,
                product,
                bit.target,
                product,
            )
        }
        product
    }

    /// Exponentiate `base` to the power of a known `exponent`.
    // TODO: Test
    pub fn exp_u64(&mut self, base: Target, mut exponent: u64) -> Target {
        let mut exp_bits = Vec::new();
        while exponent != 0 {
            let bit = (exponent & 1) == 1;
            let bit_target = self.constant_bool(bit);
            exp_bits.push(bit_target);
            exponent >>= 1;
        }

        self.exp_from_bits(base, exp_bits)
    }

    /// Computes `x / y`. Results in an unsatisfiable instance if `y = 0`.
    pub fn div(&mut self, x: Target, y: Target) -> Target {
        let x = self.convert_to_ext(x);
        let y = self.convert_to_ext(y);
        self.div_extension(x, y).0[0]
    }

    /// Computes `1 / x`. Results in an unsatisfiable instance if `x = 0`.
    pub fn inverse(&mut self, x: Target) -> Target {
        let x_ext = self.convert_to_ext(x);
        self.inverse_extension(x_ext).0[0]
    }
}

#[derive(Debug)]
struct ExponentiationOperation<F: RichField + Extendable<D>, const D: usize> {
    base: Target,
    bits: Vec<BoolTarget>,
    result: Target,
    intermediate_values: Vec<Target>,
    gate: ExponentiationGate<F, D>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for ExponentiationOperation<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        let mut ans = vec![self.base];
        ans.extend(self.bits.iter().map(|b| b.target));
        ans
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let num_power_bits = self.gate.num_power_bits;
        let base = witness.get_target(self.base);

        let power_bits = self
            .bits
            .iter()
            .map(|t| witness.get_target(t.target))
            .collect::<Vec<_>>();
        let mut intermediate_values = Vec::new();

        let mut current_intermediate_value = F::ONE;
        for i in 0..num_power_bits {
            if power_bits[num_power_bits - i - 1] == F::ONE {
                current_intermediate_value *= base;
            }
            intermediate_values.push(current_intermediate_value);
            current_intermediate_value *= current_intermediate_value;
        }

        for i in 0..num_power_bits {
            out_buffer.set_target(self.intermediate_values[i], intermediate_values[i]);
        }

        out_buffer.set_target(self.result, intermediate_values[num_power_bits - 1]);
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Operation<F, D>
    for ExponentiationOperation<F, D>
{
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn targets(&self) -> Vec<Target> {
        let mut ans = vec![self.base];
        ans.extend(self.bits.iter().map(|b| b.target));
        ans.push(self.result);
        ans.extend(&self.intermediate_values);
        ans
    }

    fn gate(&self) -> Option<GateRef<F, D>> {
        Some(GateRef(Arc::new(self.gate)))
    }

    fn constants(&self) -> Vec<F> {
        vec![]
    }
}
