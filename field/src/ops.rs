use core::ops::Mul;

pub trait Square {
    fn square(&self) -> Self;
}

impl<F: Mul<F, Output = Self> + Copy> Square for F {
    default fn square(&self) -> Self {
        *self * *self
    }
}
