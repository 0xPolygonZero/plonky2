use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::bn254_pairing::{
    gen_fp12_sparse, invariant_exponent, miller_loop, tate, Curve, TwistedCurve,
};
use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, Interpreter, InterpreterMemoryInitialization,
};
use crate::extension_tower::{FieldExt, Fp12, Fp2, Fp6, Stack, BN254};
use crate::memory::segments::Segment::BnPairing;

fn extract_stack(interpreter: Interpreter<'static>) -> Vec<U256> {
    interpreter
        .stack()
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<U256>>()
}

fn run_and_return_bn_fp6(f: Fp6<BN254>, g: Fp6<BN254>, label: &str) -> Fp6<BN254> {
    let mut stack = f.to_stack();
    if label == "mul_fp254_6" {
        stack.extend(g.to_stack());
    }
    stack.push(U256::from(0xdeadbeefu32));
    let setup = InterpreterMemoryInitialization {
        label: label.to_string(),
        stack,
        segment: BnPairing,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.stack().iter().rev().cloned().collect();
    Fp6::<BN254>::from_stack(&output)
}

#[test]
fn test_mul_fp6() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp6<BN254> = rng.gen::<Fp6<BN254>>();
    let g: Fp6<BN254> = rng.gen::<Fp6<BN254>>();

    let output_mul: Fp6<BN254> = run_and_return_bn_fp6(f, g, "mul_fp254_6");
    let output_square: Fp6<BN254> = run_and_return_bn_fp6(f, f, "square_fp254_6");

    assert_eq!(output_mul, f * g);
    assert_eq!(output_square, f * f);

    Ok(())
}

fn run_and_return_bn_fp12(
    f: Fp12<BN254>,
    g: Fp12<BN254>,
    label: &str,
) -> Fp12<BN254> {
    let in0: usize = 200;
    let in1: usize = 212;
    let out: usize = 224;

    let mut stack = vec![
        U256::from(in0),
        U256::from(in1),
        U256::from(out),
        U256::from(0xdeadbeefu32),
    ];
    if label == "square_fp254_12" {
        stack.remove(0);
    }
    let setup = InterpreterMemoryInitialization {
        label: label.to_string(),
        stack,
        segment: BnPairing,
        memory: vec![(in0, f.to_stack()), (in1, g.to_stack())],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output = interpreter.extract_kernel_memory(BnPairing, out..out + 12);
    Fp12::<BN254>::from_stack(&output)
}

#[test]
fn test_mul_fp12() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();
    let g: Fp12<BN254> = rng.gen::<Fp12<BN254>>();
    let h: Fp12<BN254> = gen_fp12_sparse(&mut rng);

    let output_normal = run_and_return_bn_fp12(f, g, "mul_fp254_12");
    let output_sparse = run_and_return_bn_fp12(f, h, "mul_fp254_12_sparse");
    let output_square = run_and_return_bn_fp12(f, f, "square_fp254_12");

    assert_eq!(output_normal, f * g);
    assert_eq!(output_sparse, f * h);
    assert_eq!(output_square, f * f);

    Ok(())
}

fn setup_frob_fp6_test(f: Fp6<BN254>, n: usize) -> InterpreterMemoryInitialization {
    InterpreterMemoryInitialization {
        label: String::from("test_frob_fp254_6_") + &(n.to_string()),
        stack: f.to_stack(),
        segment: BnPairing,
        memory: vec![],
    }
}

#[test]
fn test_frob_fp6() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp6<BN254> = rng.gen::<Fp6<BN254>>();
    for n in 1..4 {
        let setup_frob = setup_frob_fp6_test(f, n);
        let intrptr_frob: Interpreter = run_interpreter_with_memory(setup_frob).unwrap();
        let out_frob: Vec<U256> = extract_stack(intrptr_frob);
        let exp_frob: Vec<U256> = f.frob(n).to_stack();
        assert_eq!(out_frob, exp_frob);
    }
    Ok(())
}

fn setup_frob_fp12_test(ptr: usize, f: Fp12<BN254>, n: usize) -> InterpreterMemoryInitialization {
    InterpreterMemoryInitialization {
        label: String::from("test_frob_fp254_12_") + &(n.to_string()),
        stack: vec![U256::from(ptr)],
        segment: BnPairing,
        memory: vec![(ptr, f.to_stack())],
    }
}

