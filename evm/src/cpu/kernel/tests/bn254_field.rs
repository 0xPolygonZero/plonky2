use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::{run_interpreter, BN_BASE};

type Fp = U256;
type Fp2 = [U256; 2];
type Fp6 = [Fp2; 3];
type Fp12 = [Fp6; 2];

const ZERO: Fp = U256([0, 0, 0, 0]);

fn embed_fp2(x: Fp) -> Fp2 {
    [x, ZERO]
}

fn embed_fp2_fp6(a: Fp2) -> Fp6 {
    [a, embed_fp2(ZERO), embed_fp2(ZERO)]
}

fn embed_fp6(x: Fp) -> Fp6 {
    embed_fp2_fp6(embed_fp2(x))
}

fn embed_fp12(x: Fp) -> Fp12 {
    [embed_fp6(x), embed_fp6(ZERO)]
}

fn gen_fp() -> Fp {
    let mut rng = thread_rng();
    let x64 = rng.gen::<u64>();
    U256([x64, x64, x64, x64]) % BN_BASE
}

fn gen_fp6() -> Fp6 {
    [
        [gen_fp(), gen_fp()],
        [gen_fp(), gen_fp()],
        [gen_fp(), gen_fp()],
    ]
}

fn gen_fp12() -> Fp12 {
    [gen_fp6(), gen_fp6()]
}

fn gen_fp12_sparse() -> Fp12 {
    sparse_embed([gen_fp(), gen_fp(), gen_fp(), gen_fp(), gen_fp()])
}

fn add_fp(x: Fp, y: Fp) -> Fp {
    (x + y) % BN_BASE
}

fn add3_fp(x: Fp, y: Fp, z: Fp) -> Fp {
    (x + y + z) % BN_BASE
}

fn mul_fp(x: Fp, y: Fp) -> Fp {
    U256::try_from(x.full_mul(y) % BN_BASE).unwrap()
}

fn sub_fp(x: Fp, y: Fp) -> Fp {
    (BN_BASE + x - y) % BN_BASE
}

fn neg_fp(x: Fp) -> Fp {
    (BN_BASE - x) % BN_BASE
}

fn conj_fp2(a: Fp2) -> Fp2 {
    let [a, a_] = a;
    [a, neg_fp(a_)]
}

fn add_fp2(a: Fp2, b: Fp2) -> Fp2 {
    let [a, a_] = a;
    let [b, b_] = b;
    [add_fp(a, b), add_fp(a_, b_)]
}

fn add3_fp2(a: Fp2, b: Fp2, c: Fp2) -> Fp2 {
    let [a, a_] = a;
    let [b, b_] = b;
    let [c, c_] = c;
    [add3_fp(a, b, c), add3_fp(a_, b_, c_)]
}

fn sub_fp2(a: Fp2, b: Fp2) -> Fp2 {
    let [a, a_] = a;
    let [b, b_] = b;
    [sub_fp(a, b), sub_fp(a_, b_)]
}

fn neg_fp2(a: Fp2) -> Fp2 {
    sub_fp2(embed_fp2(ZERO), a)
}

fn mul_fp2(a: Fp2, b: Fp2) -> Fp2 {
    let [a, a_] = a;
    let [b, b_] = b;
    [
        sub_fp(mul_fp(a, b), mul_fp(a_, b_)),
        add_fp(mul_fp(a, b_), mul_fp(a_, b)),
    ]
}

fn i9(a: Fp2) -> Fp2 {
    let [a, a_] = a;
    let nine = U256::from(9);
    [sub_fp(mul_fp(nine, a), a_), add_fp(a, mul_fp(nine, a_))]
}

fn add_fp6(c: Fp6, d: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let e0 = add_fp2(c0, d0);
    let e1 = add_fp2(c1, d1);
    let e2 = add_fp2(c2, d2);
    [e0, e1, e2]
}

