use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run_with_kernel;

const P254: u32 = 101;

fn add_fp2(a: [u32; 2], b: [u32; 2]) -> [u32; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    [(a + b) % P254, (a_ + b_) % P254]
}

fn add3_fp2(a: [u32; 2], b: [u32; 2], c: [u32; 2]) -> [u32; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    let [c, c_] = c;
    [(a + b + c) % P254, (a_ + b_ + c_) % P254]
}

fn sub_fp2(a: [u32; 2], b: [u32; 2]) -> [u32; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    [(P254 + a - b) % P254, (P254 + a_ - b_) % P254]
}

fn mul_fp2(a: [u32; 2], b: [u32; 2]) -> [u32; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    [
        (P254 + (a * b) % P254 - (a_ * b_) % P254) % P254,
        ((a * b_) % P254 + (a_ * b) % P254) % P254,
    ]
}

fn i9(a: [u32; 2]) -> [u32; 2] {
    let [a, a_] = a;
    [(P254 + 9 * a - a_) % P254, (a + 9 * a_) % P254]
}

fn add_fp6(c: [[u32; 2]; 3], d: [[u32; 2]; 3]) -> [[u32; 2]; 3] {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let e0 = add_fp2(c0, d0);
    let e1 = add_fp2(c1, d1);
    let e2 = add_fp2(c2, d2);
    [e0, e1, e2]
}

fn sub_fp6(c: [[u32; 2]; 3], d: [[u32; 2]; 3]) -> [[u32; 2]; 3] {
    let [c0, c1, c2] = c;
    let [d0, d1, d2] = d;

    let e0 = sub_fp2(c0, d0);
    let e1 = sub_fp2(c1, d1);
    let e2 = sub_fp2(c2, d2);
    [e0, e1, e2]
}

fn mul_fp6(c: [[u32; 2]; 3], d: [[u32; 2]; 3]) -> [[u32; 2]; 3] {
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

fn sh(c: [[u32; 2]; 3]) -> [[u32; 2]; 3] {
    let [c0, c1, c2] = c;
    [i9(c2), c0, c1]
}

fn mul_fp12(f: [[[u32; 2]; 3]; 2], g: [[[u32; 2]; 3]; 2]) -> [[[u32; 2]; 3]; 2] {
    let [f0, f1] = f;
    let [g0, g1] = g;

    let h0 = mul_fp6(f0, g0);
    let h1 = mul_fp6(f1, g1);
    let h01 = mul_fp6(add_fp6(f0, f1), add_fp6(g0, g1));
    [add_fp6(h0, sh(h1)), sub_fp6(h01, add_fp6(h0, h1))]
}

fn gen_fp6() -> [[u32; 2]; 3] {
    let mut rng = thread_rng();
    [
        [rng.gen_range(0..P254), rng.gen_range(0..P254)],
        [rng.gen_range(0..P254), rng.gen_range(0..P254)],
        [rng.gen_range(0..P254), rng.gen_range(0..P254)],
    ]
}

fn as_stack(xs: Vec<u32>) -> Vec<U256> {
    xs.iter()
        .map(|&x| U256::from(x as u32) % P254)
        .rev()
        .collect()
}

#[test]
fn test_fp12() -> Result<()> {
    let f = [gen_fp6(), gen_fp6()];
    let g = [gen_fp6(), gen_fp6()];
    let input: Vec<u32> = [f, g].into_iter().flatten().flatten().flatten().collect();
    let output: Vec<u32> = mul_fp12(f, g).into_iter().flatten().flatten().collect();

    let kernel = combined_kernel();
    let initial_offset = kernel.global_labels["test_mul_Fp12"];
    let initial_stack: Vec<U256> = as_stack(input);
    let final_stack: Vec<U256> = run_with_kernel(&kernel, initial_offset, initial_stack)?
        .stack()
        .to_vec();

    let expected = as_stack(output);
    assert_eq!(final_stack, expected);

    Ok(())
}
