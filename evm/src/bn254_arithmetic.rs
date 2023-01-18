use std::ops::{Add, Div, Mul, Neg, Sub};

use ethereum_types::U256;
use itertools::Itertools;
use rand::{thread_rng, Rng};

pub const BN_BASE: U256 = U256([
    0x3c208c16d87cfd47,
    0x97816a916871ca8d,
    0xb85045b68181585d,
    0x30644e72e131a029,
]);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fp {
    val: U256,
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

impl Mul for Fp {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fp {
            val: U256::try_from((self.val).full_mul(other.val) % BN_BASE).unwrap(),
        }
    }
}

impl Div for Fp {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let inv = exp_fp(rhs, BN_BASE - 2);
        self * inv
    }
}

const ZERO_FP: Fp = Fp { val: U256::zero() };
const UNIT_FP: Fp = Fp { val: U256::one() };

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
    re: Fp,
    im: Fp,
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

impl Div for Fp2 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let norm = rhs.re * rhs.re + rhs.im * rhs.im;
        let inv = Fp2 {
            re: rhs.re / norm,
            im: -rhs.im / norm,
        };
        self * inv
    }
}

const ZERO_FP2: Fp2 = Fp2 {
    re: ZERO_FP,
    im: ZERO_FP,
};

const UNIT_FP2: Fp2 = Fp2 {
    re: UNIT_FP,
    im: ZERO_FP,
};

// This function takes the complex conjugate
fn conj_fp2(a: Fp2) -> Fp2 {
    Fp2 {
        re: a.re,
        im: -a.im,
    }
}

// This function function normalizes the input to the complex unit circle
fn normalize_fp2(a: Fp2) -> Fp2 {
    let norm = a.re * a.re + a.im * a.im;
    Fp2 {
        re: a.re / norm,
        im: a.im / norm,
    }
}

/// The degree 3 field extension Fp6 over Fp2 is given by adjoining t, where t^3 = 9 + i
/// We begin by defining a helper function which multiplies an Fp2 element by 9 + i
fn i9(a: Fp2) -> Fp2 {
    let nine = Fp { val: U256::from(9) };
    Fp2 {
        re: nine * a.re - a.im,
        im: a.re + nine * a.im,
    }
}

// Fp6 has basis 1, t, t^2 over Fp2
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fp6 {
    t0: Fp2,
    t1: Fp2,
    t2: Fp2,
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
            t0: self.t0 * other.t0 + i9(self.t1 * other.t2 + self.t2 * other.t1),
            t1: self.t0 * other.t1 + self.t1 * other.t0 + i9(self.t2 * other.t2),
            t2: self.t0 * other.t2 + self.t1 * other.t1 + self.t2 * other.t0,
        }
    }
}

impl Div for Fp6 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let b = frob_fp6(1, rhs) * frob_fp6(3, rhs);
        let e = normalize_fp2((b * frob_fp6(5, rhs)).t0);
        let f = frob_fp6(1, b);
        let inv = mul_fp2_fp6(e, f);
        self * inv
    }
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

fn mul_fp2_fp6(x: Fp2, f: Fp6) -> Fp6 {
    Fp6 {
        t0: x * f.t0,
        t1: x * f.t1,
        t2: x * f.t2,
    }
}

/// This function multiplies an Fp6 element by t, and hence shifts the bases,
/// where the t^2 coefficient picks up a factor of 9+i as the 1 coefficient of the output
fn sh(c: Fp6) -> Fp6 {
    Fp6 {
        t0: i9(c.t2),
        t1: c.t0,
        t2: c.t1,
    }
}

/// The nth frobenius endomorphism is given by sending a field element r to r^(p^n)
/// Hence an Fp6 element a + bt + ct^2 is sent to
///     a^(p^n) + b^(p^n) * t^(p^n) + c^(p^n) * t^(2p^n)
/// the constant arrays FROB_T1 and FROB_T2 record the values of t^(p^n) and t^(2p^n), respectively
/// By the comment in conj_fp2, x^(p^n) = x when n is even and conj_fp2(x) when n is odd
fn frob_fp6(n: usize, c: Fp6) -> Fp6 {
    let n = n % 6;
    let frob_t1 = FROB_T1[n];
    let frob_t2 = FROB_T2[n];

    if n % 2 != 0 {
        Fp6 {
            t0: conj_fp2(c.t0),
            t1: frob_t1 * conj_fp2(c.t1),
            t2: frob_t2 * conj_fp2(c.t2),
        }
    } else {
        Fp6 {
            t0: c.t0,
            t1: frob_t1 * c.t1,
            t2: frob_t2 * c.t2,
        }
    }
}

