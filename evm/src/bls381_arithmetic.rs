use std::mem::transmute;
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
        let xs = rng.gen::<[u64; 8]>();
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

impl Fp {
    fn lsh_128(self) -> Fp {
        let b128: U512 = U512([0, 0, 1, 0, 0, 0, 0, 0]);
        // since BLS_BASE < 2^384, multiplying by 2^128 doesn't overflow the U512
        Fp {
            val: self.val.saturating_mul(b128) % BLS_BASE,
        }
    }

    fn lsh_256(self) -> Fp {
        self.lsh_128().lsh_128()
    }

    fn lsh_512(self) -> Fp {
        self.lsh_256().lsh_256()
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Mul for Fp {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let b256: U512 = U512([0, 0, 0, 0, 1, 0, 0, 0]);
        // x1, y1 are at most (q-1) // 2^256 < 2^125
        let (x0, x1) = self.val.div_mod(b256);
        let (y0, y1) = other.val.div_mod(b256);

        let z00 = Fp {
            val: x0.saturating_mul(y0) % BLS_BASE,
        };
        let z01 = Fp {
            val: x0.saturating_mul(y1),
        };
        let z10 = Fp {
            val: x1.saturating_mul(y0),
        };
        let z11 = Fp {
            val: x1.saturating_mul(y1),
        };

        z00 + (z01 + z10).lsh_256() + z11.lsh_512()
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

/// The degree 2 field extension Fp2 is given by adjoining i, the square root of -1, to Fp
/// The arithmetic in this extension is standard complex arithmetic
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fp2 {
    pub re: Fp,
    pub im: Fp,
}

pub const ZERO_FP2: Fp2 = Fp2 {
    re: ZERO_FP,
    im: ZERO_FP,
};

pub const UNIT_FP2: Fp2 = Fp2 {
    re: UNIT_FP,
    im: ZERO_FP,
};

impl Distribution<Fp2> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp2 {
        let (re, im) = rng.gen::<(Fp, Fp)>();
        Fp2 { re, im }
    }
}

impl Add for Fp2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Fp2 {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }
}

impl Neg for Fp2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fp2 {
            re: -self.re,
            im: -self.im,
        }
    }
}

impl Sub for Fp2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fp2 {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }
}

impl Mul for Fp2 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fp2 {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
}

impl Fp2 {
    // We preemptively define a helper function which multiplies an Fp2 element by 1 + i
    fn i1(self) -> Fp2 {
        Fp2 {
            re: self.re - self.im,
            im: self.re + self.im,
        }
    }

    // This function scalar multiplies an Fp2 by an Fp
    pub fn scale(self, x: Fp) -> Fp2 {
        Fp2 {
            re: x * self.re,
            im: x * self.im,
        }
    }

    /// Return the complex conjugate z' of z: Fp2
    /// This also happens to be the frobenius map
    ///     z -> z^p
    /// since p == 3 mod 4 and hence
    ///     i^p = i^3 = -i
    fn conj(self) -> Fp2 {
        Fp2 {
            re: self.re,
            im: -self.im,
        }
    }

    // Return the magnitude squared of a complex number
    fn norm_sq(self) -> Fp {
        self.re * self.re + self.im * self.im
    }

    /// The inverse of z is given by z'/||z||^2 since ||z||^2 = zz'
    pub fn inv(self) -> Fp2 {
        let norm_sq = self.norm_sq();
        self.conj().scale(norm_sq.inv())
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for Fp2 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

/// The degree 3 field extension Fp6 over Fp2 is given by adjoining t, where t^3 = 1 + i
// Fp6 has basis 1, t, t^2 over Fp2
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fp6 {
    pub t0: Fp2,
    pub t1: Fp2,
    pub t2: Fp2,
}

pub const ZERO_FP6: Fp6 = Fp6 {
    t0: ZERO_FP2,
    t1: ZERO_FP2,
    t2: ZERO_FP2,
};

pub const UNIT_FP6: Fp6 = Fp6 {
    t0: UNIT_FP2,
    t1: ZERO_FP2,
    t2: ZERO_FP2,
};

impl Distribution<Fp6> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp6 {
        let (t0, t1, t2) = rng.gen::<(Fp2, Fp2, Fp2)>();
        Fp6 { t0, t1, t2 }
    }
}

impl Add for Fp6 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Fp6 {
            t0: self.t0 + other.t0,
            t1: self.t1 + other.t1,
            t2: self.t2 + other.t2,
        }
    }
}

