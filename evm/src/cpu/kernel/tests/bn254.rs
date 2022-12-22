use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;

use crate::bn254::{
    fp12_to_vec, frob_fp12, gen_curve_point, gen_fp12, gen_fp12_sparse, gen_twisted_curve_point,
    mul_fp12, power, store_cord, store_tangent, Curve, Fp12, TwistedCurve,
};
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::run_interpreter;

fn fp12_as_stack(f: Fp12) -> Vec<U256> {
    f.into_iter().flatten().flatten().rev().collect()
}

fn make_mul_stack(
    in0: usize,
    in1: usize,
    out: usize,
    f: Fp12,
    g: Fp12,
    mul_label: &str,
) -> Vec<U256> {
    let in0 = U256::from(in0);
    let in1 = U256::from(in1);
    let out = U256::from(out);

    let ret_stack = U256::from(KERNEL.global_labels["ret_stack"]);
    let mul_dest = U256::from(KERNEL.global_labels[mul_label]);

    let mut input = vec![in0];
    input.extend(fp12_to_vec(f));
    input.extend(vec![in1]);
    input.extend(fp12_to_vec(g));
    input.extend(vec![mul_dest, in0, in1, out, ret_stack, out]);
    input.reverse();
    input
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

    let exp_normal: Vec<U256> = fp12_as_stack(mul_fp12(f, g));
    let exp_sparse: Vec<U256> = fp12_as_stack(mul_fp12(f, h));
    let exp_square: Vec<U256> = fp12_as_stack(mul_fp12(f, f));

    assert_eq!(out_normal, exp_normal);
    assert_eq!(out_sparse, exp_sparse);
    assert_eq!(out_square, exp_square);

    Ok(())
}

#[test]
fn test_frob_fp12() -> Result<()> {
    let ptr = U256::from(100);
    let f: Fp12 = gen_fp12();

    let test_frob1 = KERNEL.global_labels["test_frob_fp12_1"];
    let test_frob2 = KERNEL.global_labels["test_frob_fp12_2"];
    let test_frob3 = KERNEL.global_labels["test_frob_fp12_3"];
    let test_frob6 = KERNEL.global_labels["test_frob_fp12_6"];

    let mut stack = vec![ptr];
    stack.extend(fp12_to_vec(f));
    stack.extend(vec![ptr]);
    stack.reverse();

    let out_frob1: Vec<U256> = run_interpreter(test_frob1, stack.clone())?.stack().to_vec();
    let out_frob2: Vec<U256> = run_interpreter(test_frob2, stack.clone())?.stack().to_vec();
    let out_frob3: Vec<U256> = run_interpreter(test_frob3, stack.clone())?.stack().to_vec();
    let out_frob6: Vec<U256> = run_interpreter(test_frob6, stack)?.stack().to_vec();

    let exp_frob1: Vec<U256> = fp12_as_stack(frob_fp12(1, f));
    let exp_frob2: Vec<U256> = fp12_as_stack(frob_fp12(2, f));
    let exp_frob3: Vec<U256> = fp12_as_stack(frob_fp12(3, f));
    let exp_frob6: Vec<U256> = fp12_as_stack(frob_fp12(6, f));

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

    let test_inv = KERNEL.global_labels["test_inv_fp12"];

    let mut stack = vec![ptr];
    stack.extend(fp12_to_vec(f));
    stack.extend(vec![ptr, inv, U256::from_str("0xdeadbeef").unwrap()]);
    stack.reverse();

    let output: Vec<U256> = run_interpreter(test_inv, stack)?.stack().to_vec();

    assert_eq!(output, vec![]);

    Ok(())
}

