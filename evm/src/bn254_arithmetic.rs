use std::ops::{Add, Div, Mul, Neg, Sub};
use std::str::FromStr;

use ethereum_types::U256;
use rand::{thread_rng, Rng};

pub const BN_BASE: U256 = U256([
    0x3c208c16d87cfd47,
    0x97816a916871ca8d,
    0xb85045b68181585d,
    0x30644e72e131a029,
]);

#[derive(Debug, Copy, Clone)]
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
        let inv = exp_fp(self, BN_BASE - 2);
        rhs * inv
    }
}

const FP_ZERO: Fp = Fp { val: U256::zero() };

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

#[derive(Debug, Copy, Clone)]
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
        let norm = self.re * self.re + self.im * self.im;
        let inv = Fp2 {
            re: self.re / norm,
            im: -self.im / norm,
        };
        rhs * inv
    }
}

const FP2_ZERO: Fp2 = Fp2 {
    re: FP_ZERO,
    im: FP_ZERO,
};

fn flatten_fp2(a: Fp2) -> [U256; 2] {
    [a.re.val, a.im.val]
}

fn embed_fp_fp2(x: Fp) -> Fp2 {
    Fp2 { re: x, im: FP_ZERO }
}

fn conj_fp2(a: Fp2) -> Fp2 {
    Fp2 {
        re: a.re,
        im: -a.im,
    }
}

fn i9(a: Fp2) -> Fp2 {
    let nine = Fp { val: U256::from(9) };
    Fp2 {
        re: nine * a.re - a.im,
        im: a.re + nine * a.im,
    }
}

#[derive(Debug, Copy, Clone)]
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

// impl Div for Fp6 {
//     type Output = Self;

//     fn div(self, rhs: Self) -> Self::Output {
//         let b = frob_fp6(1, self) * frob_fp6(3, self);
//         let e = (b * frob_fp6(5, self)).t0;
//         let n = (e * conj_fp2(e)).re;
//         let d = e / embed_fp2(n);
//         let f = frob_fp6(1, b);
//         let inv = Fp6 {
//             t0: d * f.t0,
//             t1: d * f.t1,
//             t2: d * f.t2,
//         };
//         rhs * inv
//     }
// }

// pub fn inv_fp6(c: Fp6) -> Fp6 {
//     let b = mul_fp6(frob_fp6(1, c), frob_fp6(3, c));
//     let e = mul_fp6(b, frob_fp6(5, c))[0];
//     let n = mul_fp2(e, conj_fp2(e))[0];
//     let i = inv_fp(n);
//     let d = mul_fp2(embed_fp2(i), e);
//     let [f0, f1, f2] = frob_fp6(1, b);
//     [mul_fp2(d, f0), mul_fp2(d, f1), mul_fp2(d, f2)]
// }

fn embed_fp2_fp6(x: Fp2) -> Fp6 {
    Fp6 {
        t0: x,
        t1: FP2_ZERO,
        t2: FP2_ZERO,
    }
}

fn sh(c: Fp6) -> Fp6 {
    Fp6 {
        t0: i9(c.t2),
        t1: c.t0,
        t2: c.t1,
    }
}

#[derive(Debug, Copy, Clone)]
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

fn sparse_embed(g000: Fp, g01: Fp2, g11: Fp2) -> Fp12 {
    let g0 = Fp6 {
        t0: embed_fp_fp2(g000),
        t1: g01,
        t2: FP2_ZERO,
    };

    let g1 = Fp6 {
        t0: FP2_ZERO,
        t1: g11,
        t2: FP2_ZERO,
    };

    Fp12 { z0: g0, z1: g1 }
}

// pub fn inv_fp12(f: Fp12) -> Fp12 {
//     let [f0, f1] = f;
//     let a = mul_fp12(frob_fp12(1, f), frob_fp12(7, f))[0];
//     let b = mul_fp6(a, frob_fp6(2, a));
//     let c = mul_fp6(b, frob_fp6(4, a))[0];
//     let n = mul_fp2(c, conj_fp2(c))[0];
//     let i = inv_fp(n);
//     let d = mul_fp2(embed_fp2(i), c);
//     let [g0, g1, g2] = frob_fp6(1, b);
//     let e = [mul_fp2(d, g0), mul_fp2(d, g1), mul_fp2(d, g2)];
//     [mul_fp6(e, f0), neg_fp6(mul_fp6(e, f1))]
// }

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

