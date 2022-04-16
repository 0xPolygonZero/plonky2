use std::ops::Mul;
use rand::Rng;
use std::convert::TryInto;
use std::fmt::Debug;

pub trait Square {
    fn square(&self) -> Self;
}

impl<F: Mul<F, Output = Self> + Copy> Square for F {
    default fn square(&self) -> Self {
        *self * *self
    }
}

pub trait Rand: Sized {
    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self;

    fn rand() -> Self {
        Self::rand_from_rng(&mut rand::thread_rng())
    }

    fn rand_arr<const N: usize>() -> [Self; N] where Self: Debug {
        // TODO: Implement allocation free
        Self::rand_vec(N).try_into().unwrap()
    }

    fn rand_vec(n: usize) -> Vec<Self> {
        (0..n).map(|_| Self::rand()).collect()
    }
}
