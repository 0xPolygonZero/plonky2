use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::bigint::BigUint;
use num::Integer;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::quadratic::QuadraticCrandallField;
use crate::field::extension_field::quartic::QuarticCrandallField;
use crate::field::extension_field::{Extendable, Frobenius};
use crate::field::field_types::{Field, PrimeField};

/// EPSILON = 9 * 2**28 - 1
const EPSILON: u64 = 2415919103;

/// A precomputed 8*8 Cauchy matrix, generated with `Field::mds_8`.
const CAUCHY_MDS_8: [[CrandallField; 8]; 8] = [
    [
        CrandallField(16140901062381928449),
        CrandallField(2635249153041947502),
        CrandallField(3074457345215605419),
        CrandallField(11068046442776179508),
        CrandallField(13835058053470224385),
        CrandallField(6148914690431210838),
        CrandallField(9223372035646816257),
        CrandallField(1),
    ],
    [
        CrandallField(2049638230143736946),
        CrandallField(16140901062381928449),
        CrandallField(2635249153041947502),
        CrandallField(3074457345215605419),
        CrandallField(11068046442776179508),
        CrandallField(13835058053470224385),
        CrandallField(6148914690431210838),
        CrandallField(9223372035646816257),
    ],
    [
        CrandallField(5534023221388089754),
        CrandallField(2049638230143736946),
        CrandallField(16140901062381928449),
        CrandallField(2635249153041947502),
        CrandallField(3074457345215605419),
        CrandallField(11068046442776179508),
        CrandallField(13835058053470224385),
        CrandallField(6148914690431210838),
    ],
    [
        CrandallField(16769767337539665921),
        CrandallField(5534023221388089754),
        CrandallField(2049638230143736946),
        CrandallField(16140901062381928449),
        CrandallField(2635249153041947502),
        CrandallField(3074457345215605419),
        CrandallField(11068046442776179508),
        CrandallField(13835058053470224385),
    ],
    [
        CrandallField(10760600708254618966),
        CrandallField(16769767337539665921),
        CrandallField(5534023221388089754),
        CrandallField(2049638230143736946),
        CrandallField(16140901062381928449),
        CrandallField(2635249153041947502),
        CrandallField(3074457345215605419),
        CrandallField(11068046442776179508),
    ],
    [
        CrandallField(5675921252705733081),
        CrandallField(10760600708254618966),
        CrandallField(16769767337539665921),
        CrandallField(5534023221388089754),
        CrandallField(2049638230143736946),
        CrandallField(16140901062381928449),
        CrandallField(2635249153041947502),
        CrandallField(3074457345215605419),
    ],
    [
        CrandallField(1317624576520973751),
        CrandallField(5675921252705733081),
        CrandallField(10760600708254618966),
        CrandallField(16769767337539665921),
        CrandallField(5534023221388089754),
        CrandallField(2049638230143736946),
        CrandallField(16140901062381928449),
        CrandallField(2635249153041947502),
    ],
    [
        CrandallField(15987178195121148178),
        CrandallField(1317624576520973751),
        CrandallField(5675921252705733081),
        CrandallField(10760600708254618966),
        CrandallField(16769767337539665921),
        CrandallField(5534023221388089754),
        CrandallField(2049638230143736946),
        CrandallField(16140901062381928449),
    ],
];

/// A field designed for use with the Crandall reduction algorithm.
///
/// Its order is
/// ```ignore
/// P = 2**64 - EPSILON
///   = 2**64 - 9 * 2**28 + 1
///   = 2**28 * (2**36 - 9) + 1
/// ```
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct CrandallField(pub u64);

impl Default for CrandallField {
    fn default() -> Self {
        Self::ZERO
    }
}

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
        Display::fmt(&self.to_canonical_u64(), f)
    }
}

impl Debug for CrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.to_canonical_u64(), f)
    }
}

impl Field for CrandallField {
    type PrimeField = Self;

    const ZERO: Self = Self(0);
    const ONE: Self = Self(1);
    const TWO: Self = Self(2);
    const NEG_ONE: Self = Self(Self::ORDER - 1);

