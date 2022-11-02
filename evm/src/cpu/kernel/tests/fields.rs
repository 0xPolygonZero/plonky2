use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run_with_kernel;

// TODO: 107 is hardcoded as a dummy prime for testing
// should be changed to the proper implementation prime
// once the run_{add, mul, sub}fp254 fns are implemented
const P254: u32 = 107;

fn add_fp(x: u32, y: u32) -> u32 {
    (x + y) % P254
}

fn add3_fp(x: u32, y: u32, z: u32) -> u32 {
    (x + y + z) % P254
}

fn mul_fp(x: u32, y: u32) -> u32 {
    (x * y) % P254
}

fn sub_fp(x: u32, y: u32) -> u32 {
    (P254 + x - y) % P254
}

fn add_fp2(a: [u32; 2], b: [u32; 2]) -> [u32; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    [add_fp(a, b), add_fp(a_, b_)]
}

fn add3_fp2(a: [u32; 2], b: [u32; 2], c: [u32; 2]) -> [u32; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    let [c, c_] = c;
    [add3_fp(a, b, c), add3_fp(a_, b_, c_)]
}

// fn sub_fp2(a: [u32; 2], b: [u32; 2]) -> [u32; 2] {
//     let [a, a_] = a;
//     let [b, b_] = b;
//     [sub_fp(a, b), sub_fp(a_, b_)]
// }

fn mul_fp2(a: [u32; 2], b: [u32; 2]) -> [u32; 2] {
    let [a, a_] = a;
    let [b, b_] = b;
    [
        sub_fp(mul_fp(a, b), mul_fp(a_, b_)),
        add_fp(mul_fp(a, b_), mul_fp(a_, b)),
    ]
}

fn i9(a: [u32; 2]) -> [u32; 2] {
    let [a, a_] = a;
    [sub_fp(mul_fp(9, a), a_), add_fp(a, mul_fp(9, a_))]
}

// fn add_fp6(c: [[u32; 2]; 3], d: [[u32; 2]; 3]) -> [[u32; 2]; 3] {
//     let [c0, c1, c2] = c;
//     let [d0, d1, d2] = d;

//     let e0 = add_fp2(c0, d0);
//     let e1 = add_fp2(c1, d1);
//     let e2 = add_fp2(c2, d2);
//     [e0, e1, e2]
// }

// fn sub_fp6(c: [[u32; 2]; 3], d: [[u32; 2]; 3]) -> [[u32; 2]; 3] {
//     let [c0, c1, c2] = c;
//     let [d0, d1, d2] = d;

//     let e0 = sub_fp2(c0, d0);
//     let e1 = sub_fp2(c1, d1);
//     let e2 = sub_fp2(c2, d2);
//     [e0, e1, e2]
// }

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

// fn sh(c: [[u32; 2]; 3]) -> [[u32; 2]; 3] {
//     let [c0, c1, c2] = c;
//     [i9(c2), c0, c1]
// }

// fn mul_fp12(f: [[[u32; 2]; 3]; 2], g: [[[u32; 2]; 3]; 2]) -> [[[u32; 2]; 3]; 2] {
//     let [f0, f1] = f;
//     let [g0, g1] = g;

//     let h0 = mul_fp6(f0, g0);
//     let h1 = mul_fp6(f1, g1);
//     let h01 = mul_fp6(add_fp6(f0, f1), add_fp6(g0, g1));
//     [add_fp6(h0, sh(h1)), sub_fp6(h01, add_fp6(h0, h1))]
// }

fn gen_fp6() -> [[u32; 2]; 3] {
    let mut rng = thread_rng();
    [
        [rng.gen_range(0..P254), rng.gen_range(0..P254)],
        [rng.gen_range(0..P254), rng.gen_range(0..P254)],
        [rng.gen_range(0..P254), rng.gen_range(0..P254)],
    ]
}

fn as_stack(xs: Vec<u32>) -> Vec<U256> {
    xs.iter().map(|&x| U256::from(x)).rev().collect()
}

#[test]
fn test_fp6() -> Result<()> {
    let c = gen_fp6();
    let d = gen_fp6();

    let mut input: Vec<u32> = [c, d].into_iter().flatten().flatten().collect();
    input.push(0xdeadbeef);

    let kernel = combined_kernel();
    let initial_offset = kernel.global_labels["mul_fp6"];
    let initial_stack: Vec<U256> = as_stack(input);
    let final_stack: Vec<U256> = run_with_kernel(&kernel, initial_offset, initial_stack)?
        .stack()
        .to_vec();

    let output: Vec<u32> = mul_fp6(c, d).into_iter().flatten().collect();
    let expected = as_stack(output);

    assert_eq!(final_stack, expected);

    Ok(())
}

// fn make_initial_stack(
//     f0: [[u32; 2]; 3],
//     f1: [[u32; 2]; 3],
//     g0: [[u32; 2]; 3],
//     g1: [[u32; 2]; 3],
// ) -> Vec<U256> {
//     // stack: in0, f, in0', f', in1, g, in1', g', in1, out, in0, out
//     let f0: Vec<u32> = f0.into_iter().flatten().collect();
//     let f1: Vec<u32> = f1.into_iter().flatten().collect();
//     let g0: Vec<u32> = g0.into_iter().flatten().collect();
//     let g1: Vec<u32> = g1.into_iter().flatten().collect();

//     let mut input = f0;
//     input.extend(vec![0]);
//     input.extend(f1);
//     input.extend(g0);
//     input.extend(vec![12]);
//     input.extend(g1);
//     input.extend(vec![12, 24, 0, 24]);

//     as_stack(input)
// }

// #[test]
// fn test_fp12() -> Result<()> {
//     let f0 = gen_fp6();
//     let f1 = gen_fp6();
//     let g0 = gen_fp6();
//     let g1 = gen_fp6();

//     let kernel = combined_kernel();
//     let initial_offset = kernel.global_labels["test_mul_Fp12"];
//     let initial_stack: Vec<U256> = make_initial_stack(f0, f1, g0, g1);
//     let final_stack: Vec<U256> = run_with_kernel(&kernel, initial_offset, initial_stack)?
//         .stack()
//         .to_vec();

//     let mut output: Vec<u32> = mul_fp12([f0, f1], [g0, g1])
//         .into_iter()
//         .flatten()
//         .flatten()
//         .collect();
//     output.extend(vec![24]);
//     let expected = as_stack(output);

//     assert_eq!(final_stack, expected);

//     Ok(())
// }
