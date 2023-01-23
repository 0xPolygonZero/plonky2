use std::ops::Add;

use ethereum_types::U256;

use crate::bn254_arithmetic::{gen_fp, gen_fp2, Fp, Fp12, Fp2, Fp6, UNIT_FP12, ZERO_FP, ZERO_FP2};

// The curve consists of pairs (x, y): (Fp, Fp) | y^2 = x^3 + 2
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Curve {
    x: Fp,
    y: Fp,
}

/// Standard addition formula for elliptic curves, restricted to the cases  
/// where neither inputs nor output would ever be the identity O. source:
/// https://en.wikipedia.org/wiki/Elliptic_curve#Algebraic_interpretation
impl Add for Curve {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let m = if self == other {
            Fp::new(3) * self.x * self.x / (Fp::new(2) * self.y)
        } else {
            (other.y - self.y) / (other.x - self.x)
        };
        let x = m * m - (self.x + other.x);
        Curve {
            x,
            y: m * (self.x - x) - self.y,
        }
    }
}

// The twisted curve consists of pairs (x, y): (Fp2, Fp2) | y^2 = x^3 + 3/(9 + i)
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TwistedCurve {
    x: Fp2,
    y: Fp2,
}

// The tate pairing takes a point each from the curve and its twist and outputs an Fp12 element
pub fn tate(p: Curve, q: TwistedCurve) -> Fp12 {
    let miller_output = miller_loop(p, q);
    invariance_inducing_power(miller_output)
}

