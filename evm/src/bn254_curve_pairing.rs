use ethereum_types::U256;

use crate::bn254_arithmetic::{
    frob_fp12, inv_fp12, make_fp, mul_fp_fp2, sparse_embed, Fp, Fp12, Fp2, UNIT_FP12,
};

pub type Curve = [Fp; 2];
pub type TwistedCurve = [Fp2; 2];

pub fn curve_generator() -> Curve {
    [make_fp(1), make_fp(2)]
}

pub fn twisted_curve_generator() -> TwistedCurve {
    [
        Fp2 {
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
        Fp2 {
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
                    0x90689d0585ff075,
                ]),
            },
        },
    ]
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
    let [px, py] = p;
    let [qx, qy] = q;

    let cx = -make_fp(3) * px * px;
    let cy = make_fp(2) * py;

    sparse_embed(py * py - make_fp(9), mul_fp_fp2(cx, qx), mul_fp_fp2(cy, qy))
}

pub fn cord(p1: Curve, p2: Curve, q: TwistedCurve) -> Fp12 {
    let [p1x, p1y] = p1;
    let [p2x, p2y] = p2;
    let [qx, qy] = q;

    let cx = p2y - p1y;
    let cy = p1x - p2x;

    sparse_embed(
        p1y * p2x - p2y * p1x,
        mul_fp_fp2(cx, qx),
        mul_fp_fp2(cy, qy),
    )
}

fn tangent_slope(p: Curve) -> Fp {
    let [px, py] = p;
    let num = px * px * make_fp(3);
    let denom = py * make_fp(2);
    num / denom
}

fn cord_slope(p: Curve, q: Curve) -> Fp {
    let [px, py] = p;
    let [qx, qy] = q;
    let num = qy - py;
    let denom = qx - px;
    num / denom
}

fn third_point(m: Fp, p: Curve, q: Curve) -> Curve {
    let [px, py] = p;
    let [qx, _] = q;
    let ox = m * m - (px + qx);
    let oy = m * (px - ox) - py;
    [ox, oy]
}

fn curve_add(p: Curve, q: Curve) -> Curve {
    if p == q {
        curve_double(p)
    } else {
        third_point(cord_slope(p, q), p, q)
    }
}

fn curve_double(p: Curve) -> Curve {
    third_point(tangent_slope(p), p, p)
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

pub fn tate(p: Curve, q: TwistedCurve) -> Fp12 {
    let mut out = miller_loop(p, q);

    let inv = inv_fp12(out);
    out = frob_fp12(6, out);
    out = out * inv;

    let acc = frob_fp12(2, out);
    out = out * acc;

    let pow = power(out);
    out = frob_fp12(3, out);
    out * pow
}
