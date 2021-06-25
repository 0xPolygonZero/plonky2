use std::borrow::Borrow;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, Frobenius};
use crate::field::field::Field;
use crate::gates::mul_extension::ArithmeticExtensionGate;
use crate::generator::SimpleGenerator;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::target::Target;
use crate::witness::PartialWitness;

/// When verifying the composition polynomial in FRI we have to compute sums of the form
/// `(sum_0^k a^i * x_i)/d_0 + (sum_k^r a^i * y_i)/d_1`
/// The most efficient way to do this is to compute both quotient separately using Horner's method,
/// scale the second one by `a^(r-1-k)`, and add them up.
/// This struct abstract away these operations by implementing Horner's method and keeping track
/// of the number of multiplications by `a` to compute the scaling factor.
/// See https://github.com/mir-protocol/plonky2/pull/69 for more details and discussions.
#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone)]
pub struct ReducingFactorTarget<const D: usize> {
    base: ExtensionTarget<D>,
    count: u64,
}

impl<const D: usize> ReducingFactorTarget<D> {
    pub fn new(base: ExtensionTarget<D>) -> Self {
        Self { base, count: 0 }
    }

    pub fn reduce<F>(
        &mut self,
        iter: &[ExtensionTarget<D>], // Could probably work with a `DoubleEndedIterator` too.
        builder: &mut CircuitBuilder<F, D>,
    ) -> ExtensionTarget<D>
    where
        F: Extendable<D>,
    {
        let l = iter.len();
        self.count += l as u64;
        let padded_iter = if l % 2 == 0 {
            iter.to_vec()
        } else {
            [iter, &[builder.zero_extension()]].concat()
        };
        let half_length = padded_iter.len() / 2;
        let gates = (0..half_length)
            .map(|_| builder.add_gate(ArithmeticExtensionGate::new(), vec![F::ONE, F::ONE]))
            .collect::<Vec<_>>();

        builder.add_generator(ParallelReductionGenerator {
            base: self.base,
            padded_iter: padded_iter.clone(),
            gates: gates.clone(),
            half_length,
        });

        for i in 0..half_length {
            builder.route_extension(
                ExtensionTarget::from_range(
                    gates[i],
                    ArithmeticExtensionGate::<D>::wires_addend_0(),
                ),
                padded_iter[2 * half_length - i - 1],
            );
        }
        for i in 0..half_length {
            builder.route_extension(
                ExtensionTarget::from_range(
                    gates[i],
                    ArithmeticExtensionGate::<D>::wires_addend_1(),
                ),
                padded_iter[half_length - i - 1],
            );
        }
        for gate_pair in gates[..half_length].windows(2) {
            builder.assert_equal_extension(
                ExtensionTarget::from_range(
                    gate_pair[0],
                    ArithmeticExtensionGate::<D>::wires_output_0(),
                ),
                ExtensionTarget::from_range(
                    gate_pair[1],
                    ArithmeticExtensionGate::<D>::wires_multiplicand_0(),
                ),
            );
        }
        for gate_pair in gates[half_length..].windows(2) {
            builder.assert_equal_extension(
                ExtensionTarget::from_range(
                    gate_pair[0],
                    ArithmeticExtensionGate::<D>::wires_output_1(),
                ),
                ExtensionTarget::from_range(
                    gate_pair[1],
                    ArithmeticExtensionGate::<D>::wires_multiplicand_1(),
                ),
            );
        }
        builder.assert_equal_extension(
            ExtensionTarget::from_range(
                gates[half_length - 1],
                ArithmeticExtensionGate::<D>::wires_output_0(),
            ),
            ExtensionTarget::from_range(
                gates[0],
                ArithmeticExtensionGate::<D>::wires_multiplicand_1(),
            ),
        );

        ExtensionTarget::from_range(
            gates[half_length - 1],
            ArithmeticExtensionGate::<D>::wires_output_1(),
        )
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
        let tmp = builder.mul_extension(exp, x);
        self.count = 0;
        tmp
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

struct ParallelReductionGenerator<const D: usize> {
    base: ExtensionTarget<D>,
    padded_iter: Vec<ExtensionTarget<D>>,
    gates: Vec<usize>,
    half_length: usize,
}

impl<F: Extendable<D>, const D: usize> SimpleGenerator<F> for ParallelReductionGenerator<D> {
    fn dependencies(&self) -> Vec<Target> {
        self.padded_iter
            .iter()
            .flat_map(|ext| ext.to_target_array())
            .chain(self.base.to_target_array())
            .collect()
    }

    fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
        let mut pw = PartialWitness::new();
        let base = witness.get_extension_target(self.base);
        let vs = self
            .padded_iter
            .iter()
            .map(|&ext| witness.get_extension_target(ext))
            .collect::<Vec<_>>();
        let intermediate_accs = vs
            .iter()
            .rev()
            .scan(F::Extension::ZERO, |acc, &x| {
                let tmp = *acc;
                *acc = *acc * base + x;
                Some(tmp)
            })
            .collect::<Vec<_>>();
        for i in 0..self.half_length {
            pw.set_extension_target(
                ExtensionTarget::from_range(
                    self.gates[i],
                    ArithmeticExtensionGate::<D>::wires_fixed_multiplicand(),
                ),
                base,
            );
            pw.set_extension_target(
                ExtensionTarget::from_range(
                    self.gates[i],
                    ArithmeticExtensionGate::<D>::wires_multiplicand_0(),
                ),
                intermediate_accs[i],
            );
            pw.set_extension_target(
                ExtensionTarget::from_range(
                    self.gates[i],
                    ArithmeticExtensionGate::<D>::wires_addend_0(),
                ),
                vs[2 * self.half_length - i - 1],
            );
            pw.set_extension_target(
                ExtensionTarget::from_range(
                    self.gates[i],
                    ArithmeticExtensionGate::<D>::wires_multiplicand_1(),
                ),
                intermediate_accs[self.half_length + i],
            );
            pw.set_extension_target(
                ExtensionTarget::from_range(
                    self.gates[i],
                    ArithmeticExtensionGate::<D>::wires_addend_1(),
                ),
                vs[self.half_length - i - 1],
            );
        }

        pw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;

    fn test_reduce_gadget(n: usize) {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_config();

        let mut builder = CircuitBuilder::<F, D>::new(config);

        let alpha = FF::rand();
        let alpha = FF::ONE;
        let vs = (0..n)
            .map(|i| FF::from_canonical_usize(i))
            .collect::<Vec<_>>();

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
        let proof = data.prove(PartialWitness::new());
    }

    #[test]
    fn test_reduce_gadget_even() {
        test_reduce_gadget(10);
    }

    #[test]
    fn test_reduce_gadget_odd() {
        test_reduce_gadget(11);
    }
}
