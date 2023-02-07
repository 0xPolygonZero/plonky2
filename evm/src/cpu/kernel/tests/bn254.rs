use std::ops::Range;

use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::bn254_arithmetic::{Fp, Fp12, Fp2};
use crate::bn254_pairing::{gen_fp12_sparse, miller_loop, tate, Curve, TwistedCurve};
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment;
use crate::witness::memory::MemoryAddress;

struct InterpreterSetup {
    label: String,
    stack: Vec<U256>,
    memory: Vec<(usize, Vec<U256>)>,
}

fn run_setup_interpreter(setup: InterpreterSetup) -> Result<Interpreter<'static>> {
    let label = KERNEL.global_labels[&setup.label];
    let mut stack = setup.stack;
    stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(label, stack);
    for (pointer, data) in setup.memory {
        for (i, term) in data.iter().enumerate() {
            interpreter.generation_state.memory.set(
                MemoryAddress::new(0, Segment::KernelGeneral, pointer + i),
                *term,
            )
        }
    }
    interpreter.run()?;
    Ok(interpreter)
}

fn extract_kernel_output(range: Range<usize>, interpreter: Interpreter<'static>) -> Vec<U256> {
    let mut output: Vec<U256> = vec![];
    for i in range {
        let term = interpreter.generation_state.memory.get(MemoryAddress::new(
            0,
            Segment::KernelGeneral,
            i,
        ));
        output.push(term);
    }
    output
}

fn setup_mul_test(out: usize, f: Fp12, g: Fp12, label: &str) -> InterpreterSetup {
    let in0: usize = 64;
    let in1: usize = 76;
    InterpreterSetup {
        label: label.to_string(),
        stack: vec![
            U256::from(in0),
            U256::from(in1),
            U256::from(out),
            U256::from(0xdeadbeefu32),
        ],
        memory: vec![(in0, f.on_stack()), (in1, g.on_stack())],
    }
}

#[test]
fn test_mul_fp254_12() -> Result<()> {
    let out: usize = 88;

    let mut rng = rand::thread_rng();
    let f: Fp12 = rng.gen::<Fp12>();
    let g: Fp12 = rng.gen::<Fp12>();
    let h: Fp12 = gen_fp12_sparse(&mut rng);

    let setup_normal: InterpreterSetup = setup_mul_test(out, f, g, "mul_fp254_12");
    let setup_sparse: InterpreterSetup = setup_mul_test(out, f, h, "mul_fp254_12_sparse");
    let setup_square: InterpreterSetup = setup_mul_test(out, f, f, "square_fp254_12_test");

    let intrptr_normal: Interpreter = run_setup_interpreter(setup_normal).unwrap();
    let intrptr_sparse: Interpreter = run_setup_interpreter(setup_sparse).unwrap();
    let intrptr_square: Interpreter = run_setup_interpreter(setup_square).unwrap();

    let out_normal: Vec<U256> = extract_kernel_output(out..out + 12, intrptr_normal);
    let out_sparse: Vec<U256> = extract_kernel_output(out..out + 12, intrptr_sparse);
    let out_square: Vec<U256> = extract_kernel_output(out..out + 12, intrptr_square);

    let exp_normal: Vec<U256> = (f * g).on_stack();
    let exp_sparse: Vec<U256> = (f * h).on_stack();
    let exp_square: Vec<U256> = (f * f).on_stack();

    assert_eq!(out_normal, exp_normal);
    assert_eq!(out_sparse, exp_sparse);
    assert_eq!(out_square, exp_square);

    Ok(())
}

fn setup_frob_test(ptr: usize, f: Fp12, label: &str) -> InterpreterSetup {
    InterpreterSetup {
        label: label.to_string(),
        stack: vec![U256::from(ptr)],
        memory: vec![(ptr, f.on_stack())],
    }
}

#[test]
fn test_frob_fp254_12() -> Result<()> {
    let ptr: usize = 100;

    let mut rng = rand::thread_rng();
    let f: Fp12 = rng.gen::<Fp12>();

    let setup_frob_1 = setup_frob_test(ptr, f, "test_frob_fp254_12_1");
    let setup_frob_2 = setup_frob_test(ptr, f, "test_frob_fp254_12_2");
    let setup_frob_3 = setup_frob_test(ptr, f, "test_frob_fp254_12_3");
    let setup_frob_6 = setup_frob_test(ptr, f, "test_frob_fp254_12_6");

    let intrptr_frob_1: Interpreter = run_setup_interpreter(setup_frob_1).unwrap();
    let intrptr_frob_2: Interpreter = run_setup_interpreter(setup_frob_2).unwrap();
    let intrptr_frob_3: Interpreter = run_setup_interpreter(setup_frob_3).unwrap();
    let intrptr_frob_6: Interpreter = run_setup_interpreter(setup_frob_6).unwrap();

    let out_frob_1: Vec<U256> = extract_kernel_output(ptr..ptr + 12, intrptr_frob_1);
    let out_frob_2: Vec<U256> = extract_kernel_output(ptr..ptr + 12, intrptr_frob_2);
    let out_frob_3: Vec<U256> = extract_kernel_output(ptr..ptr + 12, intrptr_frob_3);
    let out_frob_6: Vec<U256> = extract_kernel_output(ptr..ptr + 12, intrptr_frob_6);

    let exp_frob_1: Vec<U256> = f.frob(1).on_stack();
    let exp_frob_2: Vec<U256> = f.frob(2).on_stack();
    let exp_frob_3: Vec<U256> = f.frob(3).on_stack();
    let exp_frob_6: Vec<U256> = f.frob(6).on_stack();

    assert_eq!(out_frob_1, exp_frob_1);
    assert_eq!(out_frob_2, exp_frob_2);
    assert_eq!(out_frob_3, exp_frob_3);
    assert_eq!(out_frob_6, exp_frob_6);

    Ok(())
}

