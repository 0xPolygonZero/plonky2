use std::ops::{Add, Div, Mul, Neg, Sub};

use ethereum_types::U256;
use rand::{thread_rng, Rng};

pub const BN_BASE: U256 = U256([
    0x3c208c16d87cfd47,
    0x97816a916871ca8d,
    0xb85045b68181585d,
    0x30644e72e131a029,
]);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fp {
    pub val: U256,
}

impl Fp {
    pub fn new(val: usize) -> Fp {
        Fp {
            val: U256::from(val),
        }
    }
}

impl Add for Fp {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Fp {
            val: (self.val + other.val) % BN_BASE,
        }
    }
}

impl Neg for Fp {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fp {
            val: (BN_BASE - self.val) % BN_BASE,
        }
    }
}

impl Sub for Fp {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fp {
            val: (BN_BASE + self.val - other.val) % BN_BASE,
        }
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Mul for Fp {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fp {
            val: U256::try_from((self.val).full_mul(other.val) % BN_BASE).unwrap(),
        }
    }
}

impl Fp {
    pub fn inv(self) -> Fp {
        exp_fp(self, BN_BASE - 2)
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for Fp {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

pub const ZERO_FP: Fp = Fp { val: U256::zero() };
pub const UNIT_FP: Fp = Fp { val: U256::one() };

fn exp_fp(x: Fp, e: U256) -> Fp {
    let mut current = x;
    let mut product = Fp { val: U256::one() };

    for j in 0..256 {
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
    /// We preemptively define a helper function which multiplies an Fp2 element by 9 + i
    fn i9(self) -> Fp2 {
        let nine = Fp::new(9);
        Fp2 {
            re: nine * self.re - self.im,
            im: self.re + nine * self.im,
        }
    }

    pub fn scale(self, x: Fp) -> Fp2 {
        Fp2 {
            re: x * self.re,
            im: x * self.im,
        }
    }

    // This function takes the complex conjugate
    fn conj(self) -> Fp2 {
        Fp2 {
            re: self.re,
            im: -self.im,
        }
    }

    // Return the magnitude of the complex number
    fn norm(self) -> Fp {
        self.re * self.re + self.im * self.im
    }

    // This function normalizes the input to the complex unit circle
    fn normalize(self) -> Fp2 {
        let norm = self.norm();
        self.scale(UNIT_FP / norm)
    }
    /// The inverse of z is given by z'/||z|| since ||z|| = zz'
    pub fn inv(self) -> Fp2 {
        let norm = self.re * self.re + self.im * self.im;
        self.conj().scale(norm)
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for Fp2 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

/// The degree 3 field extension Fp6 over Fp2 is given by adjoining t, where t^3 = 9 + i
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
            t0: self.t0 * other.t0 + (self.t1 * other.t2 + self.t2 * other.t1).i9(),
            t1: self.t0 * other.t1 + self.t1 * other.t0 + (self.t2 * other.t2).i9(),
            t2: self.t0 * other.t2 + self.t1 * other.t1 + self.t2 * other.t0,
        }
    }
}

impl Fp6 {
    fn scale(self, x: Fp2) -> Fp6 {
        Fp6 {
            t0: x * self.t0,
            t1: x * self.t1,
            t2: x * self.t2,
        }
    }

    /// This function multiplies an Fp6 element by t, and hence shifts the bases,
    /// where the t^2 coefficient picks up a factor of 9+i as the 1 coefficient of the output
    fn sh(self) -> Fp6 {
        Fp6 {
            t0: self.t2.i9(),
            t1: self.t0,
            t2: self.t1,
        }
    }

    /// The nth frobenius endomorphism of a p^q field is given by mapping
    ///     x to x^(p^n)
    /// which sends a + bt + ct^2: Fp6 to
    ///     a^(p^n) + b^(p^n) * t^(p^n) + c^(p^n) * t^(2p^n)
    /// Note that p == 3 mod 4, and i^3 = -i, so x + yi gets mapped to
    ///     (x + yi)^(p^n) = x^(p^n) + y^(p^n) i^(p^n) = x + y i^(p^n mod 4)
    /// which reduces to x + yi for n even and x - yi for n odd
    /// The values of t^(p^n) and t^(2p^n) are precomputed in
    /// the constant arrays FROB_T1 and FROB_T2
    fn frob(self, n: usize) -> Fp6 {
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
    /// Thus phi = norm(x_1 * x_3 * x_5), and hence the inverse is given by
    ///     normalize([x_1 * x_3] * x_5) * [x_1 * x_3]_1
    pub fn inv(self) -> Fp6 {
        let prod_13 = self.frob(1) * self.frob(3);
        let prod_135 = (prod_13 * self.frob(5)).t0;
        let prod_odds_over_phi = prod_135.normalize();
        let prod_24 = prod_13.frob(1);
        prod_24.scale(prod_odds_over_phi)
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
    fn conj(self) -> Fp12 {
        Fp12 {
            z0: self.z0,
            z1: -self.z1,
        }
    }

    fn scale(self, x: Fp6) -> Fp12 {
        Fp12 {
            z0: x * self.z0,
            z1: x * self.z1,
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
    ///     x_6 = (a + bz)_6 = a - bz = conj_fp12(x)
    /// Letting prod_17 = x_1 * x_7, the remaining factors in the numerator can be expresed as:
    ///     [(prod_17) * (prod_17)_2] * (prod_17)_4 * [(prod_17) * (prod_17)_2]_1
    /// By Galois theory, both the following are in Fp2 and are complex conjugates
    ///     prod_odds,  prod_evens
    /// Thus phi = norm(prod_odds), and hence the inverse is given by
    ///    normalize(prod_odds) * prod_evens_except_six * conj_fp12(x)
    pub fn inv(self) -> Fp12 {
        let prod_17 = (self.frob(1) * self.frob(7)).z0;
        let prod_1379 = prod_17 * prod_17.frob(2);
        let prod_odds = (prod_1379 * prod_17.frob(4)).t0;
        let prod_odds_over_phi = prod_odds.normalize();
        let prod_evens_except_six = prod_1379.frob(1);
        let prod_penultimate = prod_evens_except_six.scale(prod_odds_over_phi);
        self.conj().scale(prod_penultimate)
    }
}

#[allow(clippy::suspicious_arithmetic_impl)]
impl Div for Fp12 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inv()
    }
}

const FROB_T1: [Fp2; 6] = [
    Fp2 {
        re: Fp { val: U256::one() },
        im: Fp { val: U256::zero() },
    },
    Fp2 {
        re: Fp {
            val: U256([
                0x99e39557176f553d,
                0xb78cc310c2c3330c,
                0x4c0bec3cf559b143,
                0x2fb347984f7911f7,
            ]),
        },
        im: Fp {
            val: U256([
                0x1665d51c640fcba2,
                0x32ae2a1d0b7c9dce,
                0x4ba4cc8bd75a0794,
                0x16c9e55061ebae20,
            ]),
        },
    },
    Fp2 {
        re: Fp {
            val: U256([
                0xe4bd44e5607cfd48,
                0xc28f069fbb966e3d,
                0x5e6dd9e7e0acccb0,
                0x30644e72e131a029,
            ]),
        },
        im: Fp { val: U256::zero() },
    },
    Fp2 {
        re: Fp {
            val: U256([
                0x7b746ee87bdcfb6d,
                0x805ffd3d5d6942d3,
                0xbaff1c77959f25ac,
                0x0856e078b755ef0a,
            ]),
        },
        im: Fp {
            val: U256([
                0x380cab2baaa586de,
                0x0fdf31bf98ff2631,
                0xa9f30e6dec26094f,
                0x04f1de41b3d1766f,
            ]),
        },
    },
    Fp2 {
        re: Fp {
            val: U256([
                0x5763473177fffffe,
                0xd4f263f1acdb5c4f,
                0x59e26bcea0d48bac,
                0x0,
            ]),
        },
        im: Fp { val: U256::zero() },
    },
    Fp2 {
        re: Fp {
            val: U256([
                0x62e913ee1dada9e4,
                0xf71614d4b0b71f3a,
                0x699582b87809d9ca,
                0x28be74d4bb943f51,
            ]),
        },
        im: Fp {
            val: U256([
                0xedae0bcec9c7aac7,
                0x54f40eb4c3f6068d,
                0xc2b86abcbe01477a,
                0x14a88ae0cb747b99,
            ]),
        },
    },
];

const FROB_T2: [Fp2; 6] = [
    Fp2 {
        re: Fp { val: U256::one() },
        im: Fp { val: U256::zero() },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x848a1f55921ea762,
                    0xd33365f7be94ec72,
                    0x80f3c0b75a181e84,
                    0x05b54f5e64eea801,
                ]),
            }
        },
        im: {
            Fp {
                val: U256([
                    0xc13b4711cd2b8126,
                    0x3685d2ea1bdec763,
                    0x9f3a80b03b0b1c92,
                    0x2c145edbe7fd8aee,
                ]),
            }
        },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x5763473177fffffe,
                    0xd4f263f1acdb5c4f,
                    0x59e26bcea0d48bac,
                    0x0,
                ]),
            }
        },
        im: { Fp { val: U256::zero() } },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x0e1a92bc3ccbf066,
                    0xe633094575b06bcb,
                    0x19bee0f7b5b2444e,
                    0xbc58c6611c08dab,
                ]),
            }
        },
        im: {
            Fp {
                val: U256([
                    0x5fe3ed9d730c239f,
                    0xa44a9e08737f96e5,
                    0xfeb0f6ef0cd21d04,
                    0x23d5e999e1910a12,
                ]),
            }
        },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0xe4bd44e5607cfd48,
                    0xc28f069fbb966e3d,
                    0x5e6dd9e7e0acccb0,
                    0x30644e72e131a029,
                ]),
            }
        },
        im: { Fp { val: U256::zero() } },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0xa97bda050992657f,
                    0xde1afb54342c724f,
                    0x1d9da40771b6f589,
                    0x1ee972ae6a826a7d,
                ]),
            }
        },
        im: {
            Fp {
                val: U256([
                    0x5721e37e70c255c9,
                    0x54326430418536d1,
                    0xd2b513cdbb257724,
                    0x10de546ff8d4ab51,
                ]),
            }
        },
    },
];

