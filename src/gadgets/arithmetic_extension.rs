use std::convert::{TryFrom, TryInto};
use std::ops::Range;

use itertools::Itertools;
use num::Integer;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::{ExtensionAlgebraTarget, ExtensionTarget};
use crate::field::extension_field::{Extendable, OEF};
use crate::field::field::Field;
use crate::gates::arithmetic::ArithmeticExtensionGate;
use crate::generator::SimpleGenerator;
use crate::target::Target;
use crate::util::bits_u64;
use crate::witness::PartialWitness;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn double_arithmetic_extension(
        &mut self,
        const_0: F,
        const_1: F,
        fixed_multiplicand: ExtensionTarget<D>,
        multiplicand_0: ExtensionTarget<D>,
        addend_0: ExtensionTarget<D>,
        multiplicand_1: ExtensionTarget<D>,
        addend_1: ExtensionTarget<D>,
    ) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
        let gate = self.add_gate(ArithmeticExtensionGate::new(), vec![const_0, const_1]);

        let wire_fixed_multiplicand = ExtensionTarget::from_range(
            gate,
            ArithmeticExtensionGate::<D>::wires_fixed_multiplicand(),
        );
        let wire_multiplicand_0 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_multiplicand_0());
        let wire_addend_0 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_addend_0());
        let wire_multiplicand_1 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_multiplicand_1());
        let wire_addend_1 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_addend_1());
        let wire_output_0 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_output_0());
        let wire_output_1 =
            ExtensionTarget::from_range(gate, ArithmeticExtensionGate::<D>::wires_output_1());

        self.route_extension(fixed_multiplicand, wire_fixed_multiplicand);
        self.route_extension(multiplicand_0, wire_multiplicand_0);
        self.route_extension(addend_0, wire_addend_0);
        self.route_extension(multiplicand_1, wire_multiplicand_1);
        self.route_extension(addend_1, wire_addend_1);
        (wire_output_0, wire_output_1)
    }

    pub fn arithmetic_extension(
        &mut self,
        const_0: F,
        const_1: F,
        multiplicand_0: ExtensionTarget<D>,
        multiplicand_1: ExtensionTarget<D>,
        addend: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let zero = self.zero_extension();
        self.double_arithmetic_extension(
            const_0,
            const_1,
            multiplicand_0,
            multiplicand_1,
            addend,
            zero,
            zero,
        )
        .0
    }

    pub fn add_extension(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let one = self.one_extension();
        self.arithmetic_extension(F::ONE, F::ONE, one, a, b)
    }

    pub fn add_two_extension(
        &mut self,
        a0: ExtensionTarget<D>,
        b0: ExtensionTarget<D>,
        a1: ExtensionTarget<D>,
        b1: ExtensionTarget<D>,
    ) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
        let one = self.one_extension();
        self.double_arithmetic_extension(F::ONE, F::ONE, one, a0, b0, a1, b1)
    }

    pub fn add_ext_algebra(
        &mut self,
        a: ExtensionAlgebraTarget<D>,
        b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        // We run two additions in parallel. So `[a0,a1,a2,a3] + [b0,b1,b2,b3]` is computed with two
        // `add_two_extension`, first `[a0,a1]+[b0,b1]` then `[a2,a3]+[b2,b3]`.
        let mut res = Vec::with_capacity(D);
        // We need some extra logic if D is odd.
        let d_even = D & (D ^ 1); // = 2 * (D/2)
        for mut chunk in &(0..d_even).chunks(2) {
            let i = chunk.next().unwrap();
            let j = chunk.next().unwrap();
            let (o0, o1) = self.add_two_extension(a.0[i], b.0[i], a.0[j], b.0[j]);
            res.extend([o0, o1]);
        }
        if D.is_odd() {
            res.push(self.add_extension(a.0[D - 1], b.0[D - 1]));
        }
        ExtensionAlgebraTarget(res.try_into().unwrap())
    }

    pub fn add_many_extension(&mut self, terms: &[ExtensionTarget<D>]) -> ExtensionTarget<D> {
        let zero = self.zero_extension();
        let mut terms = terms.to_vec();
        if terms.len().is_odd() {
            terms.push(zero);
        }
        // We maintain two accumulators, one for the sum of even elements, and one for odd elements.
        let mut acc0 = zero;
        let mut acc1 = zero;
        for chunk in terms.chunks_exact(2) {
            (acc0, acc1) = self.add_two_extension(acc0, chunk[0], acc1, chunk[1]);
        }
        // We sum both accumulators to get the final result.
        self.add_extension(acc0, acc1)
    }

    pub fn sub_extension(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let one = self.one_extension();
        self.arithmetic_extension(F::ONE, F::NEG_ONE, one, a, b)
    }

    pub fn sub_two_extension(
        &mut self,
        a0: ExtensionTarget<D>,
        b0: ExtensionTarget<D>,
        a1: ExtensionTarget<D>,
        b1: ExtensionTarget<D>,
    ) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
        let one = self.one_extension();
        self.double_arithmetic_extension(F::ONE, F::NEG_ONE, one, a0, b0, a1, b1)
    }

    pub fn sub_ext_algebra(
        &mut self,
        a: ExtensionAlgebraTarget<D>,
        b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        // See `add_ext_algebra`.
        let mut res = Vec::with_capacity(D);
        let d_even = D & (D ^ 1); // = 2 * (D/2)
        for mut chunk in &(0..d_even).chunks(2) {
            let i = chunk.next().unwrap();
            let j = chunk.next().unwrap();
            let (o0, o1) = self.sub_two_extension(a.0[i], b.0[i], a.0[j], b.0[j]);
            res.extend([o0, o1]);
        }
        if D.is_odd() {
            res.push(self.sub_extension(a.0[D - 1], b.0[D - 1]));
        }
        ExtensionAlgebraTarget(res.try_into().unwrap())
    }

    pub fn mul_extension_with_const(
        &mut self,
        const_0: F,
        multiplicand_0: ExtensionTarget<D>,
        multiplicand_1: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let zero = self.zero_extension();
        self.double_arithmetic_extension(
            const_0,
            F::ZERO,
            multiplicand_0,
            multiplicand_1,
            zero,
            zero,
            zero,
        )
        .0
    }

    pub fn mul_extension(
        &mut self,
        multiplicand_0: ExtensionTarget<D>,
        multiplicand_1: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        self.mul_extension_with_const(F::ONE, multiplicand_0, multiplicand_1)
    }

    /// Computes `x^2`.
    pub fn square_extension(&mut self, x: ExtensionTarget<D>) -> ExtensionTarget<D> {
        self.mul_extension(x, x)
    }

    pub fn mul_ext_algebra(
        &mut self,
        a: ExtensionAlgebraTarget<D>,
        b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        let mut res = [self.zero_extension(); D];
        let w = self.constant(F::Extension::W);
        for i in 0..D {
            for j in 0..D {
                res[(i + j) % D] = if i + j < D {
                    self.mul_add_extension(a.0[i], b.0[j], res[(i + j) % D])
                } else {
                    let ai_bi = self.mul_extension(a.0[i], b.0[j]);
                    self.scalar_mul_add_extension(w, ai_bi, res[(i + j) % D])
                }
            }
        }
        ExtensionAlgebraTarget(res)
    }

    pub fn mul_many_extension(&mut self, terms: &[ExtensionTarget<D>]) -> ExtensionTarget<D> {
        let mut product = self.one_extension();
        for term in terms {
            product = self.mul_extension(product, *term);
        }
        product
    }

    /// Like `mul_add`, but for `ExtensionTarget`s.
    pub fn mul_add_extension(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
        c: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        self.arithmetic_extension(F::ONE, F::ONE, a, b, c)
    }

    /// Like `mul_add`, but for `ExtensionTarget`s.
    pub fn scalar_mul_add_extension(
        &mut self,
        a: Target,
        b: ExtensionTarget<D>,
        c: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let a_ext = self.convert_to_ext(a);
        self.arithmetic_extension(F::ONE, F::ONE, a_ext, b, c)
    }

    /// Like `mul_sub`, but for `ExtensionTarget`s.
    pub fn mul_sub_extension(
        &mut self,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
        c: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        self.arithmetic_extension(F::ONE, F::NEG_ONE, a, b, c)
    }

    /// Like `mul_sub`, but for `ExtensionTarget`s.
    pub fn scalar_mul_sub_extension(
        &mut self,
        a: Target,
        b: ExtensionTarget<D>,
        c: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let a_ext = self.convert_to_ext(a);
        self.arithmetic_extension(F::ONE, F::NEG_ONE, a_ext, b, c)
    }

    /// Returns `a * b`, where `b` is in the extension field and `a` is in the base field.
    pub fn scalar_mul_ext(&mut self, a: Target, b: ExtensionTarget<D>) -> ExtensionTarget<D> {
        let a_ext = self.convert_to_ext(a);
        self.mul_extension(a_ext, b)
    }

    /// Returns `a * b`, where `b` is in the extension of the extension field, and `a` is in the
    /// extension field.
    pub fn scalar_mul_ext_algebra(
        &mut self,
        a: ExtensionTarget<D>,
        mut b: ExtensionAlgebraTarget<D>,
    ) -> ExtensionAlgebraTarget<D> {
        for i in 0..D {
            b.0[i] = self.mul_extension(a, b.0[i]);
        }
        b
    }

    /// Exponentiate `base` to the power of `2^power_log`.
    // TODO: Test
    pub fn exp_power_of_2(
        &mut self,
        mut base: ExtensionTarget<D>,
        power_log: usize,
    ) -> ExtensionTarget<D> {
        for _ in 0..power_log {
            base = self.square_extension(base);
        }
        base
    }

    /// Exponentiate `base` to the power of a known `exponent`.
    // TODO: Test
    pub fn exp_u64_extension(
        &mut self,
        base: ExtensionTarget<D>,
        exponent: u64,
    ) -> ExtensionTarget<D> {
        let mut current = base;
        let mut product = self.one_extension();

        for j in 0..bits_u64(exponent as u64) {
            if (exponent >> j & 1) != 0 {
                product = self.mul_extension(product, current);
            }
            current = self.square_extension(current);
        }
        product
    }

    /// Computes `x / y`. Results in an unsatisfiable instance if `y = 0`.
    pub fn div_extension(
        &mut self,
        x: ExtensionTarget<D>,
        y: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let y_inv = self.inverse_extension(y);
        self.mul_extension(x, y_inv)
    }

    /// Computes `q = x / y` by witnessing `q` and requiring that `q * y = x`. This can be unsafe in
    /// some cases, as it allows `0 / 0 = <anything>`.
    pub fn div_unsafe_extension(
        &mut self,
        x: ExtensionTarget<D>,
        y: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let quotient = self.add_virtual_extension_target();
        self.add_generator(QuotientGeneratorExtension {
            numerator: x,
            denominator: y,
            quotient,
        });

        // Enforce that q y = x.
        let q_y = self.mul_extension(quotient, y);
        self.assert_equal_extension(q_y, x);

        quotient
    }

    /// Computes `1 / x`. Results in an unsatisfiable instance if `x = 0`.
    pub fn inverse_extension(&mut self, x: ExtensionTarget<D>) -> ExtensionTarget<D> {
        let inv = self.add_virtual_extension_target();
        let one = self.one_extension();
        self.add_generator(QuotientGeneratorExtension {
            numerator: one,
            denominator: x,
            quotient: inv,
        });

        // Enforce that x times its purported inverse equals 1.
        let x_inv = self.mul_extension(x, inv);
        self.assert_equal_extension(x_inv, one);

        inv
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
        PartialWitness::singleton_extension_target(self.quotient, quotient)
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
    use crate::verifier::verify;
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
        let z = x / y;
        let xt = builder.constant_extension(x);
        let yt = builder.constant_extension(y);
        let zt = builder.constant_extension(z);
        let comp_zt = builder.div_extension(xt, yt);
        let comp_zt_unsafe = builder.div_unsafe_extension(xt, yt);
        builder.assert_equal_extension(zt, comp_zt);
        builder.assert_equal_extension(zt, comp_zt_unsafe);

        let data = builder.build();
        let proof = data.prove(PartialWitness::new());

        verify(proof, &data.verifier_only, &data.common).unwrap();
    }
}
