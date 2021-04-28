use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::Integer;

use crate::field::field::Field;
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};

#[derive(Copy, Clone)]
pub struct ProthField(pub u64);

impl PartialEq for ProthField {
    fn eq(&self, other: &Self) -> bool {
        self.to_canonical_u64() == other.to_canonical_u64()
    }
}

impl Eq for ProthField {}

impl Hash for ProthField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.to_canonical_u64())
    }
}

impl Display for ProthField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for ProthField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Field for ProthField {
    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);
    const TWO: Self = Self(2);
    const NEG_ONE: Self = Self(Self::ORDER - 1);

    const ORDER: u64 = 0x280000000000001;
    const TWO_ADICITY: usize = 55;

    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self(3);
    // FIXME: Work out what this is
    const POWER_OF_TWO_GENERATOR: Self = Self(10281950781551402419);

    #[inline]
    fn square(&self) -> Self {
        *self * *self
    }

    #[inline]
    fn cube(&self) -> Self {
        *self * *self * *self
    }

    #[allow(clippy::many_single_char_names)] // The names are from the paper.
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
        // We only need one condition subtraction, since 2 * ORDER would not fit in a u64.
        if c >= Self::ORDER {
            c -= Self::ORDER;
        }
        c
    }

    #[inline]
    fn from_canonical_u64(n: u64) -> Self {
        Self(n)
    }

    fn cube_root(&self) -> Self {
        let x0 = *self;
        let x1 = x0.square();
        let x2 = x1.square();
        let x3 = x2 * x0;
        let x4 = x3.square();
        let x5 = x4.square();
        let x7 = x5.square();
        let x8 = x7.square();
        let x9 = x8.square();
        let x10 = x9.square();
        let x11 = x10 * x5;
        let x12 = x11.square();
        let x13 = x12.square();
        let x14 = x13.square();
        let x16 = x14.square();
        let x17 = x16.square();
        let x18 = x17.square();
        let x19 = x18.square();
        let x20 = x19.square();
        let x21 = x20 * x11;
        let x22 = x21.square();
        let x23 = x22.square();
        let x24 = x23.square();
        let x25 = x24.square();
        let x26 = x25.square();
        let x27 = x26.square();
        let x28 = x27.square();
        let x29 = x28.square();
        let x30 = x29.square();
        let x31 = x30.square();
        let x32 = x31.square();
        let x33 = x32 * x14;
        let x34 = x33 * x3;
        let x35 = x34.square();
        let x36 = x35 * x34;
        let x37 = x36 * x5;
        let x38 = x37 * x34;
        let x39 = x38 * x37;
        let x40 = x39.square();
        let x41 = x40.square();
        let x42 = x41 * x38;
        let x43 = x42.square();
        let x44 = x43.square();
        let x45 = x44.square();
        let x46 = x45.square();
        let x47 = x46.square();
        let x48 = x47.square();
        let x49 = x48.square();
        let x50 = x49.square();
        let x51 = x50.square();
        let x52 = x51.square();
        let x53 = x52.square();
        let x54 = x53.square();
        let x55 = x54.square();
        let x56 = x55.square();
        let x57 = x56.square();
        let x58 = x57.square();
        let x59 = x58.square();
        let x60 = x59.square();
        let x61 = x60.square();
        let x62 = x61.square();
        let x63 = x62.square();
        let x64 = x63.square();
        let x65 = x64.square();
        let x66 = x65.square();
        let x67 = x66.square();
        let x68 = x67.square();
        let x69 = x68.square();
        let x70 = x69.square();
        let x71 = x70.square();
        let x72 = x71.square();
        let x73 = x72.square();
        let x74 = x73 * x39;
        x74
    }
}

impl Neg for ProthField {
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

impl Add for ProthField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Self) -> Self {
        let (sum, over) = self.0.overflowing_add(rhs.0);
        Self(sum.overflowing_sub((over as u64) * Self::ORDER).0)
    }
}

impl AddAssign for ProthField {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for ProthField {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for ProthField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: Self) -> Self {
        let (diff, under) = self.0.overflowing_sub(rhs.0);
        Self(diff.overflowing_add((under as u64) * Self::ORDER).0)
    }
}

impl SubAssign for ProthField {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for ProthField {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        reduce128((self.0 as u128) * (rhs.0 as u128))
    }
}

impl MulAssign for ProthField {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Product for ProthField {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl Div for ProthField {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for ProthField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

/// Reduces to a 64-bit value. The result might not be in canonical form; it could be in between the
/// field order and `2^64`.
#[inline]
fn reduce128(x: u128) -> ProthField {
    const LO_57b_MASK: u64 = (1u64 << 57) - 1u64;
    let (lo, hi) = split(x);
    let C0 = lo & LO_57b_MASK;
    let C1 = (lo >> 57) | (hi << 7);
    ProthField(29 * C1 - C0)
}

#[inline]
fn split(x: u128) -> (u64, u64) {
    (x as u64, (x >> 64) as u64)
}

#[cfg(test)]
mod tests {
    use crate::test_arithmetic;

    test_arithmetic!(crate::field::proth_field::ProthField);
}
