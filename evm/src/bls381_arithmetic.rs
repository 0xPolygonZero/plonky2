use std::ops::{Add, Div, Mul, Neg, Sub};

use ethereum_types::{U512};
// use rand::distributions::{Distribution, Standard};
// use rand::Rng;

pub trait FieldExt:
    Sized
    + std::ops::Add<Output = Self>
    + std::ops::Neg<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::Div<Output = Self>
{
    const ZERO: Self;
    const UNIT: Self;
    fn inv(self) -> Self;
}

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

// impl Distribution<Fp> for Standard {
//     fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp {
//         let xs = rng.gen::<[u64; 8]>();
//         Fp {
//             val: U512(xs) % BLS_BASE,
//         }
//     }
// }

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
        let (x1, x0) = self.val.div_mod(b256);
        let (y1, y0) = other.val.div_mod(b256);

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

impl FieldExt for Fp {
    const ZERO: Self = Fp { val: U512::zero() };
    const UNIT: Self = Fp { val: U512::one() };
    fn inv(self) -> Fp {
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
pub struct Fp2<T> where T: FieldExt {
    pub re: T,
    pub im: T,
}


// impl<T: Distribution<T>> Distribution<Fp2<T>> for Standard {
//     fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp2<T> {
//         let (re, im) = rng.gen::<(T, T)>();
//         Fp2 { re, im }
//     }
// }

impl<T: FieldExt> Add for Fp2<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Fp2 {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }
}

impl<T: FieldExt>  Neg for Fp2<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fp2 {
            re: -self.re,
            im: -self.im,
        }
    }
}

impl<T: FieldExt>  Sub for Fp2<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fp2 {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }
}

impl<T: FieldExt>  Mul
    for Fp2<T>
{
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fp2 {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
}

impl<T: FieldExt> Fp2<T> {
    /// This function scalar multiplies an Fp2 by an Fp
    pub fn scale(self, x: T) -> Self {
        Fp2 {
            re: x * self.re,
            im: x * self.im,
        }
    }

    /// Return the complex conjugate z' of z: Fp2
    /// This also happens to be the frobenius map
    ///     z -> z^p
    /// since p == 3 mod 4 and hence
    ///     i^p = i^(4k) * i^3 = 1*(-i) = -i
    fn conj(self) -> Self {
        Fp2 {
            re: self.re,
            im: -self.im,
        }
    }

    // Return the magnitude squared of a complex number
    fn norm_sq(self) -> T {
        self.re * self.re + self.im * self.im
    }
}

impl<T: FieldExt> FieldExt for Fp2<T> {
    const ZERO: Fp2<T> = Fp2 {
        re: T::ZERO,
        im: T::ZERO,
    };

    const UNIT: Fp2<T> = Fp2 {
        re: T::UNIT,
        im: T::ZERO,
    };
    /// The inverse of z is given by z'/||z||^2 since ||z||^2 = zz'
    fn inv(self) -> Fp2<T> {
        let norm_sq = self.norm_sq();
        self.conj().scale(norm_sq.inv())
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl<T: FieldExt> Div for Fp2<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

trait Adj {
    fn mul_adj(self) -> Self;
}

/// Helper function which multiplies by the Fp2 element
/// whose cube root we will adjoin in the next extension
impl Adj for Fp2<Fp> {
    fn mul_adj(self) -> Self {
        Fp2 {
            re: self.re - self.im,
            im: self.re + self.im,
        }
    }
}

/// The degree 3 field extension Fp6 over Fp2 is given by adjoining t, where t^3 = 1 + i
/// Fp6 has basis 1, t, t^2 over Fp2
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fp6<T> where T: FieldExt {
    pub t0: Fp2<T>,
    pub t1: Fp2<T>,
    pub t2: Fp2<T>,
}



// impl<T: Distribution<T>> Distribution<Fp6<T>> for Standard {
//     fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp6<T> {
//         let (t0, t1, t2) = rng.gen::<(Fp2<T>, Fp2<T>, Fp2<T>)>();
//         Fp6 { t0, t1, t2 }
//     }
// }

impl<T: FieldExt> Add for Fp6<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Fp6 {
            t0: self.t0 + other.t0,
            t1: self.t1 + other.t1,
            t2: self.t2 + other.t2,
        }
    }
}

impl<T: FieldExt> Neg for Fp6<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fp6 {
            t0: -self.t0,
            t1: -self.t1,
            t2: -self.t2,
        }
    }
}

