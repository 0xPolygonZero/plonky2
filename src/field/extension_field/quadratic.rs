use std::fmt::{Debug, Display, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::bigint::BigUint;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::{Extendable, FieldExtension, Frobenius, OEF};
use crate::field::field_types::Field;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QuadraticExtension<F: Extendable<2>>(pub(crate) [F; 2]);

impl<F: Extendable<2>> Default for QuadraticExtension<F> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<F: Extendable<2>> OEF<2> for QuadraticExtension<F> {
    const W: F = F::W;
}

impl<F: Extendable<2>> Frobenius<2> for QuadraticExtension<F> {}

impl<F: Extendable<2>> FieldExtension<2> for QuadraticExtension<F> {
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

impl<F: Extendable<2>> From<F> for QuadraticExtension<F> {
    fn from(x: F) -> Self {
        Self([x, F::ZERO])
    }
}

impl<F: Extendable<2>> Field for QuadraticExtension<F> {
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

impl<F: Extendable<2>> Display for QuadraticExtension<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} + {}*a", self.0[0], self.0[1])
    }
}

impl<F: Extendable<2>> Debug for QuadraticExtension<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<F: Extendable<2>> Neg for QuadraticExtension<F> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1]])
    }
}

impl<F: Extendable<2>> Add for QuadraticExtension<F> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self([self.0[0] + rhs.0[0], self.0[1] + rhs.0[1]])
    }
}

impl<F: Extendable<2>> AddAssign for QuadraticExtension<F> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<F: Extendable<2>> Sum for QuadraticExtension<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl<F: Extendable<2>> Sub for QuadraticExtension<F> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
    }
}

impl<F: Extendable<2>> SubAssign for QuadraticExtension<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<F: Extendable<2>> Mul for QuadraticExtension<F> {
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

impl<F: Extendable<2>> MulAssign for QuadraticExtension<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<F: Extendable<2>> Product for QuadraticExtension<F> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl<F: Extendable<2>> Div for QuadraticExtension<F> {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl<F: Extendable<2>> DivAssign for QuadraticExtension<F> {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

#[cfg(test)]
mod tests {
    use num::{BigUint, One};

    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quadratic::QuadraticExtension;
    use crate::field::extension_field::{Extendable, Frobenius};
    use crate::field::field_types::Field;
    use crate::test_field_arithmetic;

    fn test_add_neg_sub_mul<BF: Extendable<2>>() {
        let x = BF::Extension::rand();
        let y = BF::Extension::rand();
        let z = BF::Extension::rand();
        assert_eq!(x + (-x), BF::Extension::ZERO);
        assert_eq!(-x, BF::Extension::ZERO - x);
        assert_eq!(x + x, x * BF::Extension::TWO);
        assert_eq!(x * (-x), -x.square());
        assert_eq!(x + y, y + x);
        assert_eq!(x * y, y * x);
        assert_eq!(x * (y * z), (x * y) * z);
        assert_eq!(x - (y + z), (x - y) - z);
        assert_eq!((x + y) - z, x + (y - z));
        assert_eq!(x * (y + z), x * y + x * z);
    }

    fn test_inv_div<BF: Extendable<2>>() {
        let x = BF::Extension::rand();
        let y = BF::Extension::rand();
        let z = BF::Extension::rand();
        assert_eq!(x * x.inverse(), BF::Extension::ONE);
        assert_eq!(x.inverse() * x, BF::Extension::ONE);
        assert_eq!(x.square().inverse(), x.inverse().square());
        assert_eq!((x / y) * y, x);
        assert_eq!(x / (y * z), (x / y) / z);
        assert_eq!((x * y) / z, x * (y / z));
    }

    fn test_frobenius<BF: Extendable<2>>() {
        let x = BF::Extension::rand();
        assert_eq!(x.exp_biguint(&BF::order()), x.frobenius());
    }

    fn test_field_order<BF: Extendable<2>>() {
        let x = BF::Extension::rand();
        assert_eq!(
            x.exp_biguint(&(BF::Extension::order() - 1u8)),
            BF::Extension::ONE
        );
    }

    fn test_power_of_two_gen<BF: Extendable<2>>() {
        assert_eq!(
            BF::Extension::MULTIPLICATIVE_GROUP_GENERATOR
                .exp_biguint(&(BF::Extension::order() >> BF::Extension::TWO_ADICITY)),
            BF::Extension::POWER_OF_TWO_GENERATOR.into()
        );
        assert_eq!(
            BF::Extension::POWER_OF_TWO_GENERATOR
                .exp_u64(1 << (BF::Extension::TWO_ADICITY - BF::TWO_ADICITY)),
            BF::POWER_OF_TWO_GENERATOR.into()
        );
    }

    macro_rules! test_quadratic_extension {
        ($field:ty) => {
            #[test]
            fn test_add_neg_sub_mul() {
                super::test_add_neg_sub_mul::<$field>();
            }
            #[test]
            fn test_inv_div() {
                super::test_inv_div::<$field>();
            }
            #[test]
            fn test_frobenius() {
                super::test_frobenius::<$field>();
            }
            #[test]
            fn test_field_order() {
                super::test_field_order::<$field>();
            }
            #[test]
            fn test_power_of_two_gen() {
                super::test_power_of_two_gen::<$field>();
            }
        };
    }
    mod crandall {
        use crate::field::crandall_field::CrandallField;
        use crate::test_field_arithmetic;

        test_quadratic_extension!(CrandallField);
        test_field_arithmetic!(
            crate::field::extension_field::quadratic::QuadraticExtension<
                crate::field::crandall_field::CrandallField,
            >
        );
    }

    mod goldilocks {
        use crate::field::goldilocks_field::GoldilocksField;
        use crate::test_field_arithmetic;

        test_quadratic_extension!(GoldilocksField);
        test_field_arithmetic!(
            crate::field::extension_field::quadratic::QuadraticExtension<
                crate::field::goldilocks_field::GoldilocksField,
            >
        );
    }
}