// pub fn vec_to_fp12(xs: Vec<U256>) -> Fp12 {
//     xs.into_iter()
//         .tuples::<(U256, U256)>()
//         .map(|(v1, v2)| [v1, v2])
//         .tuples()
//         .map(|(a1, a2, a3, a4, a5, a6)| [[a1, a2, a3], [a4, a5, a6]])
//         .next()
//         .unwrap()
// }

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

pub fn frob_fp12(n: usize, f: Fp12) -> Fp12 {
    let scale = embed_fp2_fp6(frob_z(n));
    Fp12 {
        z0: frob_fp6(n, f.z0),
        z1: scale * frob_fp6(n, f.z1),
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
                0x856e078b755ef0a,
            ]),
        },
        im: Fp {
            val: U256([
                0x380cab2baaa586de,
                0x0fdf31bf98ff2631,
                0xa9f30e6dec26094f,
                0x4f1de41b3d1766f,
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

fn frob_z(n: usize) -> Fp2 {
    let pair = match n {
        0 => [U256::one(), U256::zero()],
        1 => [
            U256::from_str("0x1284b71c2865a7dfe8b99fdd76e68b605c521e08292f2176d60b35dadcc9e470")
                .unwrap(),
            U256::from_str("0x246996f3b4fae7e6a6327cfe12150b8e747992778eeec7e5ca5cf05f80f362ac")
                .unwrap(),
        ],
        2 => [
            U256::from_str("0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd49")
                .unwrap(),
            U256::zero(),
        ],
        3 => [
            U256::from_str("0x19dc81cfcc82e4bbefe9608cd0acaa90894cb38dbe55d24ae86f7d391ed4a67f")
                .unwrap(),
            U256::from_str("0xabf8b60be77d7306cbeee33576139d7f03a5e397d439ec7694aa2bf4c0c101")
                .unwrap(),
        ],
        4 => [
            U256::from_str("0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd48")
                .unwrap(),
            U256::zero(),
        ],
        5 => [
            U256::from_str("0x757cab3a41d3cdc072fc0af59c61f302cfa95859526b0d41264475e420ac20f")
                .unwrap(),
            U256::from_str("0xca6b035381e35b618e9b79ba4e2606ca20b7dfd71573c93e85845e34c4a5b9c")
                .unwrap(),
        ],
        6 => [
            U256::from_str("0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd46")
                .unwrap(),
            U256::zero(),
        ],
        7 => [
            U256::from_str("0x1ddf9756b8cbf849cf96a5d90a9accfd3b2f4c893f42a9166615563bfbb318d7")
                .unwrap(),
            U256::from_str("0xbfab77f2c36b843121dc8b86f6c4ccf2307d819d98302a771c39bb757899a9b")
                .unwrap(),
        ],
        8 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe").unwrap(),
            U256::zero(),
        ],
        9 => [
            U256::from_str("0x1687cca314aebb6dc866e529b0d4adcd0e34b703aa1bf84253b10eddb9a856c8")
                .unwrap(),
            U256::from_str("0x2fb855bcd54a22b6b18456d34c0b44c0187dc4add09d90a0c58be1eae3bc3c46")
                .unwrap(),
        ],
        10 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177ffffff").unwrap(),
            U256::zero(),
        ],
        11 => [
            U256::from_str("0x290c83bf3d14634db120850727bb392d6a86d50bd34b19b929bc44b896723b38")
                .unwrap(),
            U256::from_str("0x23bd9e3da9136a739f668e1adc9ef7f0f575ec93f71a8df953c846338c32a1ab")
                .unwrap(),
        ],
        _ => panic!(),
    };
    Fp2 {
        re: Fp { val: pair[0] },
        im: Fp { val: pair[1] },
    }
}
