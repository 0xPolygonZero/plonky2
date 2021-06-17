use std::borrow::Borrow;

use crate::field::extension_field::Frobenius;
use crate::field::field::Field;

#[derive(Copy, Clone)]
pub struct ScalingFactor<F: Field> {
    base: F,
    count: u64,
}

impl<F: Field> ScalingFactor<F> {
    pub fn new(base: F) -> Self {
        Self { base, count: 0 }
    }

    pub fn mul(&mut self, x: F) -> F {
        self.count += 1;
        self.base * x
    }

    pub fn scale(&mut self, iter: impl DoubleEndedIterator<Item = impl Borrow<F>>) -> F {
        iter.rev().fold(F::ZERO, |acc, x| self.mul(acc) + x)
    }

    pub fn shift(&mut self, x: F) -> F {
        let tmp = self.base.exp(self.count) * x;
        self.count = 0;
        tmp
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
