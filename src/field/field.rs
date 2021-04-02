use std::fmt::{Debug, Display};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use rand::Rng;
use rand::rngs::OsRng;

/// A finite field with prime order less than 2^64.
pub trait Field: 'static
+ Copy
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
+ Debug
+ Display
+ Send
+ Sync {
    const ZERO: Self;
    const ONE: Self;
    const TWO: Self;
    const NEG_ONE: Self;

    const ORDER: u64;
    const MULTIPLICATIVE_SUBGROUP_GENERATOR: Self;

    fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }

    fn is_one(&self) -> bool {
        *self == Self::ONE
    }

    fn sq(&self) -> Self {
        *self * *self
    }

    fn cube(&self) -> Self {
        *self * *self * *self
    }

    /// Compute the multiplicative inverse of this field element.
    fn try_inverse(&self) -> Option<Self>;

    fn inverse(&self) -> Self {
        self.try_inverse().expect("Tried to invert zero")
    }

    fn batch_multiplicative_inverse(x: &[Self]) -> Vec<Self> {
        // This is Montgomery's trick. At a high level, we invert the product of the given field
        // elements, then derive the individual inverses from that via multiplication.

        let n = x.len();
        if n == 0 {
            return Vec::new();
        }

        let mut a = Vec::with_capacity(n);
        a.push(x[0]);
        for i in 1..n {
            a.push(a[i - 1] * x[i]);
        }

        let mut a_inv = vec![Self::ZERO; n];
        a_inv[n - 1] = a[n - 1].try_inverse().expect("No inverse");
        for i in (0..n - 1).rev() {
            a_inv[i] = x[i + 1] * a_inv[i + 1];
        }

        let mut x_inv = Vec::with_capacity(n);
        x_inv.push(a_inv[0]);
        for i in 1..n {
            x_inv.push(a[i - 1] * a_inv[i]);
        }
        x_inv
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

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self::from_canonical_u64(rng.gen_range(0, Self::ORDER))
    }

    fn rand() -> Self {
        Self::rand_from_rng(&mut OsRng)
    }
}