#[test]
fn test_inv_fp254_12() -> Result<()> {
    let ptr: usize = 100;
    let inv: usize = 112;

    let mut rng = rand::thread_rng();
    let f: Fp12 = rng.gen::<Fp12>();

    let setup = InterpreterSetup {
        label: "inv_fp254_12".to_string(),
        stack: vec![U256::from(ptr), U256::from(inv), U256::from(0xdeadbeefu32)],
        memory: vec![(ptr, f.on_stack())],
    };
    let interpreter: Interpreter = run_setup_interpreter(setup).unwrap();
    let output: Vec<U256> = extract_kernel_output(inv..inv + 12, interpreter);
    let expected: Vec<U256> = f.inv().on_stack();

    assert_eq!(output, expected);

    Ok(())
}

// #[test]
// fn test_invariance_inducing_power() -> Result<()> {
//     let ptr = U256::from(300);
//     let out = U256::from(400);

//     let f: Fp12 = gen_fp12();

//     let mut stack = vec![ptr];
//     stack.extend(fp12_on_stack(f));
//     stack.extend(vec![
//         ptr,
//         out,
//         get_address_from_label("return_fp12_on_stack"),
//         out,
//     ]);

//     let output: Vec<U256> = run_setup_interpreter("test_pow", stack);
//     let expected: Vec<U256> = fp12_on_stack(invariance_inducing_power(f));

//     assert_eq!(output, expected);

//     Ok(())
// }

// The curve is cyclic with generator (1, 2)
pub const CURVE_GENERATOR: Curve = {
    Curve {
        x: Fp { val: U256::one() },
        y: Fp {
            val: U256([2, 0, 0, 0]),
        },
    }
};

// The twisted curve is cyclic with generator (x, y) as follows
pub const TWISTED_GENERATOR: TwistedCurve = {
    TwistedCurve {
        x: Fp2 {
            re: Fp {
                val: U256([
                    0x46debd5cd992f6ed,
                    0x674322d4f75edadd,
                    0x426a00665e5c4479,
                    0x1800deef121f1e76,
                ]),
            },
            im: Fp {
                val: U256([
                    0x97e485b7aef312c2,
                    0xf1aa493335a9e712,
                    0x7260bfb731fb5d25,
                    0x198e9393920d483a,
                ]),
            },
        },
        y: Fp2 {
            re: Fp {
                val: U256([
                    0x4ce6cc0166fa7daa,
                    0xe3d1e7690c43d37b,
                    0x4aab71808dcb408f,
                    0x12c85ea5db8c6deb,
                ]),
            },
            im: Fp {
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
    let ptr: usize = 300;
    let out: usize = 400;
    let inputs: Vec<U256> = vec![
        CURVE_GENERATOR.x.val,
        CURVE_GENERATOR.y.val,
        TWISTED_GENERATOR.x.re.val,
        TWISTED_GENERATOR.x.im.val,
        TWISTED_GENERATOR.y.re.val,
        TWISTED_GENERATOR.y.im.val,
    ];

    let setup = InterpreterSetup {
        label: "bn254_miller".to_string(),
        stack: vec![U256::from(ptr), U256::from(out), U256::from(0xdeadbeefu32)],
        memory: vec![(ptr, inputs)],
    };
    let interpreter = run_setup_interpreter(setup).unwrap();
    let output: Vec<U256> = extract_kernel_output(out..out + 12, interpreter);
    let expected = miller_loop(CURVE_GENERATOR, TWISTED_GENERATOR).on_stack();

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn test_tate() -> Result<()> {
    let ptr: usize = 300;
    let out: usize = 400;
    let inputs: Vec<U256> = vec![
        CURVE_GENERATOR.x.val,
        CURVE_GENERATOR.y.val,
        TWISTED_GENERATOR.x.re.val,
        TWISTED_GENERATOR.x.im.val,
        TWISTED_GENERATOR.y.re.val,
        TWISTED_GENERATOR.y.im.val,
    ];

    let setup = InterpreterSetup {
        label: "bn254_tate".to_string(),
        stack: vec![U256::from(ptr), U256::from(out), U256::from(0xdeadbeefu32)],
        memory: vec![(ptr, inputs)],
    };
    let interpreter = run_setup_interpreter(setup).unwrap();
    let output: Vec<U256> = extract_kernel_output(out..out + 12, interpreter);
    let expected = tate(CURVE_GENERATOR, TWISTED_GENERATOR).on_stack();

    assert_eq!(output, expected);

    Ok(())
}
