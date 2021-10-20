use std::borrow::Borrow;

use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::exponentiation::ExponentiationGate;
use crate::iop::target::{BoolTarget, Target};
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
        multiplicand_0: Target,
        multiplicand_1: Target,
        const_1: F,
        addend: Target,
    ) -> Target {
        let multiplicand_0_ext = self.convert_to_ext(multiplicand_0);
        let multiplicand_1_ext = self.convert_to_ext(multiplicand_1);
        let addend_ext = self.convert_to_ext(addend);

        self.arithmetic_extension(
            const_0,
            const_1,
            multiplicand_0_ext,
            multiplicand_1_ext,
            addend_ext,
        )
        .0[0]
    }

    /// Computes `x * y + z`.
    pub fn mul_add(&mut self, x: Target, y: Target, z: Target) -> Target {
        self.arithmetic(F::ONE, x, y, F::ONE, z)
    }

    /// Computes `x + C`.
    pub fn add_const(&mut self, x: Target, c: F) -> Target {
        let one = self.one();
        self.arithmetic(F::ONE, one, x, c, one)
    }

    /// Computes `C * x`.
    pub fn mul_const(&mut self, c: F, x: Target) -> Target {
        let zero = self.zero();
        self.mul_const_add(c, x, zero)
    }

    /// Computes `C * x + y`.
    pub fn mul_const_add(&mut self, c: F, x: Target, y: Target) -> Target {
        let one = self.one();
        self.arithmetic(c, x, one, F::ONE, y)
    }

    /// Computes `x * y - z`.
    pub fn mul_sub(&mut self, x: Target, y: Target, z: Target) -> Target {
        self.arithmetic(F::ONE, x, y, F::NEG_ONE, z)
    }

    /// Computes `x + y`.
    pub fn add(&mut self, x: Target, y: Target) -> Target {
        let one = self.one();
        // x + y = 1 * x * 1 + 1 * y
        self.arithmetic(F::ONE, x, one, F::ONE, y)
    }

    /// Add `n` `Target`s.
    // TODO: Can be made `D` times more efficient by using all wires of an `ArithmeticExtensionGate`.
    pub fn add_many(&mut self, terms: &[Target]) -> Target {
        let terms_ext = terms
            .iter()
            .map(|&t| self.convert_to_ext(t))
            .collect::<Vec<_>>();
        self.add_many_extension(&terms_ext).to_target_array()[0]
    }

    /// Computes `x - y`.
    pub fn sub(&mut self, x: Target, y: Target) -> Target {
        let one = self.one();
        // x - y = 1 * x * 1 + (-1) * y
        self.arithmetic(F::ONE, x, one, F::NEG_ONE, y)
    }

    /// Computes `x * y`.
    pub fn mul(&mut self, x: Target, y: Target) -> Target {
        // x * y = 1 * x * y + 0 * x
        self.arithmetic(F::ONE, x, y, F::ZERO, x)
    }

    /// Multiply `n` `Target`s.
    pub fn mul_many(&mut self, terms: &[Target]) -> Target {
        let terms_ext = terms
            .iter()
            .map(|&t| self.convert_to_ext(t))
            .collect::<Vec<_>>();
        self.mul_many_extension(&terms_ext).to_target_array()[0]
    }

    /// Exponentiate `base` to the power of `2^power_log`.
    pub fn exp_power_of_2(&mut self, base: Target, power_log: usize) -> Target {
        self.exp_u64(base, 1 << power_log)
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
        let gate_index = self.add_gate(gate.clone(), vec![]);

        self.connect(base, Target::wire(gate_index, gate.wire_base()));
        exp_bits_vec.iter().enumerate().for_each(|(i, bit)| {
            self.connect(bit.target, Target::wire(gate_index, gate.wire_power_bit(i)));
        });

        Target::wire(gate_index, gate.wire_output())
    }

    // TODO: Test
    /// Exponentiate `base` to the power of `exponent`, where `exponent < 2^num_bits`.
    pub fn exp(&mut self, base: Target, exponent: Target, num_bits: usize) -> Target {
        let exponent_bits = self.split_le(exponent, num_bits);

        self.exp_from_bits(base, exponent_bits.iter())
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
