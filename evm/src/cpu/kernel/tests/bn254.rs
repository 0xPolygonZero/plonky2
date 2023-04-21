use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::bn254_pairing::{
    final_exponent, gen_fp12_sparse, miller_loop, tate, Curve, TwistedCurve,
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

fn run_bn_mul_fp6(f: Fp6<BN254>, g: Fp6<BN254>, label: &str) -> Vec<U256> {
    let mut stack = f.on_stack();
    if label == "mul_fp254_6" {
        stack.extend(g.on_stack());
    }
    stack.push(U256::from(0xdeadbeefu32));

    let setup = InterpreterMemoryInitialization {
        label: label.to_string(),
        stack,
        segment: BnPairing,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    extract_stack(interpreter)
}

#[test]
fn test_bn_mul_fp6() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp6<BN254> = rng.gen::<Fp6<BN254>>();
    let g: Fp6<BN254> = rng.gen::<Fp6<BN254>>();

    let out_normal: Vec<U256> = run_bn_mul_fp6(f, g, "mul_fp254_6");
    let out_square: Vec<U256> = run_bn_mul_fp6(f, f, "square_fp254_6");

    let exp_normal: Vec<U256> = (f * g).on_stack();
    let exp_square: Vec<U256> = (f * f).on_stack();

    assert_eq!(out_normal, exp_normal);
    assert_eq!(out_square, exp_square);

    Ok(())
}

fn run_bn_mul_fp12(f: Fp12<BN254>, g: Fp12<BN254>, label: &str) -> Vec<U256> {
    let in0: usize = 100;
    let in1: usize = 112;
    let out: usize = 124;

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
        memory: vec![(in0, f.on_stack()), (in1, g.on_stack())],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    interpreter.extract_kernel_memory(BnPairing, out..out + 12)
}

#[test]
fn test_bn_mul_fp12() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();
    let g: Fp12<BN254> = rng.gen::<Fp12<BN254>>();
    let h: Fp12<BN254> = gen_fp12_sparse(&mut rng);

    let out_normal: Vec<U256> = run_bn_mul_fp12(f, g, "mul_fp254_12");
    let out_sparse: Vec<U256> = run_bn_mul_fp12(f, h, "mul_fp254_12_sparse");
    let out_square: Vec<U256> = run_bn_mul_fp12(f, f, "square_fp254_12");

    let exp_normal: Vec<U256> = (f * g).on_stack();
    let exp_sparse: Vec<U256> = (f * h).on_stack();
    let exp_square: Vec<U256> = (f * f).on_stack();

    assert_eq!(out_normal, exp_normal);
    assert_eq!(out_sparse, exp_sparse);
    assert_eq!(out_square, exp_square);

    Ok(())
}

fn run_bn_frob_fp6(f: Fp6<BN254>, n: usize) -> Vec<U256> {
    let setup = InterpreterMemoryInitialization {
        label: format!("test_frob_fp254_6_{}", n),
        stack: f.on_stack(),
        segment: BnPairing,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    extract_stack(interpreter)
}

#[test]
fn test_bn_frob_fp6() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp6<BN254> = rng.gen::<Fp6<BN254>>();
    for n in 1..4 {
        let output: Vec<U256> = run_bn_frob_fp6(f, n);
        let expected: Vec<U256> = f.frob(n).on_stack();
        assert_eq!(output, expected);
    }
    Ok(())
}

fn run_bn_frob_fp12(f: Fp12<BN254>, n: usize) -> Vec<U256> {
    let ptr: usize = 100;
    let setup = InterpreterMemoryInitialization {
        label: format!("test_frob_fp254_12_{}", n),
        stack: vec![U256::from(ptr)],
        segment: BnPairing,
        memory: vec![(ptr, f.on_stack())],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    interpreter.extract_kernel_memory(BnPairing, ptr..ptr + 12)
}

#[test]
fn test_bn_frob_fp12() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();

    for n in [1, 2, 3, 6] {
        let output = run_bn_frob_fp12(f, n);
        let expected: Vec<U256> = f.frob(n).on_stack();
        assert_eq!(output, expected);
    }
    Ok(())
}

#[test]
fn test_bn_inv_fp12() -> Result<()> {
    let ptr: usize = 100;
    let inv: usize = 112;
    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();

    let setup = InterpreterMemoryInitialization {
        label: "inv_fp254_12".to_string(),
        stack: vec![U256::from(ptr), U256::from(inv), U256::from(0xdeadbeefu32)],
        segment: BnPairing,
        memory: vec![(ptr, f.on_stack())],
    };
    let interpreter: Interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, inv..inv + 12);
    let expected: Vec<U256> = f.inv().on_stack();

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_bn_final_exponent() -> Result<()> {
    let ptr: usize = 100;

    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();

    let setup = InterpreterMemoryInitialization {
        label: "bn254_final_exponent".to_string(),
        stack: vec![
            U256::zero(),
            U256::zero(),
            U256::from(ptr),
            U256::from(0xdeadbeefu32),
        ],
        segment: BnPairing,
        memory: vec![(ptr, f.on_stack())],
    };

    let interpreter: Interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, ptr..ptr + 12);
    let expected: Vec<U256> = final_exponent(f).on_stack();

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
fn test_bn_miller() -> Result<()> {
    let ptr: usize = 100;
    let out: usize = 106;
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
    let expected = miller_loop(CURVE_GENERATOR, TWISTED_GENERATOR).on_stack();

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_bn_pairing() -> Result<()> {
    let out: usize = 100;
    let ptr: usize = 112;

    let inputs: Vec<U256> = vec![
        CURVE_GENERATOR.x.val,
        CURVE_GENERATOR.y.val,
        TWISTED_GENERATOR.x.re.val,
        TWISTED_GENERATOR.x.im.val,
        TWISTED_GENERATOR.y.re.val,
        TWISTED_GENERATOR.y.im.val,
    ];

    let setup = InterpreterMemoryInitialization {
        label: "bn254_pairing".to_string(),
        stack: vec![
            U256::one(),
            U256::from(ptr),
            U256::from(out),
            U256::from(0xdeadbeefu32),
        ],
        segment: BnPairing,
        memory: vec![(ptr, inputs)],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, out..out + 12);
    let expected = tate(CURVE_GENERATOR, TWISTED_GENERATOR).on_stack();

    assert_eq!(output, expected);

    Ok(())
}
