use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use rand::Rng;

use crate::field::crandall_field::CrandallField;
use crate::field::extension_field::{FieldExtension, Frobenius, OEF};
use crate::field::field::Field;

/// A quartic extension of `CrandallField`.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct QuarticCrandallField(pub(crate) [CrandallField; 4]);

impl OEF<4> for QuarticCrandallField {
    // Verifiable in Sage with
    //     R.<x> = GF(p)[]
    //     assert (x^4 - 3).is_irreducible()
    const W: CrandallField = CrandallField(3);
}

impl Frobenius<CrandallField, 4> for QuarticCrandallField {}

impl FieldExtension<4> for QuarticCrandallField {
    type BaseField = CrandallField;

    fn to_basefield_array(&self) -> [Self::BaseField; 4] {
        self.0
    }

    fn from_basefield_array(arr: [Self::BaseField; 4]) -> Self {
        Self(arr)
    }

    fn from_basefield(x: Self::BaseField) -> Self {
        x.into()
    }
}

impl From<<Self as FieldExtension<4>>::BaseField> for QuarticCrandallField {
    fn from(x: <Self as FieldExtension<4>>::BaseField) -> Self {
        Self([
            x,
            <Self as FieldExtension<4>>::BaseField::ZERO,
            <Self as FieldExtension<4>>::BaseField::ZERO,
            <Self as FieldExtension<4>>::BaseField::ZERO,
        ])
    }
}

impl Field for QuarticCrandallField {
    const ZERO: Self = Self([CrandallField::ZERO; 4]);
    const ONE: Self = Self([
        CrandallField::ONE,
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::ZERO,
    ]);
    const TWO: Self = Self([
        CrandallField::TWO,
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::ZERO,
    ]);
    const NEG_ONE: Self = Self([
        CrandallField::NEG_ONE,
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::ZERO,
    ]);

    // Does not fit in 64-bits.
    const ORDER: u64 = 0;
    const TWO_ADICITY: usize = 30;
    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self([
        CrandallField(12476589904174392631),
        CrandallField(896937834427772243),
        CrandallField(7795248119019507390),
        CrandallField(9005769437373554825),
    ]);
    // Chosen so that when raised to the power `1<<(Self::TWO_ADICITY-Self::BaseField::TWO_ADICITY)`,
    // we get `Self::BaseField::POWER_OF_TWO_GENERATOR`. This makes `primitive_root_of_unity` coherent
    // with the base field which implies that the FFT commutes with field inclusion.
    const POWER_OF_TWO_GENERATOR: Self = Self([
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField(15170983443234254033),
    ]);

    // Algorithm 11.3.4 in Handbook of Elliptic and Hyperelliptic Curve Cryptography.
    fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        let a_pow_p = self.frobenius();
        let a_pow_p_plus_1 = a_pow_p * *self;
        let a_pow_p3_plus_p2 = a_pow_p_plus_1.repeated_frobenius(2);
        let a_pow_r_minus_1 = a_pow_p3_plus_p2 * a_pow_p;
        let a_pow_r = a_pow_r_minus_1 * *self;
        debug_assert!(FieldExtension::<4>::is_in_basefield(&a_pow_r));

        Some(a_pow_r_minus_1 * a_pow_r.0[0].inverse().into())
    }

    fn to_canonical_u64(&self) -> u64 {
        self.0[0].to_canonical_u64()
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

impl Display for QuarticCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} + {}*a + {}*a^2 + {}*a^3",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

impl Debug for QuarticCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Neg for QuarticCrandallField {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1], -self.0[2], -self.0[3]])
    }
}

impl Add for QuarticCrandallField {
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

impl AddAssign for QuarticCrandallField {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for QuarticCrandallField {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for QuarticCrandallField {
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

impl SubAssign for QuarticCrandallField {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for QuarticCrandallField {
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

impl MulAssign for QuarticCrandallField {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Product for QuarticCrandallField {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl Div for QuarticCrandallField {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for QuarticCrandallField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

#[cfg(test)]
mod tests {
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::extension_field::{FieldExtension, Frobenius, OEF};
    use crate::field::field::Field;

    fn exp_naive<F: Field>(x: F, power: u128) -> F {
        let mut current = x;
        let mut product = F::ONE;

        for j in 0..128 {
            if (power >> j & 1) != 0 {
                product *= current;
            }
            current = current.square();
        }
        product
    }

    #[test]
    fn test_add_neg_sub_mul() {
        type F = QuarticCrandallField;
        let x = F::rand();
        let y = F::rand();
        let z = F::rand();
        assert_eq!(x + (-x), F::ZERO);
        assert_eq!(-x, F::ZERO - x);
        assert_eq!(x + x, x * <F as FieldExtension<4>>::BaseField::TWO.into());
        assert_eq!(x * (-x), -x.square());
        assert_eq!(x + y, y + x);
        assert_eq!(x * y, y * x);
        assert_eq!(x * (y * z), (x * y) * z);
        assert_eq!(x - (y + z), (x - y) - z);
        assert_eq!((x + y) - z, x + (y - z));
        assert_eq!(x * (y + z), x * y + x * z);
    }

    #[test]
    fn test_inv_div() {
        type F = QuarticCrandallField;
        let x = F::rand();
        let y = F::rand();
        let z = F::rand();
        assert_eq!(x * x.inverse(), F::ONE);
        assert_eq!(x.inverse() * x, F::ONE);
        assert_eq!(x.square().inverse(), x.inverse().square());
        assert_eq!((x / y) * y, x);
        assert_eq!(x / (y * z), (x / y) / z);
        assert_eq!((x * y) / z, x * (y / z));
    }

    #[test]
    fn test_frobenius() {
        type F = QuarticCrandallField;
        let x = F::rand();
        assert_eq!(
            exp_naive(x, <F as FieldExtension<4>>::BaseField::ORDER as u128),
            x.frobenius()
        );
        assert_eq!(x.repeated_frobenius(2), x.frobenius().frobenius());
        assert_eq!(
            x.repeated_frobenius(3),
            x.frobenius().frobenius().frobenius()
        );
    }

    #[test]
    fn test_field_order() {
        // F::ORDER = 340282366831806780677557380898690695168 * 340282366831806780677557380898690695170 + 1
        type F = QuarticCrandallField;
        let x = F::rand();
        assert_eq!(
            exp_naive(
                exp_naive(x, 340282366831806780677557380898690695168),
                340282366831806780677557380898690695170
            ),
            F::ONE
        );
    }

    #[test]
    fn test_power_of_two_gen() {
        type F = QuarticCrandallField;
        // F::ORDER = 2^30 * 1090552343587053358839971118999869 * 98885475095492590491252558464653635 + 1
        assert_eq!(
            exp_naive(
                exp_naive(
                    F::MULTIPLICATIVE_GROUP_GENERATOR,
                    1090552343587053358839971118999869
                ),
                98885475095492590491252558464653635
            ),
            F::POWER_OF_TWO_GENERATOR
        );
        assert_eq!(
            F::POWER_OF_TWO_GENERATOR
                .exp(1 << (F::TWO_ADICITY - <F as FieldExtension<4>>::BaseField::TWO_ADICITY)),
            <F as FieldExtension<4>>::BaseField::POWER_OF_TWO_GENERATOR.into()
        );
    }
}
