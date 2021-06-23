use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field::Field;
use crate::gates::arithmetic::ArithmeticGate;
use crate::gates::mul_extension::ArithmeticExtensionGate;
use crate::generator::SimpleGenerator;
use crate::target::Target;
use crate::wire::Wire;
use crate::witness::PartialWitness;

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
        // See if we can determine the result without adding an `ArithmeticGate`.
        if let Some(result) =
            self.arithmetic_special_cases(const_0, multiplicand_0, multiplicand_1, const_1, addend)
        {
            return result;
        }

        let gate = self.add_gate(ArithmeticGate::new(), vec![const_0, const_1]);

        let wire_multiplicand_0 = Wire {
            gate,
            input: ArithmeticGate::WIRE_MULTIPLICAND_0,
        };
        let wire_multiplicand_1 = Wire {
            gate,
            input: ArithmeticGate::WIRE_MULTIPLICAND_1,
        };
        let wire_addend = Wire {
            gate,
            input: ArithmeticGate::WIRE_ADDEND,
        };
        let wire_output = Wire {
            gate,
            input: ArithmeticGate::WIRE_OUTPUT,
        };

        self.route(multiplicand_0, Target::Wire(wire_multiplicand_0));
        self.route(multiplicand_1, Target::Wire(wire_multiplicand_1));
        self.route(addend, Target::Wire(wire_addend));
        Target::Wire(wire_output)
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

    pub fn add_many(&mut self, terms: &[Target]) -> Target {
        let mut sum = self.zero();
        for term in terms {
            sum = self.add(sum, *term);
        }
        sum
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

    pub fn mul_many(&mut self, terms: &[Target]) -> Target {
        let mut product = self.one();
        for term in terms {
            product = self.mul(product, *term);
        }
        product
    }

    // TODO: Optimize this, maybe with a new gate.
    /// Exponentiate `base` to the power of `exponent`, where `exponent < 2^num_bits`.
    pub fn exp(&mut self, base: Target, exponent: Target, num_bits: usize) -> Target {
        let mut current = base;
        let one = self.one();
        let mut product = one;
        let exponent_bits = self.split_le(exponent, num_bits);

        for bit in exponent_bits.into_iter() {
            product = self.mul_many(&[bit, current, product]);
            current = self.mul(current, current);
        }

        product
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

        // Add an `ArithmeticGate` to compute `q * y`.
        let gate = self.add_gate(ArithmeticGate::new(), vec![F::ONE, F::ZERO]);

        let wire_multiplicand_0 = Wire {
            gate,
            input: ArithmeticGate::WIRE_MULTIPLICAND_0,
        };
        let wire_multiplicand_1 = Wire {
            gate,
            input: ArithmeticGate::WIRE_MULTIPLICAND_1,
        };
        let wire_addend = Wire {
            gate,
            input: ArithmeticGate::WIRE_ADDEND,
        };
        let wire_output = Wire {
            gate,
            input: ArithmeticGate::WIRE_OUTPUT,
        };

        let q = Target::Wire(wire_multiplicand_0);
        self.add_generator(QuotientGenerator {
            numerator: x,
            denominator: y,
            quotient: q,
        });

        self.route(y, Target::Wire(wire_multiplicand_1));

        // This can be anything, since the whole second term has a weight of zero.
        self.route(zero, Target::Wire(wire_addend));

        let q_y = Target::Wire(wire_output);
        self.assert_equal(q_y, x);

        q
    }

    /// Computes `q = x / y` by witnessing `q` and requiring that `q * y = x`. This can be unsafe in
    /// some cases, as it allows `0 / 0 = <anything>`.
    pub fn div_unsafe_extension(
        &mut self,
        x: ExtensionTarget<D>,
        y: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        // Add an `ArithmeticGate` to compute `q * y`.
        let gate = self.add_gate(ArithmeticExtensionGate::new(), vec![F::ONE, F::ZERO]);

        let multiplicand_0 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_multiplicand_0());
        let multiplicand_1 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_multiplicand_1());
        let output =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_output_0());

        self.add_generator(QuotientGeneratorExtension {
            numerator: x,
            denominator: y,
            quotient: multiplicand_0,
        });

        self.route_extension(y, multiplicand_1);

        self.assert_equal_extension(output, x);

        multiplicand_0
    }
}

struct QuotientGenerator {
    numerator: Target,
    denominator: Target,
    quotient: Target,
}

impl<F: Field> SimpleGenerator<F> for QuotientGenerator {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.numerator, self.denominator]
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let num = witness.get_target(self.numerator);
        let den = witness.get_target(self.denominator);
        PartialWitness::singleton_target(self.quotient, num / den)
    }
}

struct QuotientGeneratorExtension<const D: usize> {
    numerator: ExtensionTarget<D>,
    denominator: ExtensionTarget<D>,
    quotient: ExtensionTarget<D>,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for QuotientGeneratorExtension<D> {
    fn dependencies(&self) -> Vec<Target> {
        let mut deps = self.numerator.to_target_array().to_vec();
        deps.extend(&self.denominator.to_target_array());
        deps
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let num = witness.get_extension_target(self.numerator);
        let dem = witness.get_extension_target(self.denominator);
        let quotient = num / dem;
        let mut pw = PartialWitness::new();
        for i in 0..D {
            pw.set_target(
                self.quotient.to_target_array()[i],
                quotient.to_basefield_array()[i],
            );
        }
        pw
    }
}

/// An iterator over the powers of a certain base element `b`: `b^0, b^1, b^2, ...`.
#[derive(Clone)]
pub struct PowersTarget<const D: usize> {
    base: ExtensionTarget<D>,
    current: ExtensionTarget<D>,
}

impl<const D: usize> PowersTarget<D> {
    pub fn next<F: Extendable<D>>(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
    ) -> ExtensionTarget<D> {
        let result = self.current;
        self.current = builder.mul_extension(self.base, self.current);
        result
    }

    pub fn repeated_frobenius<F: Extendable<D>>(
        self,
        k: usize,
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        let Self { base, current } = self;
        Self {
            base: base.repeated_frobenius(k, builder),
            current: current.repeated_frobenius(k, builder),
        }
    }
}

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn powers(&mut self, base: ExtensionTarget<D>) -> PowersTarget<D> {
        PowersTarget {
            base,
            current: self.one_extension(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::circuit_builder::CircuitBuilder;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field::Field;
    use crate::fri::FriConfig;
    use crate::witness::PartialWitness;

    #[test]
    fn test_div_extension() {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_config();

        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = FF::rand();
        let y = FF::rand();
        let x = FF::TWO;
        let y = FF::ONE;
        let z = x / y;
        let xt = builder.constant_extension(x);
        let yt = builder.constant_extension(y);
        let zt = builder.constant_extension(z);
        let comp_zt = builder.div_unsafe_extension(xt, yt);
        builder.assert_equal_extension(zt, comp_zt);

        let data = builder.build();
        let proof = data.prove(PartialWitness::new());
    }
}