/// The degree 2 field extension Fp12 over Fp6 is given by adjoining z, where z^2 = t.
/// It thus has basis 1, z over Fp6
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Fp12 {
    z0: Fp6,
    z1: Fp6,
}

impl Mul for Fp12 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let h0 = self.z0 * other.z0;
        let h1 = self.z1 * other.z1;
        let h01 = (self.z0 + self.z1) * (other.z0 + other.z1);
        Fp12 {
            z0: h0 + sh(h1),
            z1: h01 - (h0 + h1),
        }
    }
}

impl Div for Fp12 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let a = (frob_fp12(1, rhs) * frob_fp12(7, rhs)).z0;
        let b = a * frob_fp6(2, a);
        let c = normalize_fp2((b * frob_fp6(4, a)).t0);
        let g = frob_fp6(1, b);
        let e = mul_fp2_fp6(c, g);
        let inv = Fp12 {
            z0: e * rhs.z0,
            z1: -e * rhs.z1,
        };
        self * inv
    }
}

pub const UNIT_FP12: Fp12 = Fp12 {
    z0: UNIT_FP6,
    z1: ZERO_FP6,
};

pub fn inv_fp12(f: Fp12) -> Fp12 {
    UNIT_FP12 / f
}

fn sparse_embed(g000: Fp, g01: Fp2, g11: Fp2) -> Fp12 {
    let g0 = Fp6 {
        t0: Fp2 {
            re: g000,
            im: ZERO_FP,
        },
        t1: g01,
        t2: ZERO_FP2,
    };

    let g1 = Fp6 {
        t0: ZERO_FP2,
        t1: g11,
        t2: ZERO_FP2,
    };

    Fp12 { z0: g0, z1: g1 }
}

/// The nth frobenius endomorphism is given by sending a field element r to r^(p^n)
/// Hence an Fp12 element a + bz is sent to
///     a^(p^n) + b^(p^n) * z^(p^n)
/// the constant array FROB_Z records the values of z^p^n
pub fn frob_fp12(n: usize, f: Fp12) -> Fp12 {
    let n = n % 12;
    Fp12 {
        z0: frob_fp6(n, f.z0),
        z1: mul_fp2_fp6(FROB_Z[n], frob_fp6(n, f.z1)),
    }
}

pub fn fp12_to_array(f: Fp12) -> [U256; 12] {
    [
        f.z0.t0.re.val,
        f.z0.t0.im.val,
        f.z0.t1.re.val,
        f.z0.t1.im.val,
        f.z0.t2.re.val,
        f.z0.t2.im.val,
        f.z1.t0.re.val,
        f.z1.t0.im.val,
        f.z1.t1.re.val,
        f.z1.t1.im.val,
        f.z1.t2.re.val,
        f.z1.t2.im.val,
    ]
}

pub fn fp12_to_vec(f: Fp12) -> Vec<U256> {
    fp12_to_array(f).into_iter().collect()
}

pub fn vec_to_fp12(xs: Vec<U256>) -> Fp12 {
    xs.into_iter()
        .tuples::<(U256, U256)>()
        .map(|(v1, v2)| Fp2 {
            re: Fp { val: v1 },
            im: Fp { val: v2 },
        })
        .tuples()
        .map(|(a1, a2, a3, a4, a5, a6)| Fp12 {
            z0: Fp6 {
                t0: a1,
                t1: a2,
                t2: a3,
            },
            z1: Fp6 {
                t0: a4,
                t1: a5,
                t2: a6,
            },
        })
        .next()
        .unwrap()
}

fn gen_fp() -> Fp {
    let mut rng = thread_rng();
    let x64 = rng.gen::<u64>();
    let x256 = U256([x64, x64, x64, x64]) % BN_BASE;
    Fp { val: x256 }
}

fn gen_fp2() -> Fp2 {
    Fp2 {
        re: gen_fp(),
        im: gen_fp(),
    }
}

fn gen_fp6() -> Fp6 {
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

pub fn gen_fp12_sparse() -> Fp12 {
    sparse_embed(gen_fp(), gen_fp2(), gen_fp2())
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
                    0x5b54f5e64eea801,
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
                    0xbfab77f2c36b843,
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
