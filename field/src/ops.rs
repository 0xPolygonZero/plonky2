use std::ops::Mul;
use rand::Rng;

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

    fn rand_arr<const N: usize>() -> [Self; N] where Self: Default + Copy {
        // TODO: Use array MaybeUninit when stable for dependently typed arrays.
        // then we can drop the Default + Copy requirement.
        let mut result = [Self::default(); N];
        let mut rng = rand::thread_rng();
        for result in &mut result[..] {
            *result = Self::rand_from_rng(&mut rng);
        }
        result
    }

    fn rand_vec(n: usize) -> Vec<Self> {
        (0..n).map(|_| Self::rand()).collect()
    }
}
