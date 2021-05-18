use crate::field::crandall_field::CrandallField;
use crate::field::field::Field;
use rand::Rng;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

pub trait QuarticFieldExtension: Field + From<<Self as QuarticFieldExtension>::BaseField> {
    type BaseField: Field;

    // Element W of BaseField, such that `X^4 - W` is irreducible over BaseField.
    const W: Self::BaseField;

    fn to_canonical_representation(&self) -> [Self::BaseField; 4];

    fn from_canonical_representation(v: [Self::BaseField; 4]) -> Self;

    fn is_in_basefield(&self) -> bool {
        self.to_canonical_representation()[1..]
            .iter()
            .all(|x| x.is_zero())
    }

    /// Frobenius automorphisms: x -> x^p, where p is the order of BaseField.
    fn frobenius(&self) -> Self {
        let [a0, a1, a2, a3] = self.to_canonical_representation();
        let k = (Self::BaseField::ORDER - 1) / 4;
        let z0 = Self::W.exp_usize(k as usize);
        let mut z = Self::BaseField::ONE;
        let b0 = a0 * z;
        z *= z0;
        let b1 = a1 * z;
        z *= z0;
        let b2 = a2 * z;
        z *= z0;
        let b3 = a3 * z;

        Self::from_canonical_representation([b0, b1, b2, b3])
    }
}

#[derive(Copy, Clone)]
pub struct QuarticCrandallField([CrandallField; 4]);

impl QuarticFieldExtension for QuarticCrandallField {
    type BaseField = CrandallField;
    // Verifiable in Sage with
    // ``R.<x> = GF(p)[]; assert (x^4 -3).is_irreducible()`.
    const W: Self::BaseField = CrandallField(3);

    fn to_canonical_representation(&self) -> [Self::BaseField; 4] {
        self.0
    }

    fn from_canonical_representation(v: [Self::BaseField; 4]) -> Self {
        Self(v)
    }
}

impl From<<Self as QuarticFieldExtension>::BaseField> for QuarticCrandallField {
    fn from(x: <Self as QuarticFieldExtension>::BaseField) -> Self {
        Self([
            x,
            <Self as QuarticFieldExtension>::BaseField::ZERO,
            <Self as QuarticFieldExtension>::BaseField::ZERO,
            <Self as QuarticFieldExtension>::BaseField::ZERO,
        ])
    }
}

impl PartialEq for QuarticCrandallField {
    fn eq(&self, other: &Self) -> bool {
        self.to_canonical_representation() == other.to_canonical_representation()
    }
}

impl Eq for QuarticCrandallField {}

impl Hash for QuarticCrandallField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for l in &self.to_canonical_representation() {
            Hash::hash(l, state);
        }
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
        CrandallField(3),
        CrandallField::ONE,
        CrandallField::ZERO,
        CrandallField::ZERO,
    ]);
    const POWER_OF_TWO_GENERATOR: Self = Self([
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField::ZERO,
        CrandallField(14096607364803438105),
    ]);

    // Algorithm 11.3.4 in Handbook of Elliptic and Hyperelliptic Curve Cryptography.
    fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        let a_pow_p = self.frobenius();
        let a_pow_p_plus_1 = a_pow_p * *self;
        let a_pow_p3_plus_p2 = a_pow_p_plus_1.frobenius().frobenius();
        let a_pow_r_minus_1 = a_pow_p3_plus_p2 * a_pow_p;
        let a_pow_r = a_pow_r_minus_1 * *self;
        debug_assert!(a_pow_r.is_in_basefield());

        Some(a_pow_r_minus_1 * a_pow_r.0[0].inverse().into())
    }

    // It's important that the primitive roots of unity are the same as the ones in the base field,
    // otherwise the FFT doesn't commute with field inclusion.
    fn primitive_root_of_unity(n_log: usize) -> Self {
        if n_log <= CrandallField::TWO_ADICITY {
            CrandallField::primitive_root_of_unity(n_log).into()
        } else {
            // The root of unity isn't in the base field so we need to compute it manually.
            assert!(n_log <= Self::TWO_ADICITY);
            let mut base = Self::POWER_OF_TWO_GENERATOR;
            for _ in n_log..Self::TWO_ADICITY {
                base = base.square();
            }
            base
        }
    }

    fn to_canonical_u64(&self) -> u64 {
        self.0[0].to_canonical_u64()
    }

    fn from_canonical_u64(n: u64) -> Self {
        Self([
            <Self as QuarticFieldExtension>::BaseField::from_canonical_u64(n),
            <Self as QuarticFieldExtension>::BaseField::ZERO,
            <Self as QuarticFieldExtension>::BaseField::ZERO,
            <Self as QuarticFieldExtension>::BaseField::ZERO,
        ])
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self([
            <Self as QuarticFieldExtension>::BaseField::rand_from_rng(rng),
            <Self as QuarticFieldExtension>::BaseField::rand_from_rng(rng),
            <Self as QuarticFieldExtension>::BaseField::rand_from_rng(rng),
            <Self as QuarticFieldExtension>::BaseField::rand_from_rng(rng),
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

        let c0 = a0 * b0 + Self::W * (a1 * b3 + a2 * b2 + a3 * b1);
        let c1 = a0 * b1 + a1 * b0 + Self::W * (a2 * b3 + a3 * b2);
        let c2 = a0 * b2 + a1 * b1 + a2 * b0 + Self::W * a3 * b3;
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
    use crate::field::extension_field::quartic::{QuarticCrandallField, QuarticFieldExtension};
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
        assert_eq!(
            x + x,
            x * <F as QuarticFieldExtension>::BaseField::TWO.into()
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
            exp_naive(x, <F as QuarticFieldExtension>::BaseField::ORDER as u128),
            x.frobenius()
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
}
