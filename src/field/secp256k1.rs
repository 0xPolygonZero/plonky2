use std::convert::TryInto;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use itertools::Itertools;
use num::bigint::{BigUint, RandBigInt};
use num::{Integer, One};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::field::field_types::Field;
use crate::field::goldilocks_field::GoldilocksField;

/// The base field of the secp256k1 elliptic curve.
///
/// Its order is
/// ```ignore
/// P = 2**256 - 2**32 - 2**9 - 2**8 - 2**7 - 2**6 - 2**4 - 1
/// ```
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Secp256K1Base(pub [u64; 4]);

fn biguint_from_array(arr: [u64; 4]) -> BigUint {
    BigUint::from_slice(&[
        arr[0] as u32,
        (arr[0] >> 32) as u32,
        arr[1] as u32,
        (arr[1] >> 32) as u32,
        arr[2] as u32,
        (arr[2] >> 32) as u32,
        arr[3] as u32,
        (arr[3] >> 32) as u32,
    ])
}

impl Default for Secp256K1Base {
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq for Secp256K1Base {
    fn eq(&self, other: &Self) -> bool {
        self.to_biguint() == other.to_biguint()
    }
}

impl Eq for Secp256K1Base {}

impl Hash for Secp256K1Base {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_biguint().hash(state)
    }
}

impl Display for Secp256K1Base {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.to_biguint(), f)
    }
}

impl Debug for Secp256K1Base {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.to_biguint(), f)
    }
}

impl Field for Secp256K1Base {
    // TODO: fix
    type PrimeField = GoldilocksField;

    const ZERO: Self = Self([0; 4]);
    const ONE: Self = Self([1, 0, 0, 0]);
    const TWO: Self = Self([2, 0, 0, 0]);
    const NEG_ONE: Self = Self([
        0xFFFFFFFEFFFFFC2E,
        0xFFFFFFFFFFFFFFFF,
        0xFFFFFFFFFFFFFFFF,
        0xFFFFFFFFFFFFFFFF,
    ]);

    // TODO: fix
    const CHARACTERISTIC: u64 = 0;
    const TWO_ADICITY: usize = 1;

    // Sage: `g = GF(p).multiplicative_generator()`
    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self([5, 0, 0, 0]);

    // Sage: `g_2 = g^((p - 1) / 2)`
    const POWER_OF_TWO_GENERATOR: Self = Self::NEG_ONE;

    fn order() -> BigUint {
        BigUint::from_slice(&[
            0xFFFFFC2F, 0xFFFFFFFE, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF,
            0xFFFFFFFF,
        ])
    }

    fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        // Fermat's Little Theorem
        Some(self.exp_biguint(&(Self::order() - BigUint::one() - BigUint::one())))
    }

    fn to_biguint(&self) -> BigUint {
        let mut result = biguint_from_array(self.0);
        if result >= Self::order() {
            result -= Self::order();
        }
        result
    }

    fn from_biguint(val: BigUint) -> Self {
        Self(
            val.to_u64_digits()
                .into_iter()
                .pad_using(4, |_| 0)
                .collect::<Vec<_>>()[..]
                .try_into()
                .expect("error converting to u64 array"),
        )
    }

    #[inline]
    fn from_canonical_u64(n: u64) -> Self {
        Self([n, 0, 0, 0])
    }

    #[inline]
    fn from_noncanonical_u128(n: u128) -> Self {
        Self([n as u64, (n >> 64) as u64, 0, 0])
    }

    #[inline]
    fn from_noncanonical_u96(n: (u64, u32)) -> Self {
        Self([n.0, n.1 as u64, 0, 0])
    }

    fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self::from_biguint(rng.gen_biguint_below(&Self::order()))
    }
}

impl Neg for Secp256K1Base {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        if self.is_zero() {
            Self::ZERO
        } else {
            Self::from_biguint(Self::order() - self.to_biguint())
        }
    }
}

impl Add for Secp256K1Base {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        let mut result = self.to_biguint() + rhs.to_biguint();
        if result >= Self::order() {
            result -= Self::order();
        }
        Self::from_biguint(result)
    }
}

impl AddAssign for Secp256K1Base {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for Secp256K1Base {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl Sub for Secp256K1Base {
    type Output = Self;

    #[inline]
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: Self) -> Self {
        self + -rhs
    }
}

impl SubAssign for Secp256K1Base {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for Secp256K1Base {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::from_biguint((self.to_biguint() * rhs.to_biguint()).mod_floor(&Self::order()))
    }
}

impl MulAssign for Secp256K1Base {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Product for Secp256K1Base {
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|acc, x| acc * x).unwrap_or(Self::ONE)
    }
}

impl Div for Secp256K1Base {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl DivAssign for Secp256K1Base {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}
