use crate::field::crandall_field::CrandallField;
use crate::field::field::Field;
use rand::Rng;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

pub trait BinaryFieldExtension: Field {
    type BaseField: Field;

    // Element W of BaseField, such that `X^2 - W` is irreducible over BaseField.
    const W: Self::BaseField;

    fn to_canonical_representation(&self) -> [Self::BaseField; 2];

    fn is_in_basefield(&self) -> bool {
        self.to_canonical_representation()[1..]
            .iter()
            .all(|x| x.is_zero())
    }

    /// Frobenius automorphisms: x -> x^p, where p is the order of BaseField.
    fn frobenius(&self) -> Self;

    fn scalar_mul(&self, c: Self::BaseField) -> Self;
}

#[derive(Copy, Clone)]
pub struct BinaryCrandallField([CrandallField; 2]);

impl BinaryFieldExtension for BinaryCrandallField {
    type BaseField = CrandallField;
    // Verifiable in Sage with
    // ``R.<x> = GF(p)[]; assert (x^2 -3).is_irreducible()`.
    const W: Self::BaseField = CrandallField(3);

    fn to_canonical_representation(&self) -> [Self::BaseField; 2] {
        self.0
    }

    fn frobenius(&self) -> Self {
        let [a0, a1] = self.to_canonical_representation();
        let k = (Self::BaseField::ORDER - 1) / 2;
        let z = Self::W.exp_usize(k as usize);

        Self([a0, a1 * z])
    }

    fn scalar_mul(&self, c: Self::BaseField) -> Self {
        let [a0, a1] = self.to_canonical_representation();
        Self([a0 * c, a1 * c])
    }
}

impl PartialEq for BinaryCrandallField {
    fn eq(&self, other: &Self) -> bool {
        self.to_canonical_representation() == other.to_canonical_representation()
    }
}

impl Eq for BinaryCrandallField {}

impl Hash for BinaryCrandallField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for l in &self.to_canonical_representation() {
            Hash::hash(l, state);
        }
    }
}

impl Field for BinaryCrandallField {
    const ZERO: Self = Self([CrandallField::ZERO; 2]);
    const ONE: Self = Self([CrandallField::ONE, CrandallField::ZERO]);
    const TWO: Self = Self([CrandallField::TWO, CrandallField::ZERO]);
    const NEG_ONE: Self = Self([CrandallField::NEG_ONE, CrandallField::ZERO]);

    // Does not fit in 64-bits.
    const ORDER: u64 = 0;
    const TWO_ADICITY: usize = 29;
    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self([CrandallField(3), CrandallField::ONE]);
    const POWER_OF_TWO_GENERATOR: Self =
        Self([CrandallField::ZERO, CrandallField(7889429148549342301)]);

    // Algorithm 11.3.4 in Handbook of Elliptic and Hyperelliptic Curve Cryptography.
    fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        let a_pow_r_minus_1 = self.frobenius();
        let a_pow_r = a_pow_r_minus_1 * *self;
        debug_assert!(a_pow_r.is_in_basefield());

        Some(a_pow_r_minus_1.scalar_mul(a_pow_r.0[0].inverse()))
    }

    fn to_canonical_u64(&self) -> u64 {
        self.0[0].to_canonical_u64()
    }

    fn from_canonical_u64(n: u64) -> Self {
        Self([
            <Self as BinaryFieldExtension>::BaseField::from_canonical_u64(n),
            <Self as BinaryFieldExtension>::BaseField::ZERO,
        ])
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self([
            <Self as BinaryFieldExtension>::BaseField::rand_from_rng(rng),
            <Self as BinaryFieldExtension>::BaseField::rand_from_rng(rng),
        ])
    }
}

impl Display for BinaryCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} + {}*a", self.0[0], self.0[1])
    }
}

impl Debug for BinaryCrandallField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Neg for BinaryCrandallField {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1]])
    }
}

impl Add for BinaryCrandallField {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self([self.0[0] + rhs.0[0], self.0[1] + rhs.0[1]])
    }
}

impl AddAssign for BinaryCrandallField {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for BinaryCrandallField {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for BinaryCrandallField {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
    }
}

impl SubAssign for BinaryCrandallField {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for BinaryCrandallField {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let Self([a0, a1]) = self;
        let Self([b0, b1]) = rhs;

        let c0 = a0 * b0 + Self::W * a1 * b1;
        let c1 = a0 * b1 + a1 * b0;

        Self([c0, c1])
    }
}

impl MulAssign for BinaryCrandallField {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Product for BinaryCrandallField {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl Div for BinaryCrandallField {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for BinaryCrandallField {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::binary::{BinaryCrandallField, BinaryFieldExtension};
    use crate::field::field::Field;
    use crate::test_arithmetic;

    fn exp_naive<F: Field>(x: F, power: u64) -> F {
        let mut current = x;
        let mut product = F::ONE;

        for j in 0..64 {
            if (power >> j & 1) != 0 {
                product *= current;
            }
            current = current.square();
        }
        product
    }

    #[test]
    fn test_add_neg_sub_mul() {
        type F = BinaryCrandallField;
        let x = F::rand();
        let y = F::rand();
        let z = F::rand();
        assert_eq!(x + (-x), F::ZERO);
        assert_eq!(-x, F::ZERO - x);
        assert_eq!(
            x + x,
            x.scalar_mul(<F as BinaryFieldExtension>::BaseField::TWO)
        );
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
        type F = BinaryCrandallField;
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
        type F = BinaryCrandallField;
        let x = F::rand();
        assert_eq!(
            exp_naive(x, <F as BinaryFieldExtension>::BaseField::ORDER),
            x.frobenius()
        );
    }

    #[test]
    fn test_field_order() {
        // F::ORDER = 340282366831806780677557380898690695169 = 18446744071293632512 *18446744071293632514 + 1
        type F = BinaryCrandallField;
        let x = F::rand();
        assert_eq!(
            exp_naive(exp_naive(x, 18446744071293632512), 18446744071293632514),
            F::ONE
        );
    }
}