fn sub_fp6(c: Fp6, d: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let e0 = sub_fp2(c0, d0);
    let e1 = sub_fp2(c1, d1);
    let e2 = sub_fp2(c2, d2);
    [e0, e1, e2]
}

fn neg_fp6(a: Fp6) -> Fp6 {
    sub_fp6(embed_fp6(ZERO), a)
}

fn mul_fp6(c: Fp6, d: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let c0d0 = mul_fp2(c0, d0);
    let c0d1 = mul_fp2(c0, d1);
    let c0d2 = mul_fp2(c0, d2);
    let c1d0 = mul_fp2(c1, d0);
    let c1d1 = mul_fp2(c1, d1);
    let c1d2 = mul_fp2(c1, d2);
    let c2d0 = mul_fp2(c2, d0);
    let c2d1 = mul_fp2(c2, d1);
    let c2d2 = mul_fp2(c2, d2);
    let cd12 = add_fp2(c1d2, c2d1);

    [
        add_fp2(c0d0, i9(cd12)),
        add3_fp2(c0d1, c1d0, i9(c2d2)),
        add3_fp2(c0d2, c1d1, c2d0),
    ]
}

fn sh(c: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    [i9(c2), c0, c1]
}

fn sparse_embed(x: [U256; 5]) -> Fp12 {
    let [g0, g1, g1_, g2, g2_] = x;
    [
        [embed_fp2(g0), [g1, g1_], embed_fp2(ZERO)],
        [embed_fp2(ZERO), [g2, g2_], embed_fp2(ZERO)],
    ]
}

fn mul_fp12(f: Fp12, g: Fp12) -> Fp12 {
    let [f0, f1] = f;
    let [g0, g1] = g;

    let h0 = mul_fp6(f0, g0);
    let h1 = mul_fp6(f1, g1);
    let h01 = mul_fp6(add_fp6(f0, f1), add_fp6(g0, g1));
    [add_fp6(h0, sh(h1)), sub_fp6(h01, add_fp6(h0, h1))]
}

