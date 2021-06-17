use std::borrow::Borrow;

use crate::field::extension_field::Frobenius;
use crate::field::field::Field;
use crate::polynomial::polynomial::PolynomialCoeffs;

#[derive(Debug, Copy, Clone)]
pub struct ScalingFactor<F: Field> {
    base: F,
    count: u64,
}

impl<F: Field> ScalingFactor<F> {
    pub fn new(base: F) -> Self {
        Self { base, count: 0 }
    }

    fn mul(&mut self, x: F) -> F {
        self.count += 1;
        self.base * x
    }

    fn mul_poly(&mut self, p: PolynomialCoeffs<F>) -> PolynomialCoeffs<F> {
        self.count += 1;
        &p * self.base
    }

    pub fn scale(&mut self, iter: impl DoubleEndedIterator<Item = impl Borrow<F>>) -> F {
        iter.rev()
            .fold(F::ZERO, |acc, x| self.mul(acc) + *x.borrow())
    }

    pub fn scale_polys(
        &mut self,
        polys: impl DoubleEndedIterator<Item = impl Borrow<PolynomialCoeffs<F>>>,
    ) -> PolynomialCoeffs<F> {
        polys.rev().fold(PolynomialCoeffs::empty(), |acc, x| {
            &self.mul_poly(acc) + x.borrow()
        })
    }

    pub fn shift(&mut self, x: F) -> F {
        let tmp = self.base.exp(self.count) * x;
        self.count = 0;
        tmp
    }

    pub fn shift_poly(&mut self, p: PolynomialCoeffs<F>) -> PolynomialCoeffs<F> {
        let tmp = &p * self.base.exp(self.count);
        self.count = 0;
        tmp
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
