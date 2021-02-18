use std::fmt::Debug;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A finite field with prime order less than 2^64.
pub trait Field: 'static
+ Copy
+ Clone
+ Eq
+ Neg<Output=Self>
+ Add<Self, Output=Self>
+ AddAssign<Self>
+ Sub<Self, Output=Self>
+ SubAssign<Self>
+ Mul<Self, Output=Self>
+ MulAssign<Self>
+ Div<Self, Output=Self>
+ DivAssign<Self>
+ Debug {
    const ZERO: Self;
    const ONE: Self;
    const NEG_ONE: Self;

    fn sq(&self) -> Self;

    fn cube(&self) -> Self;

    /// Compute the multiplicative inverse of this field element.
    fn try_inverse(&self) -> Option<Self>;

    fn inverse(&self) -> Self {
        self.try_inverse().expect("Tried to invert zero")
    }

    fn primitive_root_of_unity(n_power: usize) -> Self;

    fn cyclic_subgroup_known_order(generator: Self, order: usize) -> Vec<Self>;

    fn to_canonical_u64(&self) -> u64;

    fn from_canonical_u64(n: u64) -> Self;

    fn from_canonical_usize(n: usize) -> Self {
        Self::from_canonical_u64(n as u64)
    }

    fn bits(&self) -> usize {
        64 - self.to_canonical_u64().leading_zeros() as usize
    }

    fn exp(&self, power: Self) -> Self {
        let mut current = *self;
        let mut product = Self::ONE;

        for j in 0..power.bits() {
            if (power.to_canonical_u64() >> j & 1) != 0 {
                product = product * current;
            }
            current = current.sq();
        }
        product
    }

    fn exp_usize(&self, power: usize) -> Self {
        self.exp(Self::from_canonical_usize(power))
    }
}
