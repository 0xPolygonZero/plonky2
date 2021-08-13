use std::borrow::Borrow;

use num::Integer;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, Frobenius};
use crate::field::field_types::Field;
use crate::gates::arithmetic::ArithmeticExtensionGate;
use crate::gates::reducing::ReducingGate;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::polynomial::polynomial::PolynomialCoeffs;

/// When verifying the composition polynomial in FRI we have to compute sums of the form
/// `(sum_0^k a^i * x_i)/d_0 + (sum_k^r a^i * y_i)/d_1`
/// The most efficient way to do this is to compute both quotient separately using Horner's method,
/// scale the second one by `a^(r-1-k)`, and add them up.
/// This struct abstract away these operations by implementing Horner's method and keeping track
/// of the number of multiplications by `a` to compute the scaling factor.
/// See https://github.com/mir-protocol/plonky2/pull/69 for more details and discussions.
#[derive(Debug, Clone)]
pub struct ReducingFactor<F: Field> {
    base: F,
    count: u64,
}

impl<F: Field> ReducingFactor<F> {
    pub fn new(base: F) -> Self {
        Self { base, count: 0 }
    }

    fn mul(&mut self, x: F) -> F {
        self.count += 1;
        self.base * x
    }

    fn mul_poly(&mut self, p: &mut PolynomialCoeffs<F>) {
        self.count += 1;
        *p *= self.base;
    }

    pub fn reduce(&mut self, iter: impl DoubleEndedIterator<Item = impl Borrow<F>>) -> F {
        iter.rev()
            .fold(F::ZERO, |acc, x| self.mul(acc) + *x.borrow())
    }

    pub fn reduce_polys(
        &mut self,
        polys: impl DoubleEndedIterator<Item = impl Borrow<PolynomialCoeffs<F>>>,
    ) -> PolynomialCoeffs<F> {
        polys.rev().fold(PolynomialCoeffs::empty(), |mut acc, x| {
            self.mul_poly(&mut acc);
            acc += x.borrow();
            acc
        })
    }

    pub fn reduce_polys_base<BF: Extendable<D, Extension = F>, const D: usize>(
        &mut self,
        polys: impl IntoIterator<Item = impl Borrow<PolynomialCoeffs<BF>>>,
    ) -> PolynomialCoeffs<F> {
        self.base
            .powers()
            .zip(polys)
            .map(|(base_power, poly)| {
                self.count += 1;
                poly.borrow().mul_extension(base_power)
            })
            .sum()
    }

    pub fn shift(&mut self, x: F) -> F {
        let tmp = self.base.exp(self.count) * x;
        self.count = 0;
        tmp
    }

    pub fn shift_poly(&mut self, p: &mut PolynomialCoeffs<F>) {
        *p *= self.base.exp(self.count);
        self.count = 0;
    }

    pub fn reset(&mut self) {
        self.count = 0;
    }

