// pub type Curve = [Fp; 2];
// pub type TwistedCurve = [Fp2; 2];

// pub fn curve_generator() -> Curve {
//     [U256::one(), U256::from(2)]
// }

// pub fn twisted_curve_generator() -> TwistedCurve {
//     [
//         [
//             U256::from_str("0x1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed")
//                 .unwrap(),
//             U256::from_str("0x198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2")
//                 .unwrap(),
//         ],
//         [
//             U256::from_str("0x12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa")
//                 .unwrap(),
//             U256::from_str("0x90689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b")
//                 .unwrap(),
//         ],
//     ]
// }

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
    let mut y0: Fp12 = embed_fp12(U256::one());
    let mut y2: Fp12 = embed_fp12(U256::one());
    let mut y4: Fp12 = embed_fp12(U256::one());

    for (a, b, c) in EXPS4 {
        if a != 0 {
            y4 = mul_fp12(y4, sq);
        }
        if b != 0 {
            y2 = mul_fp12(y2, sq);
        }
        if c != 0 {
            y0 = mul_fp12(y0, sq);
        }
        sq = mul_fp12(sq, sq);
    }
    y4 = mul_fp12(y4, sq);

    for (a, b) in EXPS2 {
        if a != 0 {
            y2 = mul_fp12(y2, sq);
        }
        if b != 0 {
            y0 = mul_fp12(y0, sq);
        }
        sq = mul_fp12(sq, sq);
    }
    y2 = mul_fp12(y2, sq);

    for a in EXPS0 {
        if a != 0 {
            y0 = mul_fp12(y0, sq);
        }
        sq = mul_fp12(sq, sq);
    }
    y0 = mul_fp12(y0, sq);

    y0 = inv_fp12(y0);

    y4 = mul_fp12(y4, y2);
    y4 = mul_fp12(y4, y2);
    y4 = mul_fp12(y4, y0);

    y4 = frob_fp12(1, y4);
    y2 = frob_fp12(2, y2);

    mul_fp12(mul_fp12(y4, y2), y0)
}

pub fn tangent(p: Curve, q: TwistedCurve) -> Fp12 {
    let [px, py] = p;
    let [qx, qy] = q;

    let cx = neg_fp(mul_fp(U256::from(3), mul_fp(px, px)));
    let cy = mul_fp(U256::from(2), py);

    sparse_embed(
        sub_fp(mul_fp(py, py), U256::from(9)),
        mul_fp2(embed_fp2(cx), qx),
        mul_fp2(embed_fp2(cy), qy),
    )
}

pub fn cord(p1: Curve, p2: Curve, q: TwistedCurve) -> Fp12 {
    let [p1x, p1y] = p1;
    let [p2x, p2y] = p2;
    let [qx, qy] = q;

    let cx = sub_fp(p2y, p1y);
    let cy = sub_fp(p1x, p2x);

    sparse_embed(
        sub_fp(mul_fp(p1y, p2x), mul_fp(p2y, p1x)),
        mul_fp2(embed_fp2(cx), qx),
        mul_fp2(embed_fp2(cy), qy),
    )
}

fn tangent_slope(p: Curve) -> Fp {
    let [px, py] = p;
    let num = mul_fp(mul_fp(px, px), U256::from(3));
    let denom = mul_fp(py, U256::from(2));
    div_fp(num, denom)
}

fn cord_slope(p: Curve, q: Curve) -> Fp {
    let [px, py] = p;
    let [qx, qy] = q;
    let num = sub_fp(qy, py);
    let denom = sub_fp(qx, px);
    div_fp(num, denom)
}

fn third_point(m: Fp, p: Curve, q: Curve) -> Curve {
    let [px, py] = p;
    let [qx, _] = q;
    let ox = sub_fp(mul_fp(m, m), add_fp(px, qx));
    let oy = sub_fp(mul_fp(m, sub_fp(px, ox)), py);
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
    let mut acc = embed_fp12(U256::one());
    let mut line;

    for i in EXP {
        acc = mul_fp12(acc, acc);
        line = tangent(o, q);
        acc = mul_fp12(line, acc);
        o = curve_double(o);
        if i != 0 {
            line = cord(p, o, q);
            acc = mul_fp12(line, acc);
            o = curve_add(p, o);
        }
    }
    acc
}

pub fn tate(p: Curve, q: TwistedCurve) -> Fp12 {
    let mut out = miller_loop(p, q);

    let inv = inv_fp12(out);
    out = frob_fp12(6, out);
    out = mul_fp12(out, inv);

    let acc = frob_fp12(2, out);
    out = mul_fp12(out, acc);

    let pow = power(out);
    out = frob_fp12(3, out);
    mul_fp12(out, pow)
}
