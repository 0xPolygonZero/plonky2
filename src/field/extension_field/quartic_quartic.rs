use crate::field::crandall_field::CrandallField;
use crate::field::extension_field::quartic::QuarticCrandallField;
use crate::field::extension_field::{FieldExtension, OEF};
use crate::field::field::Field;
use rand::Rng;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A quartic extension of `QuarticCrandallField`.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct QuarticQuarticCrandallField(pub(crate) [QuarticCrandallField; 4]);

impl OEF<4> for QuarticQuarticCrandallField {
    // Verifiable in Sage with
    //     p = 2^64 - 9 * 2^28 + 1
    //     F = GF(p)
    //     PR_F.<x> = PolynomialRing(F)
    //     assert (x^4 - 3).is_irreducible()
    //     F4.<y> = F.extension(x^4 - 3)
    //     PR_F4.<z> = PolynomialRing(F4)
    //     assert (x^4 - y).is_irreducible()
    const W: QuarticCrandallField = QuarticCrandallField([
        CrandallField(0),
        CrandallField(1),
        CrandallField(0),
        CrandallField(0),
    ]);
}

impl FieldExtension<4> for QuarticQuarticCrandallField {
    type BaseField = QuarticCrandallField;

    fn to_basefield_array(&self) -> [Self::BaseField; 4] {
        todo!()
    }

    fn from_basefield_array(arr: [Self::BaseField; 4]) -> Self {
        todo!()
    }

    fn from_basefield(x: Self::BaseField) -> Self {
        todo!()
    }
}

impl From<<Self as FieldExtension<4>>::BaseField> for QuarticQuarticCrandallField {
    fn from(x: <Self as FieldExtension<4>>::BaseField) -> Self {
        Self([
            x,
            <Self as FieldExtension<4>>::BaseField::ZERO,
            <Self as FieldExtension<4>>::BaseField::ZERO,
            <Self as FieldExtension<4>>::BaseField::ZERO,
        ])
    }
}

impl Field for QuarticQuarticCrandallField {
    const ZERO: Self = Self([QuarticCrandallField::ZERO; 4]);
    const ONE: Self = Self([
        QuarticCrandallField::ONE,
        QuarticCrandallField::ZERO,
        QuarticCrandallField::ZERO,
        QuarticCrandallField::ZERO,
    ]);
    const TWO: Self = Self([
        QuarticCrandallField::TWO,
        QuarticCrandallField::ZERO,
        QuarticCrandallField::ZERO,
        QuarticCrandallField::ZERO,
    ]);
    const NEG_ONE: Self = Self([
        QuarticCrandallField::NEG_ONE,
        QuarticCrandallField::ZERO,
        QuarticCrandallField::ZERO,
        QuarticCrandallField::ZERO,
    ]);

    // Does not fit in 64-bits.
    const ORDER: u64 = 0;
    const TWO_ADICITY: usize = 32;
    // TODO
    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self::ZERO;
    // TODO
    const POWER_OF_TWO_GENERATOR: Self = Self::ZERO;

    fn try_inverse(&self) -> Option<Self> {
        todo!()
    }

    fn to_canonical_u64(&self) -> u64 {
        panic!("Doesn't fit!")
    }

    fn from_canonical_u64(n: u64) -> Self {
        <Self as FieldExtension<4>>::BaseField::from_canonical_u64(n).into()
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self([
            <Self as FieldExtension<4>>::BaseField::rand_from_rng(rng),
            <Self as FieldExtension<4>>::BaseField::rand_from_rng(rng),
            <Self as FieldExtension<4>>::BaseField::rand_from_rng(rng),
            <Self as FieldExtension<4>>::BaseField::rand_from_rng(rng),
        ])
    }
}

impl Display for QuarticQuarticCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} + {}*a + {}*a^2 + {}*a^3",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

impl Debug for QuarticQuarticCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Neg for QuarticQuarticCrandallField {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1], -self.0[2], -self.0[3]])
    }
}

impl Add for QuarticQuarticCrandallField {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
        ])
    }
}

impl AddAssign for QuarticQuarticCrandallField {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for QuarticQuarticCrandallField {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for QuarticQuarticCrandallField {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self([
            self.0[0] - rhs.0[0],
            self.0[1] - rhs.0[1],
            self.0[2] - rhs.0[2],
            self.0[3] - rhs.0[3],
        ])
    }
}

impl SubAssign for QuarticQuarticCrandallField {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for QuarticQuarticCrandallField {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let Self([a0, a1, a2, a3]) = self;
        let Self([b0, b1, b2, b3]) = rhs;

        let c0 = a0 * b0 + <Self as OEF<4>>::W * (a1 * b3 + a2 * b2 + a3 * b1);
        let c1 = a0 * b1 + a1 * b0 + <Self as OEF<4>>::W * (a2 * b3 + a3 * b2);
        let c2 = a0 * b2 + a1 * b1 + a2 * b0 + <Self as OEF<4>>::W * a3 * b3;
        let c3 = a0 * b3 + a1 * b2 + a2 * b1 + a3 * b0;

        Self([c0, c1, c2, c3])
    }
}

impl MulAssign for QuarticQuarticCrandallField {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Product for QuarticQuarticCrandallField {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl Div for QuarticQuarticCrandallField {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for QuarticQuarticCrandallField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}
