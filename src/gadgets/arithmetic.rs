use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::Extendable;
use crate::target::Target;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
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
        let xe = self.convert_to_ext(x);
        self.mul_three_extension(xe, xe, xe).to_target_array()[0]
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
        // See if we can determine the result without adding an `ArithmeticGate`.
        if let Some(result) =
            self.arithmetic_special_cases(const_0, multiplicand_0, multiplicand_1, const_1, addend)
        {
            return result;
        }
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

    /// Checks for special cases where the value of
    /// `const_0 * multiplicand_0 * multiplicand_1 + const_1 * addend`
    /// can be determined without adding an `ArithmeticGate`.
    fn arithmetic_special_cases(
        &mut self,
        const_0: F,
        multiplicand_0: Target,
        multiplicand_1: Target,
        const_1: F,
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
            Some(const_0 * x * y)
        } else {
            None
        };
        let second_term_const = if second_term_zero {
            Some(F::ZERO)
        } else {
            addend_const.map(|x| const_1 * x)
        };
        if let (Some(x), Some(y)) = (first_term_const, second_term_const) {
            return Some(self.constant(x + y));
        }

        if first_term_zero && const_1.is_one() {
            return Some(addend);
        }

        if second_term_zero {
            if let Some(x) = mul_0_const {
                if (const_0 * x).is_one() {
                    return Some(multiplicand_1);
                }
            }
            if let Some(x) = mul_1_const {
                if (const_1 * x).is_one() {
                    return Some(multiplicand_0);
                }
            }
        }

        None
    }

    /// Computes `x * y + z`.
    pub fn mul_add(&mut self, x: Target, y: Target, z: Target) -> Target {
        self.arithmetic(F::ONE, x, y, F::ONE, z)
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

    /// Add `n` `Target`s with `ceil(n/2) + 1` `ArithmeticExtensionGate`s.
    // TODO: Can be made `2*D` times more efficient by using all wires of an `ArithmeticExtensionGate`.
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

    /// Multiply `n` `Target`s with `ceil(n/2) + 1` `ArithmeticExtensionGate`s.
    pub fn mul_many(&mut self, terms: &[Target]) -> Target {
        let terms_ext = terms
            .iter()
            .map(|&t| self.convert_to_ext(t))
            .collect::<Vec<_>>();
        self.mul_many_extension(&terms_ext).to_target_array()[0]
    }

    /// Exponentiate `base` to the power of `2^power_log`.
    // TODO: Test
    pub fn exp_power_of_2(&mut self, mut base: Target, power_log: usize) -> Target {
        for _ in 0..power_log {
            base = self.square(base);
        }
        base
    }

    // TODO: Optimize this, maybe with a new gate.
    // TODO: Test
    /// Exponentiate `base` to the power of `exponent`, given by its little-endian bits.
    pub fn exp_from_bits(&mut self, base: Target, exponent_bits: &[Target]) -> Target {
        let mut current = base;
        let one_ext = self.one_extension();
        let mut product = self.one();

        for &bit in exponent_bits {
            // TODO: Add base field select.
            let current_ext = self.convert_to_ext(current);
            let multiplicand = self.select(bit, current_ext, one_ext);
            product = self.mul(product, multiplicand.0[0]);
            current = self.mul(current, current);
        }

        product
    }

    // TODO: Optimize this, maybe with a new gate.
    // TODO: Test
    /// Exponentiate `base` to the power of `2^bit_length-1-exponent`, given by its little-endian bits.
    pub fn exp_from_complement_bits(&mut self, base: Target, exponent_bits: &[Target]) -> Target {
        let mut current = base;
        let one_ext = self.one_extension();
        let mut product = self.one();

        for &bit in exponent_bits {
            let current_ext = self.convert_to_ext(current);
            // TODO: Add base field select.
            let multiplicand = self.select(bit, one_ext, current_ext);
            product = self.mul(product, multiplicand.0[0]);
            current = self.mul(current, current);
        }

        product
    }

    // TODO: Optimize this, maybe with a new gate.
    // TODO: Test
    /// Exponentiate `base` to the power of `exponent`, where `exponent < 2^num_bits`.
    pub fn exp(&mut self, base: Target, exponent: Target, num_bits: usize) -> Target {
        let exponent_bits = self.split_le(exponent, num_bits);
        self.exp_from_bits(base, &exponent_bits)
    }

    /// Exponentiate `base` to the power of a known `exponent`.
    // TODO: Test
    pub fn exp_u64(&mut self, base: Target, exponent: u64) -> Target {
        let base_ext = self.convert_to_ext(base);
        self.exp_u64_extension(base_ext, exponent).0[0]
    }

    /// Computes `x / y`. Results in an unsatisfiable instance if `y = 0`.
    pub fn div(&mut self, x: Target, y: Target) -> Target {
        let y_inv = self.inverse(y);
        self.mul(x, y_inv)
    }

    /// Computes `q = x / y` by witnessing `q` and requiring that `q * y = x`. This can be unsafe in
    /// some cases, as it allows `0 / 0 = <anything>`.
    pub fn div_unsafe(&mut self, x: Target, y: Target) -> Target {
        // Check for special cases where we can determine the result without an `ArithmeticGate`.
        let zero = self.zero();
        let one = self.one();
        if x == zero {
            return zero;
        }
        if y == one {
            return x;
        }
        if let (Some(x_const), Some(y_const)) =
            (self.target_as_constant(x), self.target_as_constant(y))
        {
            return self.constant(x_const / y_const);
        }

        let x_ext = self.convert_to_ext(x);
        let y_ext = self.convert_to_ext(y);
        self.div_unsafe_extension(x_ext, y_ext).0[0]
    }

    /// Computes `1 / x`. Results in an unsatisfiable instance if `x = 0`.
    pub fn inverse(&mut self, x: Target) -> Target {
        let x_ext = self.convert_to_ext(x);
        self.inverse_extension(x_ext).0[0]
    }
}