#[test]
fn test_pow_fp12() -> Result<()> {
    let ptr = U256::from(300);
    let out = U256::from(400);

    let f: Fp12 = gen_fp12();

    let ret_stack = U256::from(KERNEL.global_labels["ret_stack"]);
    let test_pow = KERNEL.global_labels["test_pow"];

    let mut stack = vec![ptr];
    stack.extend(fp12_to_vec(f));
    stack.extend(vec![ptr, out, ret_stack, out]);
    stack.reverse();

    let output: Vec<U256> = run_interpreter(test_pow, stack)?.stack().to_vec();
    let expected: Vec<U256> = fp12_as_stack(power(f));

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_store_tangent() -> Result<()> {
    let p: Curve = gen_curve_point();
    let q: TwistedCurve = gen_twisted_curve_point();

    let p_: Vec<U256> = p.into_iter().collect();
    let q_: Vec<U256> = q.into_iter().flatten().collect();

    let test_tan = KERNEL.global_labels["test_store_tangent"];

    let mut stack = p_;
    stack.extend(q_);
    stack.reverse();

    let output: Vec<U256> = run_interpreter(test_tan, stack)?.stack().to_vec();

    let expected = fp12_as_stack(store_tangent(p, q));

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_store_cord() -> Result<()> {
    let p1: Curve = gen_curve_point();
    let p2: Curve = gen_curve_point();
    let q: TwistedCurve = gen_twisted_curve_point();

    let p1_: Vec<U256> = p1.into_iter().collect();
    let p2_: Vec<U256> = p2.into_iter().collect();
    let q_: Vec<U256> = q.into_iter().flatten().collect();

    let mut stack = p1_;
    stack.extend(p2_);
    stack.extend(q_);
    stack.reverse();

    let test_cord = KERNEL.global_labels["test_store_cord"];

    let output: Vec<U256> = run_interpreter(test_cord, stack)?.stack().to_vec();

    let expected = fp12_as_stack(store_cord(p1, p2, q));

    assert_eq!(output, expected);

    Ok(())
}

// fn make_miller_stack(p: [Fp; 2], q: [Fp2; 2]) -> Vec<U256> {
//     let ptr = U256::from(300);
//     let out = U256::from(400);

//     let p: Vec<U256> = p.into_iter().collect();
//     let q: Vec<U256> = q.into_iter().flatten().collect();

//     let ret_stack = U256::from(KERNEL.global_labels["ret_stack"]);

//     let mut input = vec![ptr];
//     input.extend(p);
//     input.extend(q);
//     input.extend(vec![ptr, out, ret_stack]);
//     input.reverse();
//     input
// }

// #[test]
// fn test_miller() -> Result<()> {
//     let p = [U256::from(1), U256::from(2)];
//     let q = [
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
//     ];

//     let test_mill = KERNEL.global_labels["test_miller"];
//     let stack = make_miller_stack(p, q);

//     let output: Vec<U256> = run_interpreter(test_mill, stack)?.stack().to_vec();
//     let mut expected: Vec<U256> = vec![
//         U256::from_str("0xbf4dbb7e41fb58122aa29dcced57731d7cbb49b1fe9a73cb13416e1002376da")
//             .unwrap(),
//         U256::from_str("0x110b019c149b43a7fbd6d42d7553debcbebd35c148f63aaecf72a5fbda451ac6")
//             .unwrap(),
//         U256::from_str("0x27225e97ee6c877964c8f32e0b54e61ead09c3e818174cd8b5beabe7cd7385e8")
//             .unwrap(),
//         U256::from_str("0x5762cb6648b4b4c5df8a8874a21d937adf185d91f34e8ccf58f5b39196db02").unwrap(),
//         U256::from_str("0x463002dc1a426b172f4a1e29486fc11eba01de99b559368139c8ef5271eb37f")
//             .unwrap(),
//         U256::from_str("0x753dcc72acdffcc45633803f1b555388969dd7c27d2a674a23a228f522480d9")
//             .unwrap(),
//         U256::from_str("0xd32a892d29151553101376a6638938135e30126f698a40a73f20c6ac64a4585")
//             .unwrap(),
//         U256::from_str("0x290afd3e28c223a624d9f5a737f9f9e4b4200b518333844d81acc445fa5910da")
//             .unwrap(),
//         U256::from_str("0x262e0ee72a8123b741dc113b8e2d207ee8bad011e0f6ae2015439960c789cf78")
//             .unwrap(),
//         U256::from_str("0x1588e0b23d868d7517e3021e620c69eb1521a49faa9bfcd4cf3a54127d4d14cb")
//             .unwrap(),
//         U256::from_str("0x1c23a135a7dfa96db62622c5fef4b9751d121523dd39ca1cefeacb3419835a53")
//             .unwrap(),
//         U256::from_str("0x2caeb873076ec8f37fa7af265d2966dd0024acbc63bd2b21f323084fc71f4a59")
//             .unwrap(),
//     ];
//     expected.reverse();

//     assert_eq!(output, expected);

//     Ok(())
// }