fn frob_t1(n: usize) -> Fp2 {
    match n {
        0 => [
            U256::from_str("0x1").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        1 => [
            U256::from_str("0x2fb347984f7911f74c0bec3cf559b143b78cc310c2c3330c99e39557176f553d")
                .unwrap(),
            U256::from_str("0x16c9e55061ebae204ba4cc8bd75a079432ae2a1d0b7c9dce1665d51c640fcba2")
                .unwrap(),
        ],
        2 => [
            U256::from_str("0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd48")
                .unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        3 => [
            U256::from_str("0x856e078b755ef0abaff1c77959f25ac805ffd3d5d6942d37b746ee87bdcfb6d")
                .unwrap(),
            U256::from_str("0x4f1de41b3d1766fa9f30e6dec26094f0fdf31bf98ff2631380cab2baaa586de")
                .unwrap(),
        ],
        4 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        5 => [
            U256::from_str("0x28be74d4bb943f51699582b87809d9caf71614d4b0b71f3a62e913ee1dada9e4")
                .unwrap(),
            U256::from_str("0x14a88ae0cb747b99c2b86abcbe01477a54f40eb4c3f6068dedae0bcec9c7aac7")
                .unwrap(),
        ],
        _ => panic!(),
    }
}

fn frob_t2(n: usize) -> Fp2 {
    match n {
        0 => [
            U256::from_str("0x1").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        1 => [
            U256::from_str("0x5b54f5e64eea80180f3c0b75a181e84d33365f7be94ec72848a1f55921ea762")
                .unwrap(),
            U256::from_str("0x2c145edbe7fd8aee9f3a80b03b0b1c923685d2ea1bdec763c13b4711cd2b8126")
                .unwrap(),
        ],
        2 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        3 => [
            U256::from_str("0xbc58c6611c08dab19bee0f7b5b2444ee633094575b06bcb0e1a92bc3ccbf066")
                .unwrap(),
            U256::from_str("0x23d5e999e1910a12feb0f6ef0cd21d04a44a9e08737f96e55fe3ed9d730c239f")
                .unwrap(),
        ],
        4 => [
            U256::from_str("0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd48")
                .unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        5 => [
            U256::from_str("0x1ee972ae6a826a7d1d9da40771b6f589de1afb54342c724fa97bda050992657f")
                .unwrap(),
            U256::from_str("0x10de546ff8d4ab51d2b513cdbb25772454326430418536d15721e37e70c255c9")
                .unwrap(),
        ],
        _ => panic!(),
    }
}

fn frob_z(n: usize) -> Fp2 {
    match n {
        0 => [
            U256::from_str("0x1").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        1 => [
            U256::from_str("0x1284b71c2865a7dfe8b99fdd76e68b605c521e08292f2176d60b35dadcc9e470")
                .unwrap(),
            U256::from_str("0x246996f3b4fae7e6a6327cfe12150b8e747992778eeec7e5ca5cf05f80f362ac")
                .unwrap(),
        ],
        2 => [
            U256::from_str("0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd49")
                .unwrap(),
            U256::from_str("0x0").unwrap(),
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
            U256::from_str("0x0").unwrap(),
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
            U256::from_str("0x0").unwrap(),
        ],
        7 => [
            U256::from_str("0x1ddf9756b8cbf849cf96a5d90a9accfd3b2f4c893f42a9166615563bfbb318d7")
                .unwrap(),
            U256::from_str("0xbfab77f2c36b843121dc8b86f6c4ccf2307d819d98302a771c39bb757899a9b")
                .unwrap(),
        ],
        8 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        9 => [
            U256::from_str("0x1687cca314aebb6dc866e529b0d4adcd0e34b703aa1bf84253b10eddb9a856c8")
                .unwrap(),
            U256::from_str("0x2fb855bcd54a22b6b18456d34c0b44c0187dc4add09d90a0c58be1eae3bc3c46")
                .unwrap(),
        ],
        10 => [
            U256::from_str("0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177ffffff").unwrap(),
            U256::from_str("0x0").unwrap(),
        ],
        11 => [
            U256::from_str("0x290c83bf3d14634db120850727bb392d6a86d50bd34b19b929bc44b896723b38")
                .unwrap(),
            U256::from_str("0x23bd9e3da9136a739f668e1adc9ef7f0f575ec93f71a8df953c846338c32a1ab")
                .unwrap(),
        ],
        _ => panic!(),
    }
}

fn frob_fp6(n: usize, c: Fp6) -> Fp6 {
    let [c0, c1, c2] = c;
    let _c0 = conj_fp2(c0);
    let _c1 = conj_fp2(c1);
    let _c2 = conj_fp2(c2);

    let n = n % 6;
    let frob_t1 = frob_t1(n);
    let frob_t2 = frob_t2(n);

    if n % 2 != 0 {
        [_c0, mul_fp2(frob_t1, _c1), mul_fp2(frob_t2, _c2)]
    } else {
        [c0, mul_fp2(frob_t1, c1), mul_fp2(frob_t2, c2)]
    }
}

fn frob_fp12(n: usize, f: Fp12) -> Fp12 {
    let [f0, f1] = f;
    let scale = embed_fp2_fp6(frob_z(n));

    [frob_fp6(n, f0), mul_fp6(scale, frob_fp6(n, f1))]
}

fn exp_fp(x: Fp, e: U256) -> Fp {
    let mut current = x;
    let mut product = U256::one();

    for j in 0..256 {
        if e.bit(j) {
            product = U256::try_from(product.full_mul(current) % BN_BASE).unwrap();
        }
        current = U256::try_from(current.full_mul(current) % BN_BASE).unwrap();
    }
    product
}

fn inv_fp(x: Fp) -> Fp {
    exp_fp(x, BN_BASE - 2)
}

fn inv_fp2(a: Fp2) -> Fp2 {
    let [a0, a1] = a;
    let norm = inv_fp(mul_fp(a0, a0) + mul_fp(a1, a1));
    [mul_fp(norm, a0), neg_fp(mul_fp(norm, a1))]
}

fn inv_fp6(c: Fp6) -> Fp6 {
    let b = mul_fp6(frob_fp6(1, c), frob_fp6(3, c));
    let e = mul_fp6(b, frob_fp6(5, c))[0];
    let n = mul_fp2(e, conj_fp2(e))[0];
    let i = inv_fp(n);
    let d = mul_fp2(embed_fp2(i), e);
    let [f0, f1, f2] = frob_fp6(1, b);
    [mul_fp2(d, f0), mul_fp2(d, f1), mul_fp2(d, f2)]
}

fn inv_fp12(f: Fp12) -> Fp12 {
    let a = mul_fp12(frob_fp12(1, f), frob_fp12(7, f))[0];
    let b = mul_fp6(a, frob_fp6(2, a));
    let c = mul_fp6(b, frob_fp6(4, a))[0];
    let n = mul_fp2(c, conj_fp2(c))[0];
    let i = inv_fp(n);
    let d = mul_fp2(embed_fp2(i), c);
    let [g0, g1, g2] = frob_fp6(1, b);
    let e = [mul_fp2(d, g0), mul_fp2(d, g1), mul_fp2(d, g2)];
    [mul_fp6(e, f[0]), neg_fp6(mul_fp6(e, f[1]))]
}

const EXPS4: [(bool, bool, bool); 65] = [
    (true, true, true),
    (true, true, false),
    (true, true, true),
    (true, true, true),
    (false, false, false),
    (false, false, true),
    (true, false, true),
    (false, true, false),
    (true, false, true),
    (true, true, false),
    (true, false, true),
    (false, true, false),
    (true, true, false),
    (true, true, false),
    (true, true, false),
    (false, true, false),
    (false, true, false),
    (false, false, true),
    (true, false, true),
    (true, true, false),
    (false, true, false),
    (true, true, false),
    (true, true, false),
    (true, true, false),
    (false, false, true),
    (false, false, true),
    (true, false, true),
    (true, false, true),
    (true, true, false),
    (true, false, false),
    (true, true, false),
    (false, true, false),
    (true, true, false),
    (true, false, false),
    (false, true, false),
    (false, false, false),
    (true, false, false),
    (true, false, false),
    (true, false, true),
    (false, false, true),
    (false, true, true),
    (false, false, true),
    (false, true, true),
    (false, true, true),
    (false, false, false),
    (true, true, true),
    (true, false, true),
    (true, false, true),
    (false, true, true),
    (true, false, true),
    (false, true, true),
    (false, true, true),
    (true, true, false),
    (true, true, false),
    (true, true, false),
    (true, false, false),
    (false, false, true),
    (true, false, false),
    (false, false, true),
    (true, false, true),
    (true, true, false),
    (true, true, true),
    (false, true, true),
    (false, true, false),
    (true, true, true),
];

const EXPS2: [(bool, bool); 62] = [
    (true, false),
    (true, true),
    (false, false),
    (true, false),
    (true, false),
    (true, true),
    (true, false),
    (true, true),
    (true, false),
    (false, true),
    (false, true),
    (true, true),
    (true, true),
    (false, false),
    (true, true),
    (false, false),
    (false, false),
    (false, true),
    (false, true),
    (true, true),
    (true, true),
    (true, true),
    (false, true),
    (true, true),
    (false, false),
    (true, true),
    (true, false),
    (true, true),
    (false, false),
    (true, true),
    (true, true),
    (true, false),
    (false, false),
    (false, true),
    (false, false),
    (true, true),
    (false, true),
    (false, false),
    (true, false),
    (false, true),
    (false, true),
    (true, false),
    (false, true),
    (false, false),
    (false, false),
    (false, false),
    (false, true),
    (true, false),
    (true, true),
    (false, true),
    (true, true),
    (true, false),
    (false, true),
    (false, false),
    (true, false),
    (false, true),
    (true, false),
    (true, true),
    (true, false),
    (true, true),
    (false, true),
    (true, true),
];

const EXPS0: [bool; 65] = [
    false, false, true, false, false, true, true, false, true, false, true, true, true, false,
    true, false, false, false, true, false, false, true, false, true, false, true, true, false,
    false, false, false, false, true, false, true, false, true, true, true, false, false, true,
    true, true, true, false, true, false, true, true, false, false, true, false, false, false,
    true, true, true, true, false, false, true, true, false,
];

fn fast_exp(f: Fp12) -> Fp12 {
    let mut sq: Fp12 = f;
    let mut y0: Fp12 = embed_fp12(U256::from(1));
    let mut y2: Fp12 = embed_fp12(U256::from(1));
    let mut y4: Fp12 = embed_fp12(U256::from(1));

    for (a, b, c) in EXPS4 {
        if a {
            y4 = mul_fp12(y4, sq);
        }
        if b {
            y2 = mul_fp12(y2, sq);
        }
        if c {
            y0 = mul_fp12(y0, sq);
        }
        sq = mul_fp12(sq, sq);
    }
    y4 = mul_fp12(y4, y4);

    for (a, b) in EXPS2 {
        if a {
            y2 = mul_fp12(y2, sq);
        }
        if b {
            y0 = mul_fp12(y0, sq);
        }
        sq = mul_fp12(sq, sq);
    }
    y2 = mul_fp12(y2, y2);

    for a in EXPS0 {
        if a {
            y0 = mul_fp12(y0, sq);
        }
        sq = mul_fp12(sq, sq);
    }
    y0 = mul_fp12(y0, y0);

    // TODO: y0 = inv_fp12(y0);

    y4 = mul_fp12(y4, y2);
    y4 = mul_fp12(y4, y2);
    y4 = mul_fp12(y4, y0);

    y4 = frob_fp12(1, y4);
    y2 = frob_fp12(2, y2);

    mul_fp12(mul_fp12(y4, y2), y0)
}

fn make_mul_stack(
    in0: usize,
    in1: usize,
    out: usize,
    f: Fp12,
    g: Fp12,
    mul_label: &str,
) -> Vec<U256> {
    // stack: in0, f, f', in1, g, g', mul_dest, in0, in1, out, ret_stack, out

    let in0 = U256::from(in0);
    let in1 = U256::from(in1);
    let out = U256::from(out);

    let f: Vec<U256> = f.into_iter().flatten().flatten().collect();
    let g: Vec<U256> = g.into_iter().flatten().flatten().collect();

    let ret_stack = U256::from(KERNEL.global_labels["ret_stack"]);
    let mul_dest = U256::from(KERNEL.global_labels[mul_label]);

    let mut input = vec![in0];
    input.extend(f);
    input.extend(vec![in1]);
    input.extend(g);
    input.extend(vec![mul_dest, in0, in1, out, ret_stack, out]);
    input.reverse();
    input
}

fn make_mul_expected(f: Fp12, g: Fp12) -> Vec<U256> {
    mul_fp12(f, g)
        .into_iter()
        .flatten()
        .flatten()
        .rev()
        .collect()
}

#[test]
fn test_mul_fp12() -> Result<()> {
    let in0 = 64;
    let in1 = 76;
    let out = 88;

    let f: Fp12 = gen_fp12();
    let g: Fp12 = gen_fp12();
    let h: Fp12 = gen_fp12_sparse();

    let test_mul = KERNEL.global_labels["test_mul_fp12"];

    let normal: Vec<U256> = make_mul_stack(in0, in1, out, f, g, "mul_fp12");
    let sparse: Vec<U256> = make_mul_stack(in0, in1, out, f, h, "mul_fp12_sparse");
    let square: Vec<U256> = make_mul_stack(in0, in1, out, f, f, "square_fp12_test");

    let out_normal: Vec<U256> = run_interpreter(test_mul, normal)?.stack().to_vec();
    let out_sparse: Vec<U256> = run_interpreter(test_mul, sparse)?.stack().to_vec();
    let out_square: Vec<U256> = run_interpreter(test_mul, square)?.stack().to_vec();

    let exp_normal: Vec<U256> = make_mul_expected(f, g);
    let exp_sparse: Vec<U256> = make_mul_expected(f, h);
    let exp_square: Vec<U256> = make_mul_expected(f, f);

    assert_eq!(out_normal, exp_normal);
    assert_eq!(out_sparse, exp_sparse);
    assert_eq!(out_square, exp_square);

    Ok(())
}

#[test]
fn test_inv_fp12() -> Result<()> {
    let ptr = U256::from(100);
    let inv = U256::from(200);

    let f: Fp12 = gen_fp12();
    let flat_f: Vec<U256> = f.into_iter().flatten().flatten().collect();
    let mut stack: Vec<U256> = flat_f.clone();
    stack.extend(vec![ptr, inv]);
    stack.reverse();

    let g = inv_fp12(f);
    let one = mul_fp12(f, g);
    println!("ONE? {:#?}", one);

    let mut expected: Vec<U256> = g.into_iter().flatten().flatten().collect();
    expected.extend(vec![inv]);
    expected.extend(flat_f);
    expected.extend(vec![ptr, inv]);
    expected.reverse();

    let initial_offset = KERNEL.global_labels["inverse_fp12"];
    let output: Vec<U256> = run_interpreter(initial_offset, stack)?.stack().to_vec();

    assert_eq!(output, expected);

    Ok(())
}

fn make_frob_stack(f: Fp12) -> Vec<U256> {
    let ptr = U256::from(100);
    let f: Vec<U256> = f.into_iter().flatten().flatten().collect();
    let mut input = vec![ptr];
    input.extend(f);
    input.extend(vec![ptr]);
    input.reverse();
    input
}

fn make_frob_expected(n: usize, f: Fp12) -> Vec<U256> {
    frob_fp12(n, f)
        .into_iter()
        .flatten()
        .flatten()
        .rev()
        .collect()
}

#[test]
fn test_frob_fp12() -> Result<()> {
    let f: Fp12 = gen_fp12();

    let test_frob1 = KERNEL.global_labels["test_frob_fp12_1"];
    let test_frob2 = KERNEL.global_labels["test_frob_fp12_2"];
    let test_frob3 = KERNEL.global_labels["test_frob_fp12_3"];
    let test_frob6 = KERNEL.global_labels["test_frob_fp12_6"];

    let stack = make_frob_stack(f);

    let out_frob1: Vec<U256> = run_interpreter(test_frob1, stack.clone())?.stack().to_vec();
    let out_frob2: Vec<U256> = run_interpreter(test_frob2, stack.clone())?.stack().to_vec();
    let out_frob3: Vec<U256> = run_interpreter(test_frob3, stack.clone())?.stack().to_vec();
    let out_frob6: Vec<U256> = run_interpreter(test_frob6, stack)?.stack().to_vec();

    let exp_frob1: Vec<U256> = make_frob_expected(1, f);
    let exp_frob2: Vec<U256> = make_frob_expected(2, f);
    let exp_frob3: Vec<U256> = make_frob_expected(3, f);
    let exp_frob6: Vec<U256> = make_frob_expected(6, f);

    assert_eq!(out_frob1, exp_frob1);
    assert_eq!(out_frob2, exp_frob2);
    assert_eq!(out_frob3, exp_frob3);
    assert_eq!(out_frob6, exp_frob6);

    Ok(())
}

fn make_power_stack(f: Fp12) -> Vec<U256> {
    let sqr = U256::from(100);
    let out = U256::from(300);
    let f: Vec<U256> = f.into_iter().flatten().flatten().collect();
    let ret_stack = U256::from(KERNEL.global_labels["ret_stack"]);

    let mut input = vec![sqr];
    input.extend(f);
    input.extend(vec![sqr, out, ret_stack, out]);
    input.reverse();
    input
}