#[test]
fn test_frob_fp12() -> Result<()> {
    let ptr: usize = 200;
    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();
    for n in [1, 2, 3, 6] {
        let setup_frob = setup_frob_fp12_test(ptr, f, n);
        let intrptr_frob: Interpreter = run_interpreter_with_memory(setup_frob).unwrap();
        let out_frob: Vec<U256> = intrptr_frob.extract_kernel_memory(BnPairing, ptr..ptr + 12);
        let exp_frob: Vec<U256> = f.frob(n).to_stack();
        assert_eq!(out_frob, exp_frob);
    }
    Ok(())
}

#[test]
fn test_inv_fp12() -> Result<()> {
    let ptr: usize = 200;
    let inv: usize = 212;
    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();

    let setup = InterpreterMemoryInitialization {
        label: "inv_fp254_12".to_string(),
        stack: vec![U256::from(ptr), U256::from(inv), U256::from(0xdeadbeefu32)],
        segment: BnPairing,
        memory: vec![(ptr, f.to_stack())],
    };
    let interpreter: Interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, inv..inv + 12);
    let expected: Vec<U256> = f.inv().to_stack();

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_invariant_exponent() -> Result<()> {
    let ptr: usize = 200;
    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();

    let setup = InterpreterMemoryInitialization {
        label: "bn254_invariant_exponent".to_string(),
        stack: vec![U256::from(ptr), U256::from(0xdeadbeefu32)],
        segment: BnPairing,
        memory: vec![(ptr, f.to_stack())],
    };

    let interpreter: Interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, ptr..ptr + 12);
    let expected: Vec<U256> = invariant_exponent(f).to_stack();

    assert_eq!(output, expected);

    Ok(())
}

// The curve is cyclic with generator (1, 2)
pub const CURVE_GENERATOR: Curve = {
    Curve {
        x: BN254 { val: U256::one() },
        y: BN254 {
            val: U256([2, 0, 0, 0]),
        },
    }
};

// The twisted curve is cyclic with generator (x, y) as follows
pub const TWISTED_GENERATOR: TwistedCurve = {
    TwistedCurve {
        x: Fp2 {
            re: BN254 {
                val: U256([
                    0x46debd5cd992f6ed,
                    0x674322d4f75edadd,
                    0x426a00665e5c4479,
                    0x1800deef121f1e76,
                ]),
            },
            im: BN254 {
                val: U256([
                    0x97e485b7aef312c2,
                    0xf1aa493335a9e712,
                    0x7260bfb731fb5d25,
                    0x198e9393920d483a,
                ]),
            },
        },
        y: Fp2 {
            re: BN254 {
                val: U256([
                    0x4ce6cc0166fa7daa,
                    0xe3d1e7690c43d37b,
                    0x4aab71808dcb408f,
                    0x12c85ea5db8c6deb,
                ]),
            },
            im: BN254 {
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

#[test]
fn test_miller() -> Result<()> {
    let ptr: usize = 200;
    let out: usize = 206;
    let inputs: Vec<U256> = vec![
        CURVE_GENERATOR.x.val,
        CURVE_GENERATOR.y.val,
        TWISTED_GENERATOR.x.re.val,
        TWISTED_GENERATOR.x.im.val,
        TWISTED_GENERATOR.y.re.val,
        TWISTED_GENERATOR.y.im.val,
    ];

    let setup = InterpreterMemoryInitialization {
        label: "bn254_miller".to_string(),
        stack: vec![U256::from(ptr), U256::from(out), U256::from(0xdeadbeefu32)],
        segment: BnPairing,
        memory: vec![(ptr, inputs)],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, out..out + 12);
    let expected = miller_loop(CURVE_GENERATOR, TWISTED_GENERATOR).to_stack();

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_tate() -> Result<()> {
    let ptr: usize = 200;
    let out: usize = 206;
    let inputs: Vec<U256> = vec![
        CURVE_GENERATOR.x.val,
        CURVE_GENERATOR.y.val,
        TWISTED_GENERATOR.x.re.val,
        TWISTED_GENERATOR.x.im.val,
        TWISTED_GENERATOR.y.re.val,
        TWISTED_GENERATOR.y.im.val,
    ];

    let setup = InterpreterMemoryInitialization {
        label: "bn254_tate".to_string(),
        stack: vec![U256::from(ptr), U256::from(out), U256::from(0xdeadbeefu32)],
        segment: BnPairing,
        memory: vec![(ptr, inputs)],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, out..out + 12);
    let expected = tate(CURVE_GENERATOR, TWISTED_GENERATOR).to_stack();

    assert_eq!(output, expected);

    Ok(())
}
