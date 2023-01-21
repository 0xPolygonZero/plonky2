use ethereum_types::U256;

use crate::bn254_arithmetic::{
    frob_fp12, inv_fp12, make_fp, mul_fp_fp2, sparse_embed, Fp, Fp12, Fp2, UNIT_FP12,
};

// The curve consists of pairs (x, y): (Fp, Fp) | y^2 = x^3 + 2
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Curve {
    x: Fp,
    y: Fp,
}

// The twisted consists of pairs (x, y): (Fp2, Fp2) |
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TwistedCurve {
    x: Fp2,
    y: Fp2,
}

// The tate pairing takes point each from the curve and its twist and outputs an Fp12
pub fn tate(p: Curve, q: TwistedCurve) -> Fp12 {
    let miller_output = miller_loop(p, q);
    let post_mul_1 = frob_fp12(6, miller_output) / miller_output;
    let post_mul_2 = frob_fp12(2, post_mul_1) * post_mul_1;
    let power_output = power(post_mul_2);
    frob_fp12(3, post_mul_2) * power_output
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
        o = curve_double(o);
        if i != 0 {
            line = cord(p, o, q);
            acc = line * acc;
            o = curve_add(p, o);
        }
    }
    acc
}

pub fn power(f: Fp12) -> Fp12 {
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

    y0 = inv_fp12(y0);

    y4 = y4 * y2;
    y4 = y4 * y2;
    y4 = y4 * y0;

    y4 = frob_fp12(1, y4);
    y2 = frob_fp12(2, y2);

    y4 * y2 * y0
}

pub fn tangent(p: Curve, q: TwistedCurve) -> Fp12 {
    let cx = -make_fp(3) * p.x * p.x;
    let cy = make_fp(2) * p.y;
    sparse_embed(
        p.y * p.y - make_fp(9),
        mul_fp_fp2(cx, q.x),
        mul_fp_fp2(cy, q.y),
    )
}

pub fn cord(p1: Curve, p2: Curve, q: TwistedCurve) -> Fp12 {
    let cx = p2.y - p1.y;
    let cy = p1.x - p2.x;

    sparse_embed(
        p1.y * p2.x - p2.y * p1.x,
        mul_fp_fp2(cx, q.x),
        mul_fp_fp2(cy, q.y),
    )
}

fn third_point(m: Fp, p: Curve, q: Curve) -> Curve {
    let x = m * m - (p.x + q.x);
    Curve {
        x,
        y: m * (p.x - x) - p.y,
    }
}

fn curve_add(p: Curve, q: Curve) -> Curve {
    if p == q {
        curve_double(p)
    } else {
        let slope = (q.y - p.y) / (q.x - p.x);
        third_point(slope, p, q)
    }
}

fn curve_double(p: Curve) -> Curve {
    let slope = p.x * p.x * make_fp(3) / (p.y * make_fp(2));
    third_point(slope, p, p)
}

// This curve is cyclic with generator (1, 2)
pub fn curve_generator() -> Curve {
    Curve {
        x: make_fp(1),
        y: make_fp(2),
    }
}

// This curve is cyclic with generator (x, y) as follows
pub fn twisted_curve_generator() -> TwistedCurve {
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
}