    const CHARACTERISTIC: u64 = Self::ORDER;
    const TWO_ADICITY: usize = 28;

    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self(5);
    const POWER_OF_TWO_GENERATOR: Self = Self(10281950781551402419);

    fn order() -> BigUint {
        BigUint::from(Self::ORDER)
    }

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
            let u_tz = u.trailing_zeros();
            u >>= u_tz;
            for _ in 0..u_tz {
                if b.is_even() {
                    b /= 2;
                } else {
                    // b = (b + p)/2, avoiding overflow
                    b = (b / 2) + (p / 2) + 1;
                }
            }

            let v_tz = v.trailing_zeros();
            v >>= v_tz;
            for _ in 0..v_tz {
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
    fn from_canonical_u64(n: u64) -> Self {
        Self(n)
    }

    fn from_canonical_biguint(n: BigUint) -> Self {
        Self(n.iter_u64_digits().next().unwrap_or(0))
    }

    #[inline]
    fn from_noncanonical_u128(n: u128) -> Self {
        reduce128(n)
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self::from_canonical_u64(rng.gen_range(0..Self::ORDER))
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

    fn mds_8(vec: [Self; 8]) -> [Self; 8] {
        let mut result = [Self::ZERO; 8];
        for r in 0..8 {
            for c in 0..8 {
                let entry = CAUCHY_MDS_8[r][c];
                result[r] += entry * vec[c];
            }
        }
        result
    }
}

impl PrimeField for CrandallField {
    const ORDER: u64 = 18446744071293632513;

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
    fn to_noncanonical_u64(&self) -> u64 {
        self.0
    }

    fn to_canonical_biguint(&self) -> BigUint {
        BigUint::from(self.to_canonical_u64())
    }
}

impl Neg for CrandallField {
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

impl Add for CrandallField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Self) -> Self {
        let (sum, over) = self.0.overflowing_add(rhs.to_canonical_u64());
        Self(sum.overflowing_sub((over as u64) * Self::ORDER).0)
    }
}

impl AddAssign for CrandallField {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for CrandallField {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for CrandallField {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: Self) -> Self {
        let (diff, under) = self.0.overflowing_sub(rhs.to_canonical_u64());
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

impl Product for CrandallField {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl Div for CrandallField {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for CrandallField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Extendable<2> for CrandallField {
    type Extension = QuadraticCrandallField;
}

impl Extendable<4> for CrandallField {
    type Extension = QuarticCrandallField;
}

/// Faster addition for when we know that lhs.0 + rhs.0 < 2^64 + Self::ORDER. If this is the case,
/// then the .to_canonical_u64() that addition usually performs is unnecessary. Omitting it saves
/// three instructions.
/// This function is marked unsafe because it may yield incorrect result if the condition is not
/// satisfied.
#[inline]
unsafe fn add_no_canonicalize(lhs: CrandallField, rhs: CrandallField) -> CrandallField {
    let (sum, over) = lhs.0.overflowing_add(rhs.0);
    CrandallField(sum.overflowing_sub((over as u64) * CrandallField::ORDER).0)
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

    unsafe {
        // This is safe to do because lo_2 + lo_3 < 2^64 + Self::ORDER. Notice that hi_2 <=
        // 2^32 - 1. Then lo_3 = hi_2 * EPSILON <= (2^32 - 1) * EPSILON < Self::ORDER.
        // Use of standard addition here would make multiplication 20% more expensive.
        add_no_canonicalize(CrandallField(lo_2), CrandallField(lo_3))
    }
}

#[inline]
fn split(x: u128) -> (u64, u64) {
    (x as u64, (x >> 64) as u64)
}

impl Frobenius<1> for CrandallField {}

#[cfg(test)]
mod tests {
    use crate::{test_field_arithmetic, test_prime_field_arithmetic};

    test_prime_field_arithmetic!(crate::field::crandall_field::CrandallField);
    test_field_arithmetic!(crate::field::crandall_field::CrandallField);
}
