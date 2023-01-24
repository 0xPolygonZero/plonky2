use std::mem::transmute;
use std::ops::Range;

use anyhow::Result;
use ethereum_types::U256;

use crate::bn254_arithmetic::{gen_fp12, Fp12};
use crate::bn254_pairing::{gen_fp12_sparse, tate, CURVE_GENERATOR, TWISTED_GENERATOR};
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

fn fp12_on_stack(f: Fp12) -> Vec<U256> {
    let f: [U256; 12] = unsafe { transmute(f) };
    f.into_iter().collect()
}

fn setup_mul_test(
    in0: usize,
    in1: usize,
    out: usize,
    f: Fp12,
    g: Fp12,
    label: &str,
) -> InterpreterSetup {
    InterpreterSetup {
        label: label.to_string(),
        stack: vec![
            U256::from(in0),
            U256::from(in1),
            U256::from(out),
            U256::from(0xdeadbeefu32),
        ],
        memory: vec![(in0, fp12_on_stack(f)), (in1, fp12_on_stack(g))],
    }
}

#[test]
fn test_mul_fp12() -> Result<()> {
    let in0: usize = 64;
    let in1: usize = 76;
    let out: usize = 88;

    let f: Fp12 = gen_fp12();
    let g: Fp12 = gen_fp12();
    let h: Fp12 = gen_fp12_sparse();

    let setup_normal: InterpreterSetup = setup_mul_test(in0, in1, out, f, g, "mul_fp12");
    let setup_sparse: InterpreterSetup = setup_mul_test(in0, in1, out, f, h, "mul_fp12_sparse");
    let setup_square: InterpreterSetup = setup_mul_test(in0, in1, out, f, f, "square_fp12_test");

    let intrptr_normal: Interpreter = run_setup_interpreter(setup_normal).unwrap();
    let intrptr_sparse: Interpreter = run_setup_interpreter(setup_sparse).unwrap();
    let intrptr_square: Interpreter = run_setup_interpreter(setup_square).unwrap();

    let out_normal: Vec<U256> = extract_kernel_output(out..out + 12, intrptr_normal);
    let out_sparse: Vec<U256> = extract_kernel_output(out..out + 12, intrptr_sparse);
    let out_square: Vec<U256> = extract_kernel_output(out..out + 12, intrptr_square);

    let exp_normal: Vec<U256> = fp12_on_stack(f * g);
    let exp_sparse: Vec<U256> = fp12_on_stack(f * h);
    let exp_square: Vec<U256> = fp12_on_stack(f * f);

    assert_eq!(out_normal, exp_normal);
    assert_eq!(out_sparse, exp_sparse);
    assert_eq!(out_square, exp_square);

    Ok(())
}

fn setup_frob_test(ptr: usize, f: Fp12, label: &str) -> InterpreterSetup {
    InterpreterSetup {
        label: label.to_string(),
        stack: vec![U256::from(ptr)],
        memory: vec![(ptr, fp12_on_stack(f))],
    }
}

#[test]
fn test_frob_fp12() -> Result<()> {
    let ptr: usize = 100;
    let f: Fp12 = gen_fp12();

    let setup_frob_1 = setup_frob_test(ptr, f, "test_frob_fp12_1");
    let setup_frob_2 = setup_frob_test(ptr, f, "test_frob_fp12_2");
    let setup_frob_3 = setup_frob_test(ptr, f, "test_frob_fp12_3");
    let setup_frob_6 = setup_frob_test(ptr, f, "test_frob_fp12_6");

    let intrptr_frob_1: Interpreter = run_setup_interpreter(setup_frob_1).unwrap();
    let intrptr_frob_2: Interpreter = run_setup_interpreter(setup_frob_2).unwrap();
    let intrptr_frob_3: Interpreter = run_setup_interpreter(setup_frob_3).unwrap();
    let intrptr_frob_6: Interpreter = run_setup_interpreter(setup_frob_6).unwrap();

    let out_frob_1: Vec<U256> = extract_kernel_output(ptr..ptr + 12, intrptr_frob_1);
    let out_frob_2: Vec<U256> = extract_kernel_output(ptr..ptr + 12, intrptr_frob_2);
    let out_frob_3: Vec<U256> = extract_kernel_output(ptr..ptr + 12, intrptr_frob_3);
    let out_frob_6: Vec<U256> = extract_kernel_output(ptr..ptr + 12, intrptr_frob_6);

    let exp_frob_1: Vec<U256> = fp12_on_stack(f.frob(1));
    let exp_frob_2: Vec<U256> = fp12_on_stack(f.frob(2));
    let exp_frob_3: Vec<U256> = fp12_on_stack(f.frob(3));
    let exp_frob_6: Vec<U256> = fp12_on_stack(f.frob(6));

    assert_eq!(out_frob_1, exp_frob_1);
    assert_eq!(out_frob_2, exp_frob_2);
    assert_eq!(out_frob_3, exp_frob_3);
    assert_eq!(out_frob_6, exp_frob_6);

    Ok(())
}

#[test]
fn test_inv_fp12() -> Result<()> {
    let ptr: usize = 100;
    let inv: usize = 112;
    let f: Fp12 = gen_fp12();

    let setup = InterpreterSetup {
        label: "inv_fp12".to_string(),
        stack: vec![U256::from(ptr), U256::from(inv), U256::from(0xdeadbeefu32)],
        memory: vec![(ptr, fp12_on_stack(f))],
    };
    let interpreter: Interpreter = run_setup_interpreter(setup).unwrap();
    let output: Vec<U256> = extract_kernel_output(inv..inv + 12, interpreter);
    let expected: Vec<U256> = fp12_on_stack(f.inv());

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

// #[test]
// fn test_miller() -> Result<()> {
//     let p: Curve = curve_generator();
//     let q: TwistedCurve = twisted_curve_generator();

//     let stack = make_tate_stack(p, q);
//     let output = run_setup_interpreter("test_miller", stack);
//     let expected = fp12_on_stack(miller_loop(p, q));

//     assert_eq!(output, expected);

//     Ok(())
// }

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
        label: "tate".to_string(),
        stack: vec![U256::from(ptr), U256::from(out), U256::from(0xdeadbeefu32)],
        memory: vec![(ptr, inputs)],
    };
    let interpreter = run_setup_interpreter(setup).unwrap();
    let output: Vec<U256> = extract_kernel_output(out..out + 12, interpreter);
    let expected = fp12_on_stack(tate(CURVE_GENERATOR, TWISTED_GENERATOR));

    assert_eq!(output, expected);

    Ok(())
}
