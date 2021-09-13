use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::bigint::BigUint;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::quadratic::QuadraticExtension;
use crate::field::extension_field::quartic::QuarticExtension;
use crate::field::extension_field::{Extendable, Frobenius};
use crate::field::field_types::{Field, PrimeField, RichField};
use crate::field::inversion::try_inverse_u64;

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
#[derive(Copy, Clone, Serialize, Deserialize)]
#[repr(transparent)] // Must be compatible with PackedCrandallAVX2
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
        Self::ORDER.into()
    }

    fn try_inverse(&self) -> Option<Self> {
        try_inverse_u64(self.0, Self::ORDER).map(|inv| Self(inv))
    }

    #[inline]
    fn from_canonical_u64(n: u64) -> Self {
        Self(n)
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
        Self(sum.wrapping_sub((over as u64) * Self::ORDER))
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
        Self(diff.wrapping_add((under as u64) * Self::ORDER))
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
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|acc, x| acc * x).unwrap_or(Self::ONE)
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
    type Extension = QuadraticExtension<Self>;

    // Verifiable in Sage with
    // `R.<x> = GF(p)[]; assert (x^2 - 3).is_irreducible()`.
    const W: Self = Self(3);

    const EXT_MULTIPLICATIVE_GROUP_GENERATOR: [Self; 2] =
        [Self(6483724566312148654), Self(12194665049945415126)];

    const EXT_POWER_OF_TWO_GENERATOR: [Self; 2] = [Self(0), Self(14420468973723774561)];
}

impl Extendable<4> for CrandallField {
    type Extension = QuarticExtension<Self>;

    const W: Self = Self(3);

    const EXT_MULTIPLICATIVE_GROUP_GENERATOR: [Self; 4] = [
        Self(12476589904174392631),
        Self(896937834427772243),
        Self(7795248119019507390),
        Self(9005769437373554825),
    ];

    const EXT_POWER_OF_TWO_GENERATOR: [Self; 4] =
        [Self(0), Self(0), Self(0), Self(15170983443234254033)];
}

impl RichField for CrandallField {}

/// Faster addition for when we know that lhs.0 + rhs.0 < 2^64 + FIELD_ORDER. If this is the case,
/// then the .to_canonical_u64() that addition usually performs is unnecessary. Omitting it saves
/// three instructions.
/// This function is marked unsafe because it may yield incorrect result if the condition is not
/// satisfied.
#[inline]
unsafe fn add_no_canonicalize(lhs: CrandallField, rhs: CrandallField) -> CrandallField {
    let (sum, over) = lhs.0.overflowing_add(rhs.0);
    CrandallField(sum.wrapping_sub((over as u64) * CrandallField::ORDER))
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
