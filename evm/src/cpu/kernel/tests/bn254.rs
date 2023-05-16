use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, Interpreter, InterpreterMemoryInitialization,
};
use crate::curve_pairings::{
    bn_final_exponent, bn_miller_loop, gen_bn_fp12_sparse, Curve, CyclicGroup,
};
use crate::extension_tower::{FieldExt, Fp12, Fp2, Fp6, Stack, BN254};
use crate::memory::segments::Segment::BnPairing;

fn run_bn_mul_fp6(f: Fp6<BN254>, g: Fp6<BN254>, label: &str) -> Fp6<BN254> {
    let mut stack = f.to_stack();
    if label == "mul_fp254_6" {
        stack.extend(g.to_stack().to_vec());
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
fn test_bn_mul_fp6() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp6<BN254> = rng.gen::<Fp6<BN254>>();
    let g: Fp6<BN254> = rng.gen::<Fp6<BN254>>();

    let output_normal: Fp6<BN254> = run_bn_mul_fp6(f, g, "mul_fp254_6");
    let output_square: Fp6<BN254> = run_bn_mul_fp6(f, f, "square_fp254_6");

    assert_eq!(output_normal, f * g);
    assert_eq!(output_square, f * f);

    Ok(())
}

fn run_bn_mul_fp12(f: Fp12<BN254>, g: Fp12<BN254>, label: &str) -> Fp12<BN254> {
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
        memory: vec![(in0, f.to_stack().to_vec()), (in1, g.to_stack().to_vec())],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output = interpreter.extract_kernel_memory(BnPairing, out..out + 12);
    Fp12::<BN254>::from_stack(&output)
}

#[test]
fn test_bn_mul_fp12() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();
    let g: Fp12<BN254> = rng.gen::<Fp12<BN254>>();
    let h: Fp12<BN254> = gen_bn_fp12_sparse(&mut rng);

    let output_normal = run_bn_mul_fp12(f, g, "mul_fp254_12");
    let output_sparse = run_bn_mul_fp12(f, h, "mul_fp254_12_sparse");
    let output_square = run_bn_mul_fp12(f, f, "square_fp254_12");

    assert_eq!(output_normal, f * g);
    assert_eq!(output_sparse, f * h);
    assert_eq!(output_square, f * f);

    Ok(())
}

fn run_bn_frob_fp6(n: usize, f: Fp6<BN254>) -> Fp6<BN254> {
    let setup = InterpreterMemoryInitialization {
        label: format!("test_frob_fp254_6_{}", n),
        stack: f.to_stack().to_vec(),
        segment: BnPairing,
        memory: vec![],
    };
    let interpreter: Interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.stack().iter().rev().cloned().collect();
    Fp6::<BN254>::from_stack(&output)
}

#[test]
fn test_bn_frob_fp6() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp6<BN254> = rng.gen::<Fp6<BN254>>();
    for n in 1..4 {
        let output = run_bn_frob_fp6(n, f);
        assert_eq!(output, f.frob(n));
    }
    Ok(())
}

fn run_bn_frob_fp12(f: Fp12<BN254>, n: usize) -> Fp12<BN254> {
    let ptr: usize = 100;
    let setup = InterpreterMemoryInitialization {
        label: format!("test_frob_fp254_12_{}", n),
        stack: vec![U256::from(ptr)],
        segment: BnPairing,
        memory: vec![(ptr, f.to_stack().to_vec())],
    };
    let interpreter: Interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, ptr..ptr + 12);
    Fp12::<BN254>::from_stack(&output)
}

#[test]
fn test_frob_fp12() -> Result<()> {
    let mut rng = rand::thread_rng();
    let f: Fp12<BN254> = rng.gen::<Fp12<BN254>>();

    for n in [1, 2, 3, 6] {
        let output = run_bn_frob_fp12(f, n);
        assert_eq!(output, f.frob(n));
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
        memory: vec![(ptr, f.to_stack().to_vec())],
    };
    let interpreter: Interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, inv..inv + 12);
    let output = Fp12::<BN254>::from_stack(&output);

    assert_eq!(output, f.inv());

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
        memory: vec![(ptr, f.to_stack().to_vec())],
    };

    let interpreter: Interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, ptr..ptr + 12);
    let expected: Vec<U256> = bn_final_exponent(f).to_stack();

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_bn_miller() -> Result<()> {
    let ptr: usize = 100;
    let out: usize = 106;

    let mut rng = rand::thread_rng();
    let p: Curve<BN254> = rng.gen::<Curve<BN254>>();
    let q: Curve<Fp2<BN254>> = rng.gen::<Curve<Fp2<BN254>>>();

    let mut input = p.to_stack();
    input.extend(q.to_stack());

    let setup = InterpreterMemoryInitialization {
        label: "bn254_miller".to_string(),
        stack: vec![U256::from(ptr), U256::from(out), U256::from(0xdeadbeefu32)],
        segment: BnPairing,
        memory: vec![(ptr, input)],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(BnPairing, out..out + 12);
    let expected = bn_miller_loop(p, q).to_stack();

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_bn_pairing() -> Result<()> {
    let out: usize = 100;
    let ptr: usize = 112;

    let mut rng = rand::thread_rng();
    let k: usize = rng.gen_range(1..10);
    let mut acc: i32 = 0;
    let mut input: Vec<U256> = vec![];
    for _ in 1..k {
        let m: i32 = rng.gen_range(-8..8);
        let n: i32 = rng.gen_range(-8..8);
        acc -= m * n;

        let p: Curve<BN254> = Curve::<BN254>::int(m);
        let q: Curve<Fp2<BN254>> = Curve::<Fp2<BN254>>::int(n);
        input.extend(p.to_stack());
        input.extend(q.to_stack());
    }
    let p: Curve<BN254> = Curve::<BN254>::int(acc);
    let q: Curve<Fp2<BN254>> = Curve::<Fp2<BN254>>::GENERATOR;
    input.extend(p.to_stack());
    input.extend(q.to_stack());

    let setup = InterpreterMemoryInitialization {
        label: "bn254_pairing".to_string(),
        stack: vec![
            U256::from(k),
            U256::from(ptr),
            U256::from(out),
            U256::from(0xdeadbeefu32),
        ],
        segment: BnPairing,
        memory: vec![(ptr, input)],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    assert_eq!(interpreter.stack()[0], U256::one());
    Ok(())
}
