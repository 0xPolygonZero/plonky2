use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;

use crate::bn254::{
    cord, fp12_to_vec, frob_fp12, gen_curve_point, gen_fp12, gen_fp12_sparse,
    gen_twisted_curve_point, miller_loop, mul_fp12, power, tangent, Curve, Fp12, TwistedCurve,
};
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::run_interpreter;

fn make_label(lbl: &str) -> U256 {
    U256::from(KERNEL.global_labels[lbl])
}

fn make_stack(vecs: Vec<Vec<U256>>) -> Vec<U256> {
    let mut stack = vec![];
    for vec in vecs {
        stack.extend(vec)
    }
    stack
}

fn get_output(lbl: &str, stack: Vec<U256>) -> Vec<U256> {
    let label = KERNEL.global_labels[lbl];
    let mut input = stack;
    input.reverse();
    let mut output = run_interpreter(label, input).unwrap().stack().to_vec();
    output.reverse();
    output
}

fn make_mul_stack(f: Fp12, g: Fp12, mul_label: &str) -> Vec<U256> {
    let in0 = U256::from(64);
    let in1 = U256::from(76);
    let out = U256::from(88);

    make_stack(vec![
        vec![in0],
        fp12_to_vec(f),
        vec![in1],
        fp12_to_vec(g),
        vec![
            make_label(mul_label),
            in0,
            in1,
            out,
            make_label("ret_stack"),
            out,
        ],
    ])
}

#[test]
fn test_mul_fp12() -> Result<()> {
    let f: Fp12 = gen_fp12();
    let g: Fp12 = gen_fp12();
    let h: Fp12 = gen_fp12_sparse();

    let normal: Vec<U256> = make_mul_stack(f, g, "mul_fp12");
    let sparse: Vec<U256> = make_mul_stack(f, h, "mul_fp12_sparse");
    let square: Vec<U256> = make_mul_stack(f, f, "square_fp12_test");

    let out_normal: Vec<U256> = get_output("test_mul_fp12", normal);
    let out_sparse: Vec<U256> = get_output("test_mul_fp12", sparse);
    let out_square: Vec<U256> = get_output("test_mul_fp12", square);

    let exp_normal: Vec<U256> = fp12_to_vec(mul_fp12(f, g));
    let exp_sparse: Vec<U256> = fp12_to_vec(mul_fp12(f, h));
    let exp_square: Vec<U256> = fp12_to_vec(mul_fp12(f, f));

    assert_eq!(out_normal, exp_normal);
    assert_eq!(out_sparse, exp_sparse);
    assert_eq!(out_square, exp_square);

    Ok(())
}

#[test]
fn test_frob_fp12() -> Result<()> {
    let ptr = U256::from(100);

    let f: Fp12 = gen_fp12();

    let stack = make_stack(vec![vec![ptr], fp12_to_vec(f), vec![ptr]]);

    let out_frob1: Vec<U256> = get_output("test_frob_fp12_1", stack.clone());
    let out_frob2: Vec<U256> = get_output("test_frob_fp12_2", stack.clone());
    let out_frob3: Vec<U256> = get_output("test_frob_fp12_3", stack.clone());
    let out_frob6: Vec<U256> = get_output("test_frob_fp12_6", stack);

    let exp_frob1: Vec<U256> = fp12_to_vec(frob_fp12(1, f));
    let exp_frob2: Vec<U256> = fp12_to_vec(frob_fp12(2, f));
    let exp_frob3: Vec<U256> = fp12_to_vec(frob_fp12(3, f));
    let exp_frob6: Vec<U256> = fp12_to_vec(frob_fp12(6, f));

    assert_eq!(out_frob1, exp_frob1);
    assert_eq!(out_frob2, exp_frob2);
    assert_eq!(out_frob3, exp_frob3);
    assert_eq!(out_frob6, exp_frob6);

    Ok(())
}

#[test]
fn test_inv_fp12() -> Result<()> {
    let ptr = U256::from(200);
    let inv = U256::from(300);

    let f: Fp12 = gen_fp12();

    let mut stack = vec![ptr];
    stack.extend(fp12_to_vec(f));
    stack.extend(vec![ptr, inv, U256::from_str("0xdeadbeef").unwrap()]);

    let output: Vec<U256> = get_output("test_inv_fp12", stack);

    assert_eq!(output, vec![]);

    Ok(())
}

#[test]
fn test_power() -> Result<()> {
    let ptr = U256::from(300);
    let out = U256::from(400);

    let f: Fp12 = gen_fp12();

    let stack = make_stack(vec![
        vec![ptr],
        fp12_to_vec(f),
        vec![ptr, out, make_label("ret_stack"), out],
    ]);

    let output: Vec<U256> = get_output("test_pow", stack);
    let expected: Vec<U256> = fp12_to_vec(power(f));

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_miller() -> Result<()> {
    let ptr = U256::from(300);
    let out = U256::from(400);

    let p: Curve = [U256::one(), U256::from(2)];
    let q: TwistedCurve = [
        [
            U256::from_str("0x1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed")
                .unwrap(),
            U256::from_str("0x198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2")
                .unwrap(),
        ],
        [
            U256::from_str("0x12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa")
                .unwrap(),
            U256::from_str("0x90689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b")
                .unwrap(),
        ],
    ];

    let p_: Vec<U256> = p.into_iter().collect();
    let q_: Vec<U256> = q.into_iter().flatten().collect();

    let ret_stack = make_label("ret_stack");

    let initial_stack = make_stack(vec![vec![ptr], p_, q_, vec![ptr, out, ret_stack]]);

    let output = get_output("test_miller", initial_stack);
    let expected = fp12_to_vec(miller_loop(p, q));

    assert_eq!(output, expected);

    Ok(())
}
