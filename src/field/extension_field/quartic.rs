use std::fmt::{Debug, Display, Formatter};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::bigint::BigUint;
use num::traits::Pow;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::{Extendable, FieldExtension, Frobenius, OEF};
use crate::field::field_types::Field;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QuarticExtension<F: Extendable<4>>(pub(crate) [F; 4]);

impl<F: Extendable<4>> Default for QuarticExtension<F> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<F: Extendable<4>> OEF<4> for QuarticExtension<F> {
    const W: F = F::W;
}

impl<F: Extendable<4>> Frobenius<4> for QuarticExtension<F> {}

impl<F: Extendable<4>> FieldExtension<4> for QuarticExtension<F> {
    type BaseField = F;

    fn to_basefield_array(&self) -> [F; 4] {
        self.0
    }

    fn from_basefield_array(arr: [F; 4]) -> Self {
        Self(arr)
    }

    fn from_basefield(x: F) -> Self {
        x.into()
    }
}

impl<F: Extendable<4>> From<F> for QuarticExtension<F> {
    fn from(x: F) -> Self {
        Self([x, F::ZERO, F::ZERO, F::ZERO])
    }
}

impl<F: Extendable<4>> Field for QuarticExtension<F> {
    type PrimeField = F;

    const ZERO: Self = Self([F::ZERO; 4]);
    const ONE: Self = Self([F::ONE, F::ZERO, F::ZERO, F::ZERO]);
    const TWO: Self = Self([F::TWO, F::ZERO, F::ZERO, F::ZERO]);
    const NEG_ONE: Self = Self([F::NEG_ONE, F::ZERO, F::ZERO, F::ZERO]);

    const CHARACTERISTIC: u64 = F::ORDER;

    // `p^4 - 1 = (p - 1)(p + 1)(p^2 + 1)`. The `p - 1` term has a two-adicity of `F::TWO_ADICITY`.
    // As long as `F::TWO_ADICITY >= 2`, `p` can be written as `4n + 1`, so `p + 1` can be written as
    // `2(2n + 1)`, which has a 2-adicity of 1. A similar argument can show that `p^2 + 1` also has
    // a 2-adicity of 1.
    const TWO_ADICITY: usize = F::TWO_ADICITY + 2;

    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self(F::EXT_MULTIPLICATIVE_GROUP_GENERATOR);
    const POWER_OF_TWO_GENERATOR: Self = Self(F::EXT_POWER_OF_TWO_GENERATOR);

    fn order() -> BigUint {
        F::order().pow(4u32)
    }

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

        Some(FieldExtension::<4>::scalar_mul(
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
        Self::from_basefield_array([
            F::rand_from_rng(rng),
            F::rand_from_rng(rng),
            F::rand_from_rng(rng),
            F::rand_from_rng(rng),
        ])
    }
}

impl<F: Extendable<4>> Display for QuarticExtension<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} + {}*a + {}*a^2 + {}*a^3",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

impl<F: Extendable<4>> Debug for QuarticExtension<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<F: Extendable<4>> Neg for QuarticExtension<F> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1], -self.0[2], -self.0[3]])
    }
}

impl<F: Extendable<4>> Add for QuarticExtension<F> {
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

impl<F: Extendable<4>> AddAssign for QuarticExtension<F> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<F: Extendable<4>> Sum for QuarticExtension<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl<F: Extendable<4>> Sub for QuarticExtension<F> {
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

impl<F: Extendable<4>> SubAssign for QuarticExtension<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<F: Extendable<4>> Mul for QuarticExtension<F> {
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

impl<F: Extendable<4>> MulAssign for QuarticExtension<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<F: Extendable<4>> Product for QuarticExtension<F> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl<F: Extendable<4>> Div for QuarticExtension<F> {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl<F: Extendable<4>> DivAssign for QuarticExtension<F> {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticExtension;
    use crate::field::extension_field::Frobenius;
    use crate::field::field_types::Field;
    use crate::test_field_arithmetic;

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
        type F = QuarticExtension<CrandallField>;
        let x = F::rand();
        let y = F::rand();
        let z = F::rand();
        assert_eq!(x + (-x), F::ZERO);
        assert_eq!(-x, F::ZERO - x);
        assert_eq!(x + x, x * F::TWO.into());
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
        type F = QuarticExtension<CrandallField>;
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
        type F = QuarticExtension<CrandallField>;
        const D: usize = 4;
        let x = F::rand();
        assert_eq!(x.exp_biguint(&CrandallField::order()), x.frobenius());
        for count in 2..D {
            assert_eq!(
                x.repeated_frobenius(count),
                (0..count).fold(x, |acc, _| acc.frobenius())
            );
        }
    }

    #[test]
    fn test_field_order() {
        // F::order() = 340282366831806780677557380898690695168 * 340282366831806780677557380898690695170 + 1
        type F = QuarticExtension<CrandallField>;
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
        type F = QuarticExtension<CrandallField>;
        // F::order() = 2^30 * 1090552343587053358839971118999869 * 98885475095492590491252558464653635 + 1
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
            F::POWER_OF_TWO_GENERATOR.exp_u64(1 << (F::TWO_ADICITY - CrandallField::TWO_ADICITY)),
            CrandallField::POWER_OF_TWO_GENERATOR.into()
        );
    }

    test_field_arithmetic!(
        crate::field::extension_field::quartic::QuarticExtension<
            crate::field::crandall_field::CrandallField,
        >
    );
}