impl<T: FieldExt> Sub for Fp6<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fp6 {
            t0: self.t0 - other.t0,
            t1: self.t1 - other.t1,
            t2: self.t2 - other.t2,
        }
    }
}

impl<T: FieldExt> Mul for Fp6<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fp6 {
            t0: self.t0 * other.t0 + (self.t1 * other.t2 + self.t2 * other.t1).mul_adj(),
            t1: self.t0 * other.t1 + self.t1 * other.t0 + (self.t2 * other.t2).mul_adj(),
            t2: self.t0 * other.t2 + self.t1 * other.t1 + self.t2 * other.t0,
        }
    }
}

impl<T: FieldExt> Fp6<T> {
    // This function scalar multiplies an Fp6 by an Fp2
    fn scale(self, x: Fp2<T>) -> Fp6<T> {
        Fp6 {
            t0: x * self.t0,
            t1: x * self.t1,
            t2: x * self.t2,
        }
    }
}

impl<T: FieldExt + Adj> Fp6<T> {
    /// This function multiplies an Fp6 element by t, and hence shifts the bases,
    /// where the t^2 coefficient picks up a factor of 1+i as the 1 coefficient of the output
    fn sh(self) -> Fp6<T> {
        Fp6 {
            t0: self.t2.mul_adj(),
            t1: self.t0,
            t2: self.t1,
        }
    }
}

pub trait Frob {
    const FROB_T: Self;
    const FROB_Z: Self;
}

impl<T: Frob + FieldExt> Fp6<T> {
    /// The nth frobenius endomorphism of a p^q field is given by mapping
    ///     x to x^(p^n)
    /// which sends a + bt + ct^2: Fp6 to
    ///     a^(p^n) + b^(p^n) * t^(p^n) + c^(p^n) * t^(2p^n)
    /// The Fp2 coefficients are determined by the comment in the conj method,
    /// while the values of
    ///     t^(p^n) and t^(2p^n)
    /// are precomputed in the constant arrays FROB_T1 and FROB_T2
    pub fn frob(self, n: usize) -> Fp6<T> {
        let n = n % 6;
        let frob_t1 = Self::FROB_T[0][n];
        let frob_t2 = Self::FROB_T[1][n];

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
}

impl<T: FieldExt + Adj> FieldExt for Fp6<T> {
    const ZERO: Fp6<T> = Fp6 {
        t0: Fp2::<T>::ZERO,
        t1: Fp2::<T>::ZERO,
        t2: Fp2::<T>::ZERO,
    };

