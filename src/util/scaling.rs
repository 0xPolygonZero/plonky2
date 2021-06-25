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

// #[derive(Debug, Copy, Clone)]
// pub struct ReducingFactorTarget<const D: usize> {
//     base: ExtensionTarget<D>,
//     count: u64,
// }
//
// impl<F: Extendable<D>, const D: usize> ReducingFactorTarget<D> {
//     pub fn new(base: ExtensionTarget<D>) -> Self {
//         Self { base, count: 0 }
//     }
//
//     fn mul(
//         &mut self,
//         x: ExtensionTarget<D>,
//         builder: &mut CircuitBuilder<F, D>,
//     ) -> ExtensionTarget<D> {
//         self.count += 1;
//         builder.mul_extension(self.base, x)
//     }
//
//     pub fn reduce(
//         &mut self,
//         iter: &[ExtensionTarget<D>], // Could probably work with a `DoubleEndedIterator` too.
//         builder: &mut CircuitBuilder<F, D>,
//     ) -> ExtensionTarget<D> {
//         let l = iter.len();
//         let padded_iter = if l % 2 == 0 {
//             iter.to_vec()
//         } else {
//             [iter, &[builder.zero_extension()]].concat()
//         };
//         let half_length = padded_iter.len() / 2;
//         let gates = (0..half_length)
//             .map(|_| builder.add_gate(ArithmeticExtensionGate::new(), vec![F::ONE, F::ONE]))
//             .collect::<Vec<_>>();
//
//         struct ParallelReductionGenerator<'a, const D: usize> {
//             base: ExtensionTarget<D>,
//             padded_iter: &'a [ExtensionTarget<D>],
//             gates: &'a [usize],
//             half_length: usize,
//         }
//
//         impl<'a, F: Extendable<D>, const D: usize> SimpleGenerator<F>
//             for ParallelReductionGenerator<'a, D>
//         {
//             fn dependencies(&self) -> Vec<Target> {
//                 self.padded_iter
//                     .iter()
//                     .flat_map(|ext| ext.to_target_array())
//                     .chain(self.base.to_target_array())
//                     .collect()
//             }
//
//             fn run_once(&self, witness: &PartialWitness<F>) -> PartialWitness<F> {
//                 let mut pw = PartialWitness::new();
//                 let base = witness.get_extension_target(self.base);
//                 let vs = self
//                     .padded_iter
//                     .iter()
//                     .map(|&ext| witness.get_extension_target(ext))
//                     .collect::<Vec<_>>();
//                 let first_half = &vs[..self.half_length];
//                 let intermediate_acc = base.reduce(first_half);
//             }
//         }
//     }
//
//     pub fn reduce_parallel(
//         &mut self,
//         iter0: impl DoubleEndedIterator<Item = impl Borrow<ExtensionTarget<D>>>,
//         iter1: impl DoubleEndedIterator<Item = impl Borrow<ExtensionTarget<D>>>,
//         builder: &mut CircuitBuilder<F, D>,
//     ) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
//         iter.rev().fold(builder.zero_extension(), |acc, x| {
//             builder.arithmetic_extension(F::ONE, F::ONE, self.base, acc, x)
//         })
//     }
//
//     pub fn shift(
//         &mut self,
//         x: ExtensionTarget<D>,
//         builder: &mut CircuitBuilder<F, D>,
//     ) -> ExtensionTarget<D> {
//         let tmp = self.base.exp(self.count) * x;
//         self.count = 0;
//         tmp
//     }
//
//     pub fn shift_poly(
//         &mut self,
//         p: &mut PolynomialCoeffs<ExtensionTarget<D>>,
//         builder: &mut CircuitBuilder<F, D>,
//     ) {
//         *p *= self.base.exp(self.count);
//         self.count = 0;
//     }
//
//     pub fn reset(&mut self) {
//         self.count = 0;
//     }
//
//     pub fn repeated_frobenius(&self, count: usize, builder: &mut CircuitBuilder<F, D>) -> Self {
//         Self {
//             base: self.base.repeated_frobenius(count),
//             count: self.count,
//         }
//     }
// }
