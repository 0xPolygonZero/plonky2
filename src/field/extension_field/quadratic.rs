use std::fmt::{Debug, Display, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::bigint::BigUint;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::{AutoExtendable, Extendable, FieldExtension, Frobenius, OEF};
use crate::field::field_types::Field;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QuadraticExtension<F: AutoExtendable<2>>(pub(crate) [F; 2]);

impl<F: AutoExtendable<2>> Extendable<2> for F {
    type Extension = QuadraticExtension<Self>;
}

impl<F: AutoExtendable<2>> Default for QuadraticExtension<F> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<F: AutoExtendable<2>> OEF<2> for QuadraticExtension<F> {
    const W: F = F::W;
}

impl<F: AutoExtendable<2>> Frobenius<2> for QuadraticExtension<F> {}

impl<F: AutoExtendable<2>> FieldExtension<2> for QuadraticExtension<F> {
    type BaseField = F;

    fn to_basefield_array(&self) -> [F; 2] {
        self.0
    }

    fn from_basefield_array(arr: [F; 2]) -> Self {
        Self(arr)
    }

    fn from_basefield(x: F) -> Self {
        x.into()
    }
}

impl<F: AutoExtendable<2>> From<F> for QuadraticExtension<F> {
    fn from(x: F) -> Self {
        Self([x, F::ZERO])
    }
}

impl<F: AutoExtendable<2>> Field for QuadraticExtension<F> {
    type PrimeField = F;

    const ZERO: Self = Self([F::ZERO; 2]);
    const ONE: Self = Self([F::ONE, F::ZERO]);
    const TWO: Self = Self([F::TWO, F::ZERO]);
    const NEG_ONE: Self = Self([F::NEG_ONE, F::ZERO]);

    const CHARACTERISTIC: u64 = F::CHARACTERISTIC;

    // `p^2 - 1 = (p - 1)(p + 1)`. The `p - 1` term has a two-adicity of `F::TWO_ADICITY`. As
    // long as `F::TWO_ADICITY >= 2`, `p` can be written as `4n + 1`, so `p + 1` can be written as
    // `2(2n + 1)`, which has a 2-adicity of 1.
    const TWO_ADICITY: usize = F::TWO_ADICITY + 1;

    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self(F::EXT_MULTIPLICATIVE_GROUP_GENERATOR);
    const POWER_OF_TWO_GENERATOR: Self = Self(F::EXT_POWER_OF_TWO_GENERATOR);

    fn order() -> BigUint {
        F::order() * F::order()
    }

    // Algorithm 11.3.4 in Handbook of Elliptic and Hyperelliptic Curve Cryptography.
    fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        let a_pow_r_minus_1 = self.frobenius();
        let a_pow_r = a_pow_r_minus_1 * *self;
        debug_assert!(FieldExtension::<2>::is_in_basefield(&a_pow_r));

        Some(FieldExtension::<2>::scalar_mul(
            &a_pow_r_minus_1,
            a_pow_r.0[0].inverse(),
        ))
    }

    fn from_canonical_u64(n: u64) -> Self {
        F::from_canonical_u64(n).into()
    }

    fn from_noncanonical_u128(n: u128) -> Self {
        F::from_noncanonical_u128(n).into()
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self([F::rand_from_rng(rng), F::rand_from_rng(rng)])
    }
}

impl<F: AutoExtendable<2>> Display for QuadraticExtension<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} + {}*a", self.0[0], self.0[1])
    }
}

impl<F: AutoExtendable<2>> Debug for QuadraticExtension<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<F: AutoExtendable<2>> Neg for QuadraticExtension<F> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1]])
    }
}

impl<F: AutoExtendable<2>> Add for QuadraticExtension<F> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self([self.0[0] + rhs.0[0], self.0[1] + rhs.0[1]])
    }
}

impl<F: AutoExtendable<2>> AddAssign for QuadraticExtension<F> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<F: AutoExtendable<2>> Sum for QuadraticExtension<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl<F: AutoExtendable<2>> Sub for QuadraticExtension<F> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
    }
}

impl<F: AutoExtendable<2>> SubAssign for QuadraticExtension<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<F: AutoExtendable<2>> Mul for QuadraticExtension<F> {
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

impl<F: AutoExtendable<2>> MulAssign for QuadraticExtension<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<F: AutoExtendable<2>> Product for QuadraticExtension<F> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl<F: AutoExtendable<2>> Div for QuadraticExtension<F> {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl<F: AutoExtendable<2>> DivAssign for QuadraticExtension<F> {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quadratic::QuadraticExtension;
    use crate::field::extension_field::{FieldExtension, Frobenius};
    use crate::field::field_types::Field;
    use crate::test_field_arithmetic;

    #[test]
    fn test_add_neg_sub_mul() {
        type F = QuadraticExtension<CrandallField>;
        let x = F::rand();
        let y = F::rand();
        let z = F::rand();
        assert_eq!(x + (-x), F::ZERO);
        assert_eq!(-x, F::ZERO - x);
        assert_eq!(x + x, x * F::TWO);
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
        type F = QuadraticExtension<CrandallField>;
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
        type F = QuadraticExtension<CrandallField>;
        let x = F::rand();
        assert_eq!(x.exp_biguint(&CrandallField::order()), x.frobenius());
    }

    #[test]
    fn test_field_order() {
        // F::order() = 340282366831806780677557380898690695169 = 18446744071293632512 *18446744071293632514 + 1
        type F = QuadraticExtension<CrandallField>;
        let x = F::rand();
        assert_eq!(
            x.exp_u64(18446744071293632512)
                .exp_u64(18446744071293632514),
            F::ONE
        );
    }

    #[test]
    fn test_power_of_two_gen() {
        type F = QuadraticExtension<CrandallField>;
        // F::order() = 2^29 * 2762315674048163 * 229454332791453 + 1
        assert_eq!(
            F::MULTIPLICATIVE_GROUP_GENERATOR
                .exp_u64(2762315674048163)
                .exp_u64(229454332791453),
            F::POWER_OF_TWO_GENERATOR
        );
        assert_eq!(
            F::POWER_OF_TWO_GENERATOR.exp_u64(1 << (F::TWO_ADICITY - CrandallField::TWO_ADICITY)),
            CrandallField::POWER_OF_TWO_GENERATOR.into()
        );
    }

    test_field_arithmetic!(
        crate::field::extension_field::quadratic::QuadraticExtension<
            crate::field::crandall_field::CrandallField,
        >
    );
}