    const UNIT: Fp6<T> = Fp6 {
        t0: Fp2::<T>::UNIT,
        t1: Fp2::<T>::ZERO,
        t2: Fp2::<T>::ZERO,
    };

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
    fn inv(self) -> Fp6<T> {
        let prod_13 = self.frob(1) * self.frob(3);
        let prod_135 = (prod_13 * self.frob(5)).t0;
        let phi = prod_135.norm_sq();
        let prod_odds_over_phi = prod_135.scale(phi.inv());
        let prod_24 = prod_13.frob(1);
        prod_24.scale(prod_odds_over_phi)
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl<T: FieldExt> Div for Fp6<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

// /// The degree 2 field extension Fp12 over Fp6 is given by adjoining z, where z^2 = t.
// /// It thus has basis 1, z over Fp6
// #[derive(Debug, Copy, Clone, PartialEq)]
// pub struct Fp12<T> {
//     pub z0: Fp6<T>,
//     pub z1: Fp6<T>,
// }

// impl<T: Unital> Unital for Fp12<T> {
//     const ZERO: Fp12<T> = Fp12 {
//         z0: Fp6::<T>::ZERO,
//         z1: Fp6::<T>::ZERO,
//     };

//     const UNIT: Fp12<T> = Fp12 {
//         z0: Fp6::<T>::UNIT,
//         z1: Fp6::<T>::ZERO,
//     };
// }

// // impl<T: Distribution<T>> Distribution<Fp12<T>> for Standard {
// //     fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp12<T> {
// //         let (z0, z1) = rng.gen::<(Fp6, Fp6)>();
// //         Fp12 { z0, z1 }
// //     }
// // }

// impl<T: Mul> Mul for Fp12<T> {
//     type Output = Self;

//     fn mul(self, other: Self) -> Self {
//         let h0 = self.z0 * other.z0;
//         let h1 = self.z1 * other.z1;
//         let h01 = (self.z0 + self.z1) * (other.z0 + other.z1);
//         Fp12 {
//             z0: h0 + h1.sh(),
//             z1: h01 - (h0 + h1),
//         }
//     }
// }

// impl<T> Fp12<T> {
//     // This function scalar multiplies an Fp12 by an Fp6
//     fn scale(self, x: Fp6<T>) -> Fp12<T> {
//         Fp12 {
//             z0: x * self.z0,
//             z1: x * self.z1,
//         }
//     }

//     fn conj(self) -> Fp12<T> {
//         Fp12 {
//             z0: self.z0,
//             z1: -self.z1,
//         }
//     }
// }

// impl<T: Frob> Fp12<T> {
//     /// The nth frobenius endomorphism of a p^q field is given by mapping
//     ///     x to x^(p^n)
//     /// which sends a + bz: Fp12 to
//     ///     a^(p^n) + b^(p^n) * z^(p^n)
//     /// where the values of z^(p^n) are precomputed in the constant array FROB_Z
//     pub fn frob(self, n: usize) -> Fp12<T> {
//         let n = n % 12;
//         Fp12 {
//             z0: self.z0.frob(n),
//             z1: self.z1.frob(n).scale(Self::FROB_Z[n]),
//         }
//     }

//     /// By Galois Theory, given x: Fp12, the product
//     ///     phi = Prod_{i=0}^11 x_i
//     /// lands in Fp, and hence the inverse of x is given by
//     ///     (Prod_{i=1}^11 x_i) / phi
//     /// The 6th Frob map is nontrivial but leaves Fp6 fixed and hence must be the conjugate:
//     ///     x_6 = (a + bz)_6 = a - bz = x.conj()
//     /// Letting prod_17 = x_1 * x_7, the remaining factors in the numerator can be expresed as:
//     ///     [(prod_17) * (prod_17)_2] * (prod_17)_4 * [(prod_17) * (prod_17)_2]_1
//     /// By Galois theory, both the following are in Fp2 and are complex conjugates
//     ///     prod_odds,  prod_evens
//     /// Thus phi = ||prod_odds||^2, and hence the inverse is given by
//     ///    prod_odds * prod_evens_except_six * x.conj() / ||prod_odds||^2
//     pub fn inv(self) -> Fp12<T> {
//         let prod_17 = (self.frob(1) * self.frob(7)).z0;
//         let prod_1379 = prod_17 * prod_17.frob(2);
//         let prod_odds = (prod_1379 * prod_17.frob(4)).t0;
//         let phi = prod_odds.norm_sq();
//         let prod_odds_over_phi = prod_odds.scale(phi.inv());
//         let prod_evens_except_six = prod_1379.frob(1);
//         let prod_except_six = prod_evens_except_six.scale(prod_odds_over_phi);
//         self.conj().scale(prod_except_six)
//     }
// }

// #[allow(clippy::suspicious_arithmetic_impl)]
// impl<T: std::ops::Div<Output = T>> Div for Fp12<T> {
//     type Output = Self;

//     fn div(self, rhs: Self) -> Self::Output {
//         self * rhs.inv()
//     }
// }

// trait Stack {
//     fn on_stack(self) -> Vec<U256>;
// }
