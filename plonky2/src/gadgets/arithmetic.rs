use std::borrow::Borrow;

use plonky2_field::extension::Extendable;
use plonky2_field::types::Field64;

use crate::gates::arithmetic_base::ArithmeticGate;
use crate::gates::exponentiation::ExponentiationGate;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
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
        self.mul_many([x, x, x])
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
        // If we're not configured to use the base arithmetic gate, just call arithmetic_extension.
        if !self.config.use_base_arithmetic_gate {
            let multiplicand_0_ext = self.convert_to_ext(multiplicand_0);
            let multiplicand_1_ext = self.convert_to_ext(multiplicand_1);
            let addend_ext = self.convert_to_ext(addend);

            return self
                .arithmetic_extension(
                    const_0,
                    const_1,
                    multiplicand_0_ext,
                    multiplicand_1_ext,
                    addend_ext,
                )
                .0[0];
        }

        // See if we can determine the result without adding an `ArithmeticGate`.
        if let Some(result) =
            self.arithmetic_special_cases(const_0, const_1, multiplicand_0, multiplicand_1, addend)
        {
            return result;
        }

        // See if we've already computed the same operation.
        let operation = BaseArithmeticOperation {
            const_0,
            const_1,
            multiplicand_0,
            multiplicand_1,
            addend,
        };
        if let Some(&result) = self.base_arithmetic_results.get(&operation) {
            return result;
        }

        // Otherwise, we must actually perform the operation using an ArithmeticExtensionGate slot.
        let result = self.add_base_arithmetic_operation(operation);
        self.base_arithmetic_results.insert(operation, result);
        result
    }

    fn add_base_arithmetic_operation(&mut self, operation: BaseArithmeticOperation<F>) -> Target {
        let gate = ArithmeticGate::new_from_config(&self.config);
        let constants = vec![operation.const_0, operation.const_1];
        let (gate, i) = self.find_slot(gate, &constants, &constants);
        let wires_multiplicand_0 = Target::wire(gate, ArithmeticGate::wire_ith_multiplicand_0(i));
        let wires_multiplicand_1 = Target::wire(gate, ArithmeticGate::wire_ith_multiplicand_1(i));
        let wires_addend = Target::wire(gate, ArithmeticGate::wire_ith_addend(i));

        self.connect(operation.multiplicand_0, wires_multiplicand_0);
        self.connect(operation.multiplicand_1, wires_multiplicand_1);
        self.connect(operation.addend, wires_addend);

        Target::wire(gate, ArithmeticGate::wire_ith_output(i))
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
    pub fn add_many<T>(&mut self, terms: impl IntoIterator<Item = T>) -> Target
    where
        T: Borrow<Target>,
    {
        terms
            .into_iter()
            .fold(self.zero(), |acc, t| self.add(acc, *t.borrow()))
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
    pub fn mul_many<T>(&mut self, terms: impl IntoIterator<Item = T>) -> Target
    where
        T: Borrow<Target>,
    {
        terms
            .into_iter()
            .fold(self.one(), |acc, t| self.mul(acc, *t.borrow()))
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
        let _false = self._false();
        let gate = ExponentiationGate::new_from_config(&self.config);
        let num_power_bits = gate.num_power_bits;
        let mut exp_bits_vec: Vec<BoolTarget> =
            exponent_bits.into_iter().map(|b| *b.borrow()).collect();
        while exp_bits_vec.len() < num_power_bits {
            exp_bits_vec.push(_false);
        }
        let row = self.add_gate(gate.clone(), vec![]);

        self.connect(base, Target::wire(row, gate.wire_base()));
        exp_bits_vec.iter().enumerate().for_each(|(i, bit)| {
            self.connect(bit.target, Target::wire(row, gate.wire_power_bit(i)));
        });

        Target::wire(row, gate.wire_output())
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

    pub fn not(&mut self, b: BoolTarget) -> BoolTarget {
        let one = self.one();
        let res = self.sub(one, b.target);
        BoolTarget::new_unsafe(res)
    }

    pub fn and(&mut self, b1: BoolTarget, b2: BoolTarget) -> BoolTarget {
        BoolTarget::new_unsafe(self.mul(b1.target, b2.target))
    }

    pub fn _if(&mut self, b: BoolTarget, x: Target, y: Target) -> Target {
        let not_b = self.not(b);
        let maybe_x = self.mul(b.target, x);
        self.mul_add(not_b.target, y, maybe_x)
    }

    pub fn is_equal(&mut self, x: Target, y: Target) -> BoolTarget {
        let zero = self.zero();

        let equal = self.add_virtual_bool_target_unsafe();
        let not_equal = self.not(equal);
        let inv = self.add_virtual_target();
        self.add_simple_generator(EqualityGenerator { x, y, equal, inv });

        let diff = self.sub(x, y);
        let not_equal_check = self.mul(equal.target, diff);

        let diff_normalized = self.mul(diff, inv);
        let equal_check = self.sub(diff_normalized, not_equal.target);

        self.connect(not_equal_check, zero);
        self.connect(equal_check, zero);

        equal
    }
}

#[derive(Debug)]
struct EqualityGenerator {
    x: Target,
    y: Target,
    equal: BoolTarget,
    inv: Target,
}

impl<F: RichField> SimpleGenerator<F> for EqualityGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.x, self.y]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let x = witness.get_target(self.x);
        let y = witness.get_target(self.y);

        let inv = if x != y { (x - y).inverse() } else { F::ZERO };

        out_buffer.set_bool_target(self.equal, x == y);
        out_buffer.set_target(self.inv, inv);
    }
}

/// Represents a base arithmetic operation in the circuit. Used to memoize results.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct BaseArithmeticOperation<F: Field64> {
    const_0: F,
    const_1: F,
    multiplicand_0: Target,
    multiplicand_1: Target,
    addend: Target,
}
