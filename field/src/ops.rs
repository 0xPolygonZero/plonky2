use std::ops::Mul;

pub trait Squarable {
    fn square(&self) -> Self;
}

impl<F: Mul<F, Output = Self> + Copy> Squarable for F {
    default fn square(&self) -> Self {
        *self * *self
    }
}