const FROB_Z: [Fp2; 12] = [
    Fp2 {
        re: { Fp { val: U256::one() } },
        im: { Fp { val: U256::zero() } },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0xd60b35dadcc9e470,
                    0x5c521e08292f2176,
                    0xe8b99fdd76e68b60,
                    0x1284b71c2865a7df,
                ]),
            }
        },
        im: {
            Fp {
                val: U256([
                    0xca5cf05f80f362ac,
                    0x747992778eeec7e5,
                    0xa6327cfe12150b8e,
                    0x246996f3b4fae7e6,
                ]),
            }
        },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0xe4bd44e5607cfd49,
                    0xc28f069fbb966e3d,
                    0x5e6dd9e7e0acccb0,
                    0x30644e72e131a029,
                ]),
            }
        },
        im: { Fp { val: U256::zero() } },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0xe86f7d391ed4a67f,
                    0x894cb38dbe55d24a,
                    0xefe9608cd0acaa90,
                    0x19dc81cfcc82e4bb,
                ]),
            }
        },
        im: {
            Fp {
                val: U256([
                    0x7694aa2bf4c0c101,
                    0x7f03a5e397d439ec,
                    0x06cbeee33576139d,
                    0xabf8b60be77d73,
                ]),
            }
        },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0xe4bd44e5607cfd48,
                    0xc28f069fbb966e3d,
                    0x5e6dd9e7e0acccb0,
                    0x30644e72e131a029,
                ]),
            }
        },
        im: { Fp { val: U256::zero() } },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x1264475e420ac20f,
                    0x2cfa95859526b0d4,
                    0x072fc0af59c61f30,
                    0x757cab3a41d3cdc,
                ]),
            }
        },
        im: {
            Fp {
                val: U256([
                    0xe85845e34c4a5b9c,
                    0xa20b7dfd71573c93,
                    0x18e9b79ba4e2606c,
                    0xca6b035381e35b6,
                ]),
            }
        },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x3c208c16d87cfd46,
                    0x97816a916871ca8d,
                    0xb85045b68181585d,
                    0x30644e72e131a029,
                ]),
            }
        },
        im: { Fp { val: U256::zero() } },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x6615563bfbb318d7,
                    0x3b2f4c893f42a916,
                    0xcf96a5d90a9accfd,
                    0x1ddf9756b8cbf849,
                ]),
            }
        },
        im: {
            Fp {
                val: U256([
                    0x71c39bb757899a9b,
                    0x2307d819d98302a7,
                    0x121dc8b86f6c4ccf,
                    0x0bfab77f2c36b843,
                ]),
            }
        },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x5763473177fffffe,
                    0xd4f263f1acdb5c4f,
                    0x59e26bcea0d48bac,
                    0x0,
                ]),
            }
        },
        im: { Fp { val: U256::zero() } },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x53b10eddb9a856c8,
                    0x0e34b703aa1bf842,
                    0xc866e529b0d4adcd,
                    0x1687cca314aebb6d,
                ]),
            }
        },
        im: {
            Fp {
                val: U256([
                    0xc58be1eae3bc3c46,
                    0x187dc4add09d90a0,
                    0xb18456d34c0b44c0,
                    0x2fb855bcd54a22b6,
                ]),
            }
        },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x5763473177ffffff,
                    0xd4f263f1acdb5c4f,
                    0x59e26bcea0d48bac,
                    0x0,
                ]),
            }
        },
        im: { Fp { val: U256::zero() } },
    },
    Fp2 {
        re: {
            Fp {
                val: U256([
                    0x29bc44b896723b38,
                    0x6a86d50bd34b19b9,
                    0xb120850727bb392d,
                    0x290c83bf3d14634d,
                ]),
            }
        },
        im: {
            Fp {
                val: U256([
                    0x53c846338c32a1ab,
                    0xf575ec93f71a8df9,
                    0x9f668e1adc9ef7f0,
                    0x23bd9e3da9136a73,
                ]),
            }
        },
    },
];

pub fn gen_fp() -> Fp {
    let mut rng = thread_rng();
    let x64 = rng.gen::<u64>();
    let x256 = U256([x64, x64, x64, x64]) % BN_BASE;
    Fp { val: x256 }
}

pub fn gen_fp2() -> Fp2 {
    Fp2 {
        re: gen_fp(),
        im: gen_fp(),
    }
}

pub fn gen_fp6() -> Fp6 {
    Fp6 {
        t0: gen_fp2(),
        t1: gen_fp2(),
        t2: gen_fp2(),
    }
}

pub fn gen_fp12() -> Fp12 {
    Fp12 {
        z0: gen_fp6(),
        z1: gen_fp6(),
    }
}
