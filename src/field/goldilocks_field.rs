use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::BigUint;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::field_types::{Field, PrimeField};
use crate::field::inversion::try_inverse_u64;

/// A field selected to have fast reduction.
///
/// Its order is 2^64 - 2^32 + 1.
/// ```ignore
/// P = 2**64 - EPSILON
///   = 2**64 - 2**32 + 1
///   = 2**32 * (2**32 - 1) + 1
/// ```
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct GoldilocksField(pub u64);

impl Default for GoldilocksField {
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for GoldilocksField {
    fn eq(&self, other: &Self) -> bool {
        self.to_canonical_u64() == other.to_canonical_u64()
    }
}

impl Eq for GoldilocksField {}

impl Hash for GoldilocksField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.to_canonical_u64())
    }
}

impl Display for GoldilocksField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for GoldilocksField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Field for GoldilocksField {
    type PrimeField = Self;

    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);
    const TWO: Self = Self(2);
    const NEG_ONE: Self = Self(Self::ORDER - 1);
    const CHARACTERISTIC: u64 = Self::ORDER;

    const TWO_ADICITY: usize = 32;

    // Sage: `g = GF(p).multiplicative_generator()`
    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self(7);

    // Sage:
    // ```
    // g_2 = g^((p - 1) / 2^32)
    // g_2.multiplicative_order()
    // ```
    const POWER_OF_TWO_GENERATOR: Self = Self(1753635133440165772);

    fn order() -> BigUint {
        Self::ORDER.into()
    }

    fn try_inverse(&self) -> Option<Self> {
        try_inverse_u64(self.0, Self::ORDER).map(|inv| Self(inv))
    }

    #[inline]
    fn from_canonical_u64(n: u64) -> Self {
        Self(n)
    }

    fn from_noncanonical_u128(n: u128) -> Self {
        reduce128(n)
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self::from_canonical_u64(rng.gen_range(0..Self::ORDER))
    }
}

impl PrimeField for GoldilocksField {
    const ORDER: u64 = 0xFFFFFFFF00000001;

    #[inline]
    fn to_canonical_u64(&self) -> u64 {
        let mut c = self.0;
        // We only need one condition subtraction, since 2 * ORDER would not fit in a u64.
        if c >= Self::ORDER {
            c -= Self::ORDER;
        }
        c
    }

    fn to_noncanonical_u64(&self) -> u64 {
        self.0
    }
}

impl Neg for GoldilocksField {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        if self.is_zero() {
            Self::ZERO
        } else {
            Self(Self::ORDER - self.to_canonical_u64())
        }
    }
}

impl Add for GoldilocksField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Self) -> Self {
        let (sum, over) = self.0.overflowing_add(rhs.to_canonical_u64());
        Self(sum.overflowing_sub((over as u64) * Self::ORDER).0)
    }
}

impl AddAssign for GoldilocksField {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for GoldilocksField {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for GoldilocksField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: Self) -> Self {
        let (diff, under) = self.0.overflowing_sub(rhs.to_canonical_u64());
        Self(diff.overflowing_add((under as u64) * Self::ORDER).0)
    }
}

impl SubAssign for GoldilocksField {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for GoldilocksField {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        reduce128((self.0 as u128) * (rhs.0 as u128))
    }
}

impl MulAssign for GoldilocksField {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Product for GoldilocksField {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl Div for GoldilocksField {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for GoldilocksField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

/// Reduces to a 64-bit value. The result might not be in canonical form; it could be in between the
/// field order and `2^64`.
#[inline]
fn reduce128(x: u128) -> GoldilocksField {
    // Write x = a0 + a1*2^64  (0 <= a0, a1 < 2^64)
    //         = b0 + b1*2^32 + b2*2^64 + b3*2^96  (0 <= b0, b1, b2, b3 < 2^32)
    const MASK_LO_32_BITS: u64 = (1u64 << 32) - 1u64;
    let (a0, a1) = split(x); // This is a no-op
    let b3 = a1 >> 32;
    let b2 = a1 & MASK_LO_32_BITS;
    let x = (b2 << 32) - b2; // Can't underflow
    let (y, cy) = x.overflowing_add(a0);
    let (mut z, br) = y.overflowing_sub(b3);

    if cy {
        z = z.overflowing_sub(GoldilocksField::ORDER).0;
    }
    if br {
        z = z.overflowing_add(GoldilocksField::ORDER).0;
    }
    GoldilocksField(z)
}

#[inline]
fn split(x: u128) -> (u64, u64) {
    (x as u64, (x >> 64) as u64)
}

#[cfg(test)]
mod tests {
    use crate::{test_field_arithmetic, test_prime_field_arithmetic};

    test_prime_field_arithmetic!(crate::field::goldilocks_field::GoldilocksField);
    test_field_arithmetic!(crate::field::goldilocks_field::GoldilocksField);
}