impl Neg for Fp6 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fp6 {
            t0: -self.t0,
            t1: -self.t1,
            t2: -self.t2,
        }
    }
}

impl Sub for Fp6 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fp6 {
            t0: self.t0 - other.t0,
            t1: self.t1 - other.t1,
            t2: self.t2 - other.t2,
        }
    }
}

impl Mul for Fp6 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fp6 {
            t0: self.t0 * other.t0 + (self.t1 * other.t2 + self.t2 * other.t1).i1(),
            t1: self.t0 * other.t1 + self.t1 * other.t0 + (self.t2 * other.t2).i1(),
            t2: self.t0 * other.t2 + self.t1 * other.t1 + self.t2 * other.t0,
        }
    }
}

impl Fp6 {
    // This function scalar multiplies an Fp6 by an Fp2
    fn scale(self, x: Fp2) -> Fp6 {
        Fp6 {
            t0: x * self.t0,
            t1: x * self.t1,
            t2: x * self.t2,
        }
    }

    /// This function multiplies an Fp6 element by t, and hence shifts the bases,
    /// where the t^2 coefficient picks up a factor of 1+i as the 1 coefficient of the output
    fn sh(self) -> Fp6 {
        Fp6 {
            t0: self.t2.i1(),
            t1: self.t0,
            t2: self.t1,
        }
    }

    /// The nth frobenius endomorphism of a p^q field is given by mapping
    ///     x to x^(p^n)
    /// which sends a + bt + ct^2: Fp6 to
    ///     a^(p^n) + b^(p^n) * t^(p^n) + c^(p^n) * t^(2p^n)
    /// The Fp2 coefficients are determined by the comment in the conj method,
    /// while the values of
    ///     t^(p^n) and t^(2p^n)
    /// are precomputed in the constant arrays FROB_T1 and FROB_T2
    pub fn frob(self, n: usize) -> Fp6 {
        let n = n % 6;
        let frob_t1 = FROB_T1[n];
        let frob_t2 = FROB_T2[n];

        if n % 2 != 0 {
            Fp6 {
                t0: self.t0.conj(),
                t1: frob_t1 * self.t1.conj(),
                t2: frob_t2 * self.t2.conj(),
            }
        } else {
            Fp6 {
                t0: self.t0,
                t1: frob_t1 * self.t1,
                t2: frob_t2 * self.t2,
            }
        }
    }

    /// Let x_n = x^(p^n) and note that
    ///     x_0 = x^(p^0) = x^1 = x
    ///     (x_n)_m = (x^(p^n))^(p^m) = x^(p^n * p^m) = x^(p^(n+m)) = x_{n+m}
    /// By Galois Theory, given x: Fp6, the product
    ///     phi = x_0 * x_1 * x_2 * x_3 * x_4 * x_5
    /// lands in Fp, and hence the inverse of x is given by
    ///     (x_1 * x_2 * x_3 * x_4 * x_5) / phi
    /// We can save compute by rearranging the numerator:
    ///     (x_1 * x_3) * x_5 * (x_1 * x_3)_1
    /// By Galois theory, the following are in Fp2 and are complex conjugates
    ///     x_1 * x_3 * x_5,  x_0 * x_2 * x_4
    /// and therefore
    ///     phi = ||x_1 * x_3 * x_5||^2
    /// and hence the inverse is given by
    ///     ([x_1 * x_3] * x_5) * [x_1 * x_3]_1 / ||[x_1 * x_3] * x_5||^2
    pub fn inv(self) -> Fp6 {
        let prod_13 = self.frob(1) * self.frob(3);
        let prod_135 = (prod_13 * self.frob(5)).t0;
        let phi = prod_135.norm_sq();
        let prod_odds_over_phi = prod_135.scale(phi.inv());
        let prod_24 = prod_13.frob(1);
        prod_24.scale(prod_odds_over_phi)
    }