    pub fn repeated_frobenius<const D: usize>(&self, count: usize) -> Self
    where
        F: Frobenius<D>,
    {
        Self {
            base: self.base.repeated_frobenius(count),
            count: self.count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReducingFactorTarget<const D: usize> {
    base: ExtensionTarget<D>,
    count: u64,
}

impl<const D: usize> ReducingFactorTarget<D> {
    pub fn new(base: ExtensionTarget<D>) -> Self {
        Self { base, count: 0 }
    }

    /// Reduces a length `n` vector of `Target`s using `n/21` `ReducingGate`s (with 33 routed wires and 126 wires).
    pub fn reduce_base<F>(
        &mut self,
        terms: &[Target],
        builder: &mut CircuitBuilder<F, D>,
    ) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        let max_coeffs_len = ReducingGate::<D>::max_coeffs_len(
            builder.config.num_wires,
            builder.config.num_routed_wires,
        );
        self.count += terms.len() as u64;
        let zero = builder.zero();
        let zero_ext = builder.zero_extension();
        let mut acc = zero_ext;
        let mut reversed_terms = terms.to_vec();
        while reversed_terms.len() % max_coeffs_len != 0 {
            reversed_terms.push(zero);
        }
        reversed_terms.reverse();
        for chunk in reversed_terms.chunks_exact(max_coeffs_len) {
            let gate = ReducingGate::new(max_coeffs_len);
            let gate_index = builder.add_gate(gate.clone(), Vec::new());

            builder.route_extension(
                self.base,
                ExtensionTarget::from_range(gate_index, ReducingGate::<D>::wires_alpha()),
            );
            builder.route_extension(
                acc,
                ExtensionTarget::from_range(gate_index, ReducingGate::<D>::wires_old_acc()),
            );
            for (&t, c) in chunk.iter().zip(gate.wires_coeffs()) {
                builder.route(t, Target::wire(gate_index, c));
            }

            acc = ExtensionTarget::from_range(gate_index, ReducingGate::<D>::wires_output());
        }

        acc
    }

    /// Reduces a length `n` vector of `ExtensionTarget`s using `n/2` `ArithmeticExtensionGate`s.
    /// It does this by batching two steps of Horner's method in each gate.
    /// Here's an example with `n=4, alpha=2, D=1`:
    /// 1st gate: 2  0 4  4 3  4 11 <- 2*0+4=4, 2*4+3=11
    /// 2nd gate: 2 11 2 24 1 24 49 <- 2*11+2=24, 2*24+1=49
    /// which verifies that `2.reduce([1,2,3,4]) = 49`.
    pub fn reduce<F>(
        &mut self,
        terms: &[ExtensionTarget<D>], // Could probably work with a `DoubleEndedIterator` too.
        builder: &mut CircuitBuilder<F, D>,
    ) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        let zero = builder.zero_extension();
        let l = terms.len();
        self.count += l as u64;

        let mut terms_vec = terms.to_vec();
        // If needed, we pad the original vector so that it has even length.
        if terms_vec.len().is_odd() {
            terms_vec.push(zero);
        }
        terms_vec.reverse();

        let mut acc = zero;
        for pair in terms_vec.chunks(2) {
            // We will route the output of the first arithmetic operation to the multiplicand of the
            // second, i.e. we compute the following:
            //     out_0 = alpha acc + pair[0]
            //     acc' = out_1 = alpha out_0 + pair[1]

            let (gate, range) = if let Some((g, c_0, c_1)) = builder.free_arithmetic {
                if c_0 == F::ONE && c_1 == F::ONE {
                    (g, ArithmeticExtensionGate::<D>::wires_third_output())
                } else {
                    (
                        builder.num_gates(),
                        ArithmeticExtensionGate::<D>::wires_first_output(),
                    )
                }
            } else {
                (
                    builder.num_gates(),
                    ArithmeticExtensionGate::<D>::wires_first_output(),
                )
            };
            let out_0 = ExtensionTarget::from_range(gate, range);
            acc = builder
                .double_arithmetic_extension(
                    F::ONE,
                    F::ONE,
                    self.base,
                    acc,
                    pair[0],
                    self.base,
                    out_0,
                    pair[1],
                )
                .1;
        }
        acc
    }

    pub fn shift<F>(
        &mut self,
        x: ExtensionTarget<D>,
        builder: &mut CircuitBuilder<F, D>,
    ) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        let exp = builder.exp_u64_extension(self.base, self.count);
        self.count = 0;
        builder.mul_extension(exp, x)
    }

    pub fn shift_and_mul<F>(
        &mut self,
        x: ExtensionTarget<D>,
        a: ExtensionTarget<D>,
        b: ExtensionTarget<D>,
        builder: &mut CircuitBuilder<F, D>,
    ) -> (ExtensionTarget<D>, ExtensionTarget<D>)
    where
        F: Extendable<D>,
    {
        let exp = builder.exp_u64_extension(self.base, self.count);
        self.count = 0;
        builder.mul_two_extension(exp, x, a, b)
    }

    pub fn reset(&mut self) {
        self.count = 0;
    }

    pub fn repeated_frobenius<F>(&self, count: usize, builder: &mut CircuitBuilder<F, D>) -> Self
    where
        F: Extendable<D>,
    {
        Self {
            base: self.base.repeated_frobenius(count, builder),
            count: self.count,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    fn test_reduce_gadget_base(n: usize) -> Result<()> {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_config();

        let pw = PartialWitness::new(config.num_wires);
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let alpha = FF::rand();
        let vs = F::rand_vec(n);

        let manual_reduce = ReducingFactor::new(alpha).reduce(vs.iter().map(|&v| FF::from(v)));
        let manual_reduce = builder.constant_extension(manual_reduce);

        let mut alpha_t = ReducingFactorTarget::new(builder.constant_extension(alpha));
        let vs_t = vs.iter().map(|&v| builder.constant(v)).collect::<Vec<_>>();
        let circuit_reduce = alpha_t.reduce_base(&vs_t, &mut builder);

        builder.assert_equal_extension(manual_reduce, circuit_reduce);

        let data = builder.build();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }

    fn test_reduce_gadget(n: usize) -> Result<()> {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_config();

        let pw = PartialWitness::new(config.num_wires);
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let alpha = FF::rand();
        let vs = (0..n).map(FF::from_canonical_usize).collect::<Vec<_>>();

        let manual_reduce = ReducingFactor::new(alpha).reduce(vs.iter());
        let manual_reduce = builder.constant_extension(manual_reduce);

        let mut alpha_t = ReducingFactorTarget::new(builder.constant_extension(alpha));
        let vs_t = vs
            .iter()
            .map(|&v| builder.constant_extension(v))
            .collect::<Vec<_>>();
        let circuit_reduce = alpha_t.reduce(&vs_t, &mut builder);

        builder.assert_equal_extension(manual_reduce, circuit_reduce);

        let data = builder.build();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_reduce_gadget_even() -> Result<()> {
        test_reduce_gadget(10)
    }

    #[test]
    fn test_reduce_gadget_odd() -> Result<()> {
        test_reduce_gadget(11)
    }

    #[test]
    fn test_reduce_gadget_base_100() -> Result<()> {
        test_reduce_gadget_base(100)
    }
}
