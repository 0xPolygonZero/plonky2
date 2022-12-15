use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::{run_interpreter, BN_BASE};

fn add_fp(x: U256, y: U256) -> U256 {
    (x + y) % BN_BASE
}

fn add3_fp(x: U256, y: U256, z: U256) -> U256 {
    (x + y + z) % BN_BASE
}

fn mul_fp(x: U256, y: U256) -> U256 {
    U256::try_from(x.full_mul(y) % BN_BASE).unwrap()
}

fn sub_fp(x: U256, y: U256) -> U256 {
    (BN_BASE + x - y) % BN_BASE
}

fn add_fp2(a: [U256; 2], b: [U256; 2]) -> [U256; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    [add_fp(a, b), add_fp(a_, b_)]
}

fn add3_fp2(a: [U256; 2], b: [U256; 2], c: [U256; 2]) -> [U256; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    let [c, c_] = c;
    [add3_fp(a, b, c), add3_fp(a_, b_, c_)]
}

fn sub_fp2(a: [U256; 2], b: [U256; 2]) -> [U256; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    [sub_fp(a, b), sub_fp(a_, b_)]
}

fn mul_fp2(a: [U256; 2], b: [U256; 2]) -> [U256; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    [
        sub_fp(mul_fp(a, b), mul_fp(a_, b_)),
        add_fp(mul_fp(a, b_), mul_fp(a_, b)),
    ]
}

fn i9(a: [U256; 2]) -> [U256; 2] {
    let [a, a_] = a;
    [
        sub_fp(mul_fp(U256::from(9), a), a_),
        add_fp(a, mul_fp(U256::from(9), a_)),
    ]
}

fn add_fp6(c: [[U256; 2]; 3], d: [[U256; 2]; 3]) -> [[U256; 2]; 3] {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let e0 = add_fp2(c0, d0);
    let e1 = add_fp2(c1, d1);
    let e2 = add_fp2(c2, d2);
    [e0, e1, e2]
}

fn sub_fp6(c: [[U256; 2]; 3], d: [[U256; 2]; 3]) -> [[U256; 2]; 3] {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let e0 = sub_fp2(c0, d0);
    let e1 = sub_fp2(c1, d1);
    let e2 = sub_fp2(c2, d2);
    [e0, e1, e2]
}

fn mul_fp6(c: [[U256; 2]; 3], d: [[U256; 2]; 3]) -> [[U256; 2]; 3] {
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

fn sh(c: [[U256; 2]; 3]) -> [[U256; 2]; 3] {
    let [c0, c1, c2] = c;
    [i9(c2), c0, c1]
}

fn sparse_embed(x: [U256; 5]) -> [[[U256; 2]; 3]; 2] {
    let [g0, g1, g1_, g2, g2_] = x;
    let z = U256::from(0);
    [[[g0, z], [g1, g1_], [z, z]], [[z, z], [g2, g2_], [z, z]]]
}

fn mul_fp12(f: [[[U256; 2]; 3]; 2], g: [[[U256; 2]; 3]; 2]) -> [[[U256; 2]; 3]; 2] {
    let [f0, f1] = f;
    let [g0, g1] = g;

    let h0 = mul_fp6(f0, g0);
    let h1 = mul_fp6(f1, g1);
    let h01 = mul_fp6(add_fp6(f0, f1), add_fp6(g0, g1));
    [add_fp6(h0, sh(h1)), sub_fp6(h01, add_fp6(h0, h1))]
}

fn gen_fp() -> U256 {
    let mut rng = thread_rng();
    let x64 = rng.gen::<u64>();
    U256([x64, x64, x64, x64]) % BN_BASE
}

fn gen_fp6() -> [[U256; 2]; 3] {
    [
        [gen_fp(), gen_fp()],
        [gen_fp(), gen_fp()],
        [gen_fp(), gen_fp()],
    ]
}

fn gen_fp12_sparse() -> [[[U256; 2]; 3]; 2] {
    sparse_embed([gen_fp(), gen_fp(), gen_fp(), gen_fp(), gen_fp()])
}

fn make_initial_stack(
    in1: usize,
    in2: usize,
    out: usize,
    f0: [[U256; 2]; 3],
    f1: [[U256; 2]; 3],
    g0: [[U256; 2]; 3],
    g1: [[U256; 2]; 3],
) -> Vec<U256> {
    // stack: in0, f, in0', f', in1, g, in1', g', in1, out, in0, out

    let in1 = U256::from(in1);
    let in2 = U256::from(in2);
    let out = U256::from(out);

    let f0: Vec<U256> = f0.into_iter().flatten().collect();
    let f1: Vec<U256> = f1.into_iter().flatten().collect();
    let g0: Vec<U256> = g0.into_iter().flatten().collect();
    let g1: Vec<U256> = g1.into_iter().flatten().collect();

    let mut input = f0;
    input.extend(vec![in1]);
    input.extend(f1);
    input.extend(g0);
    input.extend(vec![U256::from(in2)]);
    input.extend(g1);
    input.extend(vec![in2, out, in1]);
    input.reverse();

    input
}

#[test]
fn test_fp12() -> Result<()> {
    let in1 = 64;
    let in2 = 76;
    let out = 88;

    let f0 = gen_fp6();
    let f1 = gen_fp6();
    let g0 = gen_fp6();
    let g1 = gen_fp6();

    let initial_offset = KERNEL.global_labels["test_mul_fp12"];
    let initial_stack: Vec<U256> = make_initial_stack(in1, in2, out, f0, f1, g0, g1);
    let final_stack: Vec<U256> = run_interpreter(initial_offset, initial_stack)?
        .stack()
        .to_vec();

    let expected: Vec<U256> = mul_fp12([f0, f1], [g0, g1])
        .into_iter()
        .flatten()
        .flatten()
        .rev()
        .collect();

    assert_eq!(final_stack, expected);

    Ok(())
}

// #[test]
fn test_fp12_sparse() -> Result<()> {
    let in1 = 64;
    let in2 = 76;
    let out = 88;

    let f0 = gen_fp6();
    let f1 = gen_fp6();
    let [g0, g1] = gen_fp12_sparse();

    let initial_offset = KERNEL.global_labels["test_mul_fp12"];
    let initial_stack: Vec<U256> = make_initial_stack(in1, in2, out, f0, f1, g0, g1);
    let final_stack: Vec<U256> = run_interpreter(initial_offset, initial_stack)?
        .stack()
        .to_vec();

    let expected: Vec<U256> = mul_fp12([f0, f1], [g0, g1])
        .into_iter()
        .flatten()
        .flatten()
        .rev()
        .collect();

    assert_eq!(final_stack, expected);

    Ok(())
}

// #[test]
fn test_fp12_square() -> Result<()> {
    let in1 = 64;
    let in2 = 76;
    let out = 88;

    let f0 = gen_fp6();
    let f1 = gen_fp6();

    let initial_offset = KERNEL.global_labels["test_mul_fp12"];
    let initial_stack: Vec<U256> = make_initial_stack(in1, in2, out, f0, f1, f0, f1);
    let final_stack: Vec<U256> = run_interpreter(initial_offset, initial_stack)?
        .stack()
        .to_vec();

    let expected: Vec<U256> = mul_fp12([f0, f1], [f0, f1])
        .into_iter()
        .flatten()
        .flatten()
        .rev()
        .collect();

    assert_eq!(final_stack, expected);

    Ok(())
}