    pub fn on_stack(self) -> Vec<U512> {
        let f: [U512; 6] = unsafe { transmute(self) };
        f.into_iter().collect()
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for Fp6 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

/// The degree 2 field extension Fp12 over Fp6 is given by adjoining z, where z^2 = t.
/// It thus has basis 1, z over Fp6
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fp12 {
    pub z0: Fp6,
    pub z1: Fp6,
}

pub const UNIT_FP12: Fp12 = Fp12 {
    z0: UNIT_FP6,
    z1: ZERO_FP6,
};

impl Distribution<Fp12> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp12 {
        let (z0, z1) = rng.gen::<(Fp6, Fp6)>();
        Fp12 { z0, z1 }
    }
}

impl Mul for Fp12 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let h0 = self.z0 * other.z0;
        let h1 = self.z1 * other.z1;
        let h01 = (self.z0 + self.z1) * (other.z0 + other.z1);
        Fp12 {
            z0: h0 + h1.sh(),
            z1: h01 - (h0 + h1),
        }
    }
}

impl Fp12 {
    // This function scalar multiplies an Fp12 by an Fp6
    fn scale(self, x: Fp6) -> Fp12 {
        Fp12 {
            z0: x * self.z0,
            z1: x * self.z1,
        }
    }

    fn conj(self) -> Fp12 {
        Fp12 {
            z0: self.z0,
            z1: -self.z1,
        }
    }
    /// The nth frobenius endomorphism of a p^q field is given by mapping
    ///     x to x^(p^n)
    /// which sends a + bz: Fp12 to
    ///     a^(p^n) + b^(p^n) * z^(p^n)
    /// where the values of z^(p^n) are precomputed in the constant array FROB_Z
    pub fn frob(self, n: usize) -> Fp12 {
        let n = n % 12;
        Fp12 {
            z0: self.z0.frob(n),
            z1: self.z1.frob(n).scale(FROB_Z[n]),
        }
    }

    /// By Galois Theory, given x: Fp12, the product
    ///     phi = Prod_{i=0}^11 x_i
    /// lands in Fp, and hence the inverse of x is given by
    ///     (Prod_{i=1}^11 x_i) / phi
    /// The 6th Frob map is nontrivial but leaves Fp6 fixed and hence must be the conjugate:
    ///     x_6 = (a + bz)_6 = a - bz = x.conj()
    /// Letting prod_17 = x_1 * x_7, the remaining factors in the numerator can be expresed as:
    ///     [(prod_17) * (prod_17)_2] * (prod_17)_4 * [(prod_17) * (prod_17)_2]_1
    /// By Galois theory, both the following are in Fp2 and are complex conjugates
    ///     prod_odds,  prod_evens
    /// Thus phi = ||prod_odds||^2, and hence the inverse is given by
    ///    prod_odds * prod_evens_except_six * x.conj() / ||prod_odds||^2
    pub fn inv(self) -> Fp12 {
        let prod_17 = (self.frob(1) * self.frob(7)).z0;
        let prod_1379 = prod_17 * prod_17.frob(2);
        let prod_odds = (prod_1379 * prod_17.frob(4)).t0;
        let phi = prod_odds.norm_sq();
        let prod_odds_over_phi = prod_odds.scale(phi.inv());
        let prod_evens_except_six = prod_1379.frob(1);
        let prod_except_six = prod_evens_except_six.scale(prod_odds_over_phi);
        self.conj().scale(prod_except_six)
    }

    pub fn on_stack(self) -> Vec<U512> {
        let f: [U512; 12] = unsafe { transmute(self) };
        f.into_iter().collect()
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for Fp12 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

const FROB_T1: [Fp2; 6] = [ZERO_FP2; 6];

const FROB_T2: [Fp2; 6] = [ZERO_FP2; 6];

const FROB_Z: [Fp2; 12] = [ZERO_FP2; 12];
