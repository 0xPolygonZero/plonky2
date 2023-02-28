use std::ops::{Add, Div, Mul, Neg, Sub};

use ethereum_types::U512;
use rand::distributions::{Distribution, Standard};
use rand::Rng;

pub const BLS_BASE: U512 = U512([
    0xb9feffffffffaaab,
    0x1eabfffeb153ffff,
    0x6730d2a0f6b0f624,
    0x64774b84f38512bf,
    0x4b1ba7b6434bacd7,
    0x1a0111ea397fe69a,
    0x0,
    0x0,
]);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fp {
    pub val: U512,
}

impl Fp {
    pub fn new(val: usize) -> Fp {
        Fp {
            val: U512::from(val),
        }
    }
}

impl Distribution<Fp> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp {
        let xs = rng.gen::<[u64;8]>();
        Fp {
            val: U512(xs) % BLS_BASE,
        }
    }
}

impl Add for Fp {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Fp {
            val: (self.val + other.val) % BLS_BASE,
        }
    }
}

impl Neg for Fp {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fp {
            val: (BLS_BASE - self.val) % BLS_BASE,
        }
    }
}

impl Sub for Fp {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fp {
            val: (BLS_BASE + self.val - other.val) % BLS_BASE,
        }
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Mul for Fp {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fp {
            val: (self.val).overflowing_mul(other.val).0 % BLS_BASE,
        }
    }
}

impl Fp {
    pub fn inv(self) -> Fp {
        exp_fp(self, BLS_BASE - 2)
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for Fp {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

pub const ZERO_FP: Fp = Fp { val: U512::zero() };
pub const UNIT_FP: Fp = Fp { val: U512::one() };

fn exp_fp(x: Fp, e: U512) -> Fp {
    let mut current = x;
    let mut product = Fp { val: U512::one() };

    for j in 0..512 {
        if e.bit(j) {
            product = product * current;
        }
        current = current * current;
    }
    product
}
