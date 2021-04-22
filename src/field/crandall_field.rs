use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::Integer;

use crate::field::field::Field;
use std::hash::{Hash, Hasher};

/// EPSILON = 9 * 2**28 - 1
const EPSILON: u64 = 2415919103;

/// A field designed for use with the Crandall reduction algorithm.
///
/// Its order is
/// ```ignore
/// P = 2**64 - EPSILON
///   = 2**64 - 9 * 2**28 + 1
///   = 2**28 * (2**36 - 9) + 1
/// ```
#[derive(Copy, Clone)]
pub struct CrandallField(pub u64);

impl PartialEq for CrandallField {
    fn eq(&self, other: &Self) -> bool {
        self.to_canonical_u64() == other.to_canonical_u64()
    }
}

impl Eq for CrandallField {}

impl Hash for CrandallField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.to_canonical_u64())
    }
}

impl Display for CrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for CrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Field for CrandallField {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);
    const TWO: Self = Self(2);
    const NEG_ONE: Self = Self(Self::ORDER - 1);

    const ORDER: u64 = 18446744071293632513;
    const TWO_ADICITY: usize = 28;

    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self(5);
    const POWER_OF_TWO_GENERATOR: Self = Self(10281950781551402419);

    #[inline]
    fn square(&self) -> Self {
        *self * *self
    }

    #[inline]
    fn cube(&self) -> Self {
        *self * *self * *self
    }

    fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        // Based on Algorithm 16 of "Efficient Software-Implementation of Finite Fields with
        // Applications to Cryptography".

        let p = Self::ORDER;
        let mut u = self.to_canonical_u64();
        let mut v = p;
        let mut b = 1u64;
        let mut c = 0u64;

        while u != 1 && v != 1 {
            while u.is_even() {
                u /= 2;
                if b.is_even() {
                    b /= 2;
                } else {
                    // b = (b + p)/2, avoiding overflow
                    b = (b / 2) + (p / 2) + 1;
                }
            }

            while v.is_even() {
                v /= 2;
                if c.is_even() {
                    c /= 2;
                } else {
                    // c = (c + p)/2, avoiding overflow
                    c = (c / 2) + (p / 2) + 1;
                }
            }

            if u >= v {
                u -= v;
                // b -= c
                let (mut diff, under) = b.overflowing_sub(c);
                if under {
                    diff = diff.overflowing_add(p).0;
                }
                b = diff;
            } else {
                v -= u;
                // c -= b
                let (mut diff, under) = c.overflowing_sub(b);
                if under {
                    diff = diff.overflowing_add(p).0;
                }
                c = diff;
            }
        }

        let inverse = Self(if u == 1 { b } else { c });

        // Should change to debug_assert_eq; using assert_eq as an extra precaution for now until
        // we're more confident the impl is correct.
        assert_eq!(*self * inverse, Self::ONE);
        Some(inverse)
    }

    #[inline]
    fn to_canonical_u64(&self) -> u64 {
        let mut c = self.0;
        while c >= Self::ORDER {
            c -= Self::ORDER;
        }
        c
    }

    #[inline]
    fn from_canonical_u64(n: u64) -> Self {
        Self(n)
    }
}

impl Neg for CrandallField {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        if self.is_zero() {
            Self::ZERO
        } else {
            // TODO: This could underflow if we're not canonical.
            Self(Self::ORDER - self.0)
        }
    }
}

impl Add for CrandallField {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        let (sum, over) = self.0.overflowing_add(rhs.0);
        Self(sum.overflowing_sub((over as u64) * Self::ORDER).0)
    }
}

impl AddAssign for CrandallField {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for CrandallField {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        let (diff, under) = self.0.overflowing_sub(rhs.0);
        Self(diff.overflowing_add((under as u64) * Self::ORDER).0)
    }
}

impl SubAssign for CrandallField {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for CrandallField {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        reduce128((self.0 as u128) * (rhs.0 as u128))
    }
}

impl MulAssign for CrandallField {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for CrandallField {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for CrandallField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

/// Reduces to a 64-bit value. The result might not be in canonical form; it could be in between the
/// field order and `2^64`.
#[inline]
fn reduce128(x: u128) -> CrandallField {
    // This is Crandall's algorithm. When we have some high-order bits (i.e. with a weight of 2^64),
    // we convert them to low-order bits by multiplying by EPSILON (the logic is a simple
    // generalization of Mersenne prime reduction). The first time we do this, the product will take
    // ~96 bits, so we still have some high-order bits. But when we repeat this another time, the
    // product will fit in 64 bits.
    let (lo_1, hi_1) = split(x);
    let (lo_2, hi_2) = split((EPSILON as u128) * (hi_1 as u128) + (lo_1 as u128));
    let lo_3 = hi_2 * EPSILON;

    CrandallField(lo_2) + CrandallField(lo_3)
}

#[inline]
fn split(x: u128) -> (u64, u64) {
    (x as u64, (x >> 64) as u64)
}

#[cfg(test)]
mod tests {
    use crate::test_arithmetic;

    test_arithmetic!(crate::field::crandall_field::CrandallField);
}
