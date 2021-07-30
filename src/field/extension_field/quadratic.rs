use std::fmt::{Debug, Display, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::bigint::BigUint;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::crandall_field::CrandallField;
use crate::field::extension_field::{FieldExtension, Frobenius, OEF};
use crate::field::field_types::Field;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct QuadraticCrandallField([CrandallField; 2]);

impl OEF<2> for QuadraticCrandallField {
    // Verifiable in Sage with
    // ``R.<x> = GF(p)[]; assert (x^2 -3).is_irreducible()`.
    const W: CrandallField = CrandallField(3);
}

impl Frobenius<2> for QuadraticCrandallField {}

impl FieldExtension<2> for QuadraticCrandallField {
    type BaseField = CrandallField;

    fn to_basefield_array(&self) -> [Self::BaseField; 2] {
        self.0
    }

    fn from_basefield_array(arr: [Self::BaseField; 2]) -> Self {
        Self(arr)
    }

    fn from_basefield(x: Self::BaseField) -> Self {
        x.into()
    }
}

impl From<<Self as FieldExtension<2>>::BaseField> for QuadraticCrandallField {
    fn from(x: <Self as FieldExtension<2>>::BaseField) -> Self {
        Self([x, <Self as FieldExtension<2>>::BaseField::ZERO])
    }
}

impl Field for QuadraticCrandallField {
    type PrimeField = CrandallField;

    const ZERO: Self = Self([CrandallField::ZERO; 2]);
    const ONE: Self = Self([CrandallField::ONE, CrandallField::ZERO]);
    const TWO: Self = Self([CrandallField::TWO, CrandallField::ZERO]);
    const NEG_ONE: Self = Self([CrandallField::NEG_ONE, CrandallField::ZERO]);

    const CHARACTERISTIC: u64 = CrandallField::CHARACTERISTIC;
    const TWO_ADICITY: usize = 29;
    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self([
        CrandallField(6483724566312148654),
        CrandallField(12194665049945415126),
    ]);
    // Chosen so that when raised to the power `1<<(Self::TWO_ADICITY-Self::BaseField::TWO_ADICITY)`,
    // we get `Self::BaseField::POWER_OF_TWO_GENERATOR`. This makes `primitive_root_of_unity` coherent
    // with the base field which implies that the FFT commutes with field inclusion.
    const POWER_OF_TWO_GENERATOR: Self =
        Self([CrandallField::ZERO, CrandallField(14420468973723774561)]);

    fn order() -> BigUint {
        CrandallField::order() * CrandallField::order()
    }

    // Algorithm 11.3.4 in Handbook of Elliptic and Hyperelliptic Curve Cryptography.
    fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        let a_pow_r_minus_1 = self.frobenius();
        let a_pow_r = a_pow_r_minus_1 * *self;
        debug_assert!(FieldExtension::<2>::is_in_basefield(&a_pow_r));

        Some(a_pow_r_minus_1 * a_pow_r.0[0].inverse().into())
    }

    fn to_canonical_u64(&self) -> u64 {
        self.0[0].to_canonical_u64()
    }

    fn from_canonical_u64(n: u64) -> Self {
        <Self as FieldExtension<2>>::BaseField::from_canonical_u64(n).into()
    }

    fn to_canonical_biguint(&self) -> BigUint {
        let first = self.0[0].to_canonical_biguint();
        let second = self.0[1].to_canonical_biguint();
        let combined = second * Self::CHARACTERISTIC + first;

        combined
    }

    fn from_canonical_biguint(n: BigUint) -> Self {
        let smaller = n.clone() % Self::CHARACTERISTIC;
        let larger = n.clone() / Self::CHARACTERISTIC;

        Self([
            <Self as FieldExtension<2>>::BaseField::from_canonical_biguint(smaller),
            <Self as FieldExtension<2>>::BaseField::from_canonical_biguint(larger),
        ])
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self([
            <Self as FieldExtension<2>>::BaseField::rand_from_rng(rng),
            <Self as FieldExtension<2>>::BaseField::rand_from_rng(rng),
        ])
    }
}

impl Display for QuadraticCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} + {}*a", self.0[0], self.0[1])
    }
}

impl Debug for QuadraticCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Neg for QuadraticCrandallField {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1]])
    }
}

impl Add for QuadraticCrandallField {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self([self.0[0] + rhs.0[0], self.0[1] + rhs.0[1]])
    }
}

impl AddAssign for QuadraticCrandallField {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for QuadraticCrandallField {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for QuadraticCrandallField {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
    }
}

impl SubAssign for QuadraticCrandallField {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for QuadraticCrandallField {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let Self([a0, a1]) = self;
        let Self([b0, b1]) = rhs;

        let c0 = a0 * b0 + <Self as OEF<2>>::W * a1 * b1;
        let c1 = a0 * b1 + a1 * b0;

        Self([c0, c1])
    }
}

impl MulAssign for QuadraticCrandallField {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Product for QuadraticCrandallField {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl Div for QuadraticCrandallField {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for QuadraticCrandallField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

#[cfg(test)]
mod tests {
    use crate::field::extension_field::quadratic::QuadraticCrandallField;
    use crate::field::extension_field::{FieldExtension, Frobenius};
    use crate::field::field_types::Field;
    use crate::test_field_arithmetic;

    #[test]
    fn test_add_neg_sub_mul() {
        type F = QuadraticCrandallField;
        let x = F::rand();
        let y = F::rand();
        let z = F::rand();
        assert_eq!(x + (-x), F::ZERO);
        assert_eq!(-x, F::ZERO - x);
        assert_eq!(x + x, x * <F as FieldExtension<2>>::BaseField::TWO.into());
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
        type F = QuadraticCrandallField;
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
        type F = QuadraticCrandallField;
        let x = F::rand();
        assert_eq!(
            x.exp_biguint(&<F as FieldExtension<2>>::BaseField::order()),
            x.frobenius()
        );
    }

    #[test]
    fn test_field_order() {
        // F::order() = 340282366831806780677557380898690695169 = 18446744071293632512 *18446744071293632514 + 1
        type F = QuadraticCrandallField;
        let x = F::rand();
        assert_eq!(
            x.exp(18446744071293632512).exp(18446744071293632514),
            F::ONE
        );
    }

    #[test]
    fn test_power_of_two_gen() {
        type F = QuadraticCrandallField;
        // F::order() = 2^29 * 2762315674048163 * 229454332791453 + 1
        assert_eq!(
            F::MULTIPLICATIVE_GROUP_GENERATOR
                .exp(2762315674048163)
                .exp(229454332791453),
            F::POWER_OF_TWO_GENERATOR
        );
        assert_eq!(
            F::POWER_OF_TWO_GENERATOR
                .exp(1 << (F::TWO_ADICITY - <F as FieldExtension<2>>::BaseField::TWO_ADICITY)),
            <F as FieldExtension<2>>::BaseField::POWER_OF_TWO_GENERATOR.into()
        );
    }

    test_field_arithmetic!(crate::field::extension_field::quadratic::QuadraticCrandallField);
}
