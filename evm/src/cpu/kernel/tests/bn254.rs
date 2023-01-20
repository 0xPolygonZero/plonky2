use std::ops::Range;

use anyhow::Result;
use ethereum_types::U256;

use crate::bn254_arithmetic::{fp12_to_vec, frob_fp12, gen_fp12, gen_fp12_sparse, inv_fp12, Fp12};
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment;
use crate::witness::memory::MemoryAddress;

struct InterpreterSetup {
    offset: String,
    stack: Vec<U256>,
    memory: Vec<(usize, Vec<U256>)>,
    output: Range<usize>,
}

fn get_interpreter_output(setup: InterpreterSetup) -> Result<Vec<U256>> {
    let label = KERNEL.global_labels[&setup.offset];
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

    let kernel = &interpreter.generation_state.memory.contexts[interpreter.context].segments
        [Segment::KernelGeneral as usize]
        .content;

    let mut output: Vec<U256> = vec![];
    for i in setup.output {
        output.push(kernel[i]);
    }
    Ok(output)
}

fn setup_mul_test(f: Fp12, g: Fp12, label: &str) -> InterpreterSetup {
    let in0: usize = 64;
    let in1: usize = 76;
    let out: usize = 88;

    let stack = vec![
        U256::from(in0),
        U256::from(in1),
        U256::from(out),
        U256::from(0xdeadbeefu32),
    ];
    let memory = vec![(in0, fp12_to_vec(f)), (in1, fp12_to_vec(g))];

    InterpreterSetup {
        offset: label.to_string(),
        stack: stack,
        memory: memory,
        output: out..out+12,
    }
}

#[test]
fn test_mul_fp12() -> Result<()> {
    let f: Fp12 = gen_fp12();
    let g: Fp12 = gen_fp12();
    let h: Fp12 = gen_fp12_sparse();

    let setup_normal: InterpreterSetup = setup_mul_test(f, g, "mul_fp12");
    let setup_sparse: InterpreterSetup = setup_mul_test(f, h, "mul_fp12_sparse");
    let setup_square: InterpreterSetup = setup_mul_test(f, f, "square_fp12_test");

    let out_normal: Vec<U256> = get_interpreter_output(setup_normal).unwrap();
    let out_sparse: Vec<U256> = get_interpreter_output(setup_sparse).unwrap();
    let out_square: Vec<U256> = get_interpreter_output(setup_square).unwrap();

    let exp_normal: Vec<U256> = fp12_to_vec(f * g);
    let exp_sparse: Vec<U256> = fp12_to_vec(f * h);
    let exp_square: Vec<U256> = fp12_to_vec(f * f);

    assert_eq!(out_normal, exp_normal);
    assert_eq!(out_sparse, exp_sparse);
    assert_eq!(out_square, exp_square);

    Ok(())
}

fn setup_frob_test(f: Fp12, label: &str) -> InterpreterSetup {
    let ptr: usize = 100;
    let stack = vec![U256::from(ptr)];
    let memory = vec![(ptr, fp12_to_vec(f))];

    InterpreterSetup {
        offset: label.to_string(),
        stack: stack,
        memory: memory,
        output: ptr..ptr+12,
    }
}

#[test]
fn test_frob_fp12() -> Result<()> {
    let f: Fp12 = gen_fp12();

    let setup_frob_1 = setup_frob_test(f, "test_frob_fp12_1");
    let setup_frob_2 = setup_frob_test(f, "test_frob_fp12_2");
    let setup_frob_3 = setup_frob_test(f, "test_frob_fp12_3");
    let setup_frob_6 = setup_frob_test(f, "test_frob_fp12_6");

    let out_frob_1: Vec<U256> = get_interpreter_output(setup_frob_1).unwrap();
    let out_frob_2: Vec<U256> = get_interpreter_output(setup_frob_2).unwrap();
    let out_frob_3: Vec<U256> = get_interpreter_output(setup_frob_3).unwrap();
    let out_frob_6: Vec<U256> = get_interpreter_output(setup_frob_6).unwrap();

    let exp_frob_1: Vec<U256> = fp12_to_vec(frob_fp12(1, f));
    let exp_frob_2: Vec<U256> = fp12_to_vec(frob_fp12(2, f));
    let exp_frob_3: Vec<U256> = fp12_to_vec(frob_fp12(3, f));
    let exp_frob_6: Vec<U256> = fp12_to_vec(frob_fp12(6, f));

    assert_eq!(out_frob_1, exp_frob_1);
    assert_eq!(out_frob_2, exp_frob_2);
    assert_eq!(out_frob_3, exp_frob_3);
    assert_eq!(out_frob_6, exp_frob_6);

    Ok(())
}

fn setup_inv_test(f: Fp12) -> InterpreterSetup {
    let ptr: usize = 100;
    let inv: usize = 112;
    let stack = vec![U256::from(ptr), U256::from(inv), U256::from(0xdeadbeefu32)];
    let memory = vec![(ptr, fp12_to_vec(f))];

    InterpreterSetup {
        offset: "inv_fp12".to_string(),
        stack: stack,
        memory: memory,
        output: inv..inv+12,
    }
}

#[test]
fn test_inv_fp12() -> Result<()> {
    let f: Fp12 = gen_fp12();

    let setup = setup_inv_test(f);
    let output: Vec<U256> = get_interpreter_output(setup).unwrap();
    let expected: Vec<U256> = fp12_to_vec(inv_fp12(f));

    assert_eq!(output, expected);

    Ok(())
}

// #[test]
// fn test_power() -> Result<()> {
//     let ptr = U256::from(300);
//     let out = U256::from(400);

//     let f: Fp12 = gen_fp12();

//     let mut stack = vec![ptr];
//     stack.extend(fp12_to_vec(f));
//     stack.extend(vec![
//         ptr,
//         out,
//         get_address_from_label("return_fp12_on_stack"),
//         out,
//     ]);

//     let output: Vec<U256> = get_interpreter_output("test_pow", stack);
//     let expected: Vec<U256> = fp12_to_vec(power(f));

//     assert_eq!(output, expected);

//     Ok(())
// }

// fn make_tate_stack(p: Curve, q: TwistedCurve) -> Vec<U256> {
//     let ptr = U256::from(300);
//     let out = U256::from(400);

//     let p_: Vec<U256> = p.into_iter().collect();
//     let q_: Vec<U256> = q.into_iter().flatten().collect();

//     let mut stack = vec![ptr];
//     stack.extend(p_);
//     stack.extend(q_);
//     stack.extend(vec![
//         ptr,
//         out,
//         get_address_from_label("return_fp12_on_stack"),
//         out,
//     ]);
//     stack
// }

// #[test]
// fn test_miller() -> Result<()> {
//     let p: Curve = curve_generator();
//     let q: TwistedCurve = twisted_curve_generator();

//     let stack = make_tate_stack(p, q);
//     let output = get_interpreter_output("test_miller", stack);
//     let expected = fp12_to_vec(miller_loop(p, q));

//     assert_eq!(output, expected);

//     Ok(())
// }

// #[test]
// fn test_tate() -> Result<()> {
//     let p: Curve = curve_generator();
//     let q: TwistedCurve = twisted_curve_generator();

//     let stack = make_tate_stack(p, q);
//     let output = get_interpreter_output("test_tate", stack);
//     let expected = fp12_to_vec(tate(p, q));

//     assert_eq!(output, expected);

//     Ok(())
// }