pub fn miller_loop(p: Curve, q: TwistedCurve) -> Fp12 {
    const EXP: [usize; 253] = [
        1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 1,
        1, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 1, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0,
        1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 0, 1, 1, 0,
        1, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 0, 0, 1, 0,
        1, 1, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, 0,
        1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0,
        0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0,
        1, 0, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    let mut o = p;
    let mut acc = UNIT_FP12;
    let mut line;

    for i in EXP {
        acc = acc * acc;
        line = tangent(o, q);
        acc = line * acc;
        o = o + o;
        if i != 0 {
            line = cord(p, o, q);
            acc = line * acc;
            o = o + p;
        }
    }
    acc
}

pub fn gen_fp12_sparse() -> Fp12 {
    sparse_embed(gen_fp(), gen_fp2(), gen_fp2())
}

pub fn sparse_embed(g000: Fp, g01: Fp2, g11: Fp2) -> Fp12 {
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

pub fn tangent(p: Curve, q: TwistedCurve) -> Fp12 {
    let cx = -Fp::new(3) * p.x * p.x;
    let cy = Fp::new(2) * p.y;
    sparse_embed(p.y * p.y - Fp::new(9), q.x.scale(cx), q.y.scale(cy))
}

pub fn cord(p1: Curve, p2: Curve, q: TwistedCurve) -> Fp12 {
    let cx = p2.y - p1.y;
    let cy = p1.x - p2.x;
    sparse_embed(p1.y * p2.x - p2.y * p1.x, q.x.scale(cx), q.y.scale(cy))
}

/// The output T of the miller loop is not an invariant,
/// but one gets an invariant by raising T to the power
///     (p^12 - 1)/N = (p^6 - 1)(p^2 + 1)(p^4 - p^2 + 1)/N
/// where N is the cyclic group order of the curve.
/// To achieve this, we first exponentiate T by p^6 - 1 via
///     T = T_6 / T
/// and then exponentiate the result by p^2 + 1 via
///     T = T_2 * T
/// We then note that (p^4 - p^2 + 1)/N can be rewritten as
///     (p^4 - p^2 + 1)/N = p^3 + (a2)p^2 - (a1)p - a0
/// where 0 < a0, a1, a2 < p. Then the final power is given by
///     T = T_3 * (T^a2)_2 * (T^-a1)_1 * (T^-a0)
pub fn invariance_inducing_power(f: Fp12) -> Fp12 {
    let mut t = f.frob(6) / f;
    t = t.frob(2) * t;
    let (t_a2, t_a1, t_a0) = get_powers(t);
    t.frob(3) * t_a2.frob(2) * t_a1.frob(1) * t_a0
}

/// Given an f: Fp12, this function computes the triple
///     T^a2, T^(-a1), T^(-a0)
fn get_powers(f: Fp12) -> (Fp12, Fp12, Fp12) {
    const EXPS4: [(usize, usize, usize); 64] = [
        (1, 1, 0),
        (1, 1, 1),
        (1, 1, 1),
        (0, 0, 0),
        (0, 0, 1),
        (1, 0, 1),
        (0, 1, 0),
        (1, 0, 1),
        (1, 1, 0),
        (1, 0, 1),
        (0, 1, 0),
        (1, 1, 0),
        (1, 1, 0),
        (1, 1, 0),
        (0, 1, 0),
        (0, 1, 0),
        (0, 0, 1),
        (1, 0, 1),
        (1, 1, 0),
        (0, 1, 0),
        (1, 1, 0),
        (1, 1, 0),
        (1, 1, 0),
        (0, 0, 1),
        (0, 0, 1),
        (1, 0, 1),
        (1, 0, 1),
        (1, 1, 0),
        (1, 0, 0),
        (1, 1, 0),
        (0, 1, 0),
        (1, 1, 0),
        (1, 0, 0),
        (0, 1, 0),
        (0, 0, 0),
        (1, 0, 0),
        (1, 0, 0),
        (1, 0, 1),
        (0, 0, 1),
        (0, 1, 1),
        (0, 0, 1),
        (0, 1, 1),
        (0, 1, 1),
        (0, 0, 0),
        (1, 1, 1),
        (1, 0, 1),
        (1, 0, 1),
        (0, 1, 1),
        (1, 0, 1),
        (0, 1, 1),
        (0, 1, 1),
        (1, 1, 0),
        (1, 1, 0),
        (1, 1, 0),
        (1, 0, 0),
        (0, 0, 1),
        (1, 0, 0),
        (0, 0, 1),
        (1, 0, 1),
        (1, 1, 0),
        (1, 1, 1),
        (0, 1, 1),
        (0, 1, 0),
        (1, 1, 1),
    ];

    const EXPS2: [(usize, usize); 62] = [
        (1, 0),
        (1, 1),
        (0, 0),
        (1, 0),
        (1, 0),
        (1, 1),
        (1, 0),
        (1, 1),
        (1, 0),
        (0, 1),
        (0, 1),
        (1, 1),
        (1, 1),
        (0, 0),
        (1, 1),
        (0, 0),
        (0, 0),
        (0, 1),
        (0, 1),
        (1, 1),
        (1, 1),
        (1, 1),
        (0, 1),
        (1, 1),
        (0, 0),
        (1, 1),
        (1, 0),
        (1, 1),
        (0, 0),
        (1, 1),
        (1, 1),
        (1, 0),
        (0, 0),
        (0, 1),
        (0, 0),
        (1, 1),
        (0, 1),
        (0, 0),
        (1, 0),
        (0, 1),
        (0, 1),
        (1, 0),
        (0, 1),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (0, 1),
        (1, 1),
        (1, 0),
        (0, 1),
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (1, 0),
        (1, 1),
        (0, 1),
        (1, 1),
    ];

    const EXPS0: [usize; 65] = [
        0, 0, 1, 0, 0, 1, 1, 0, 1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 0,
        0, 0, 1, 0, 1, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 1, 1,
        0, 0, 1, 1, 0,
    ];

    let mut sq: Fp12 = f;
    let mut y0: Fp12 = UNIT_FP12;
    let mut y2: Fp12 = UNIT_FP12;
    let mut y4: Fp12 = UNIT_FP12;

    for (a, b, c) in EXPS4 {
        if a != 0 {
            y4 = y4 * sq;
        }
        if b != 0 {
            y2 = y2 * sq;
        }
        if c != 0 {
            y0 = y0 * sq;
        }
        sq = sq * sq;
    }
    y4 = y4 * sq;

    for (a, b) in EXPS2 {
        if a != 0 {
            y2 = y2 * sq;
        }
        if b != 0 {
            y0 = y0 * sq;
        }
        sq = sq * sq;
    }
    y2 = y2 * sq;

    for a in EXPS0 {
        if a != 0 {
            y0 = y0 * sq;
        }
        sq = sq * sq;
    }
    y0 = y0 * sq;

    (y2, y4 * y2 * y2 * y0, y0.inv())
}

// The curve is cyclic with generator (1, 2)
pub const CURVE_GENERATOR: Curve = {
    Curve {
        x: Fp { val: U256::one() },
        y: Fp {
            val: U256([2, 0, 0, 0]),
        },
    }
};

// The twisted curve is cyclic with generator (x, y) as follows
pub const TWISTED_GENERATOR: TwistedCurve = {
    TwistedCurve {
        x: Fp2 {
            re: Fp {
                val: U256([
                    0x46debd5cd992f6ed,
                    0x674322d4f75edadd,
                    0x426a00665e5c4479,
                    0x1800deef121f1e76,
                ]),
            },
            im: Fp {
                val: U256([
                    0x97e485b7aef312c2,
                    0xf1aa493335a9e712,
                    0x7260bfb731fb5d25,
                    0x198e9393920d483a,
                ]),
            },
        },
        y: Fp2 {
            re: Fp {
                val: U256([
                    0x4ce6cc0166fa7daa,
                    0xe3d1e7690c43d37b,
                    0x4aab71808dcb408f,
                    0x12c85ea5db8c6deb,
                ]),
            },
            im: Fp {
                val: U256([
                    0x55acdadcd122975b,
                    0xbc4b313370b38ef3,
                    0xec9e99ad690c3395,
                    0x090689d0585ff075,
                ]),
            },
        },
    }
};
