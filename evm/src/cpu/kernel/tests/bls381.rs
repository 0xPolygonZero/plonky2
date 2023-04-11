use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, InterpreterMemoryInitialization,
};
use crate::extension_tower::{Fp2, Stack, BLS381};
use crate::memory::segments::Segment::KernelGeneral;

fn run_bls_ops(label: &str, x: BLS381, y: BLS381) -> BLS381 {
    let mut stack = x.to_stack();
    stack.extend(y.to_stack());
    let setup = InterpreterMemoryInitialization {
        label: label.to_string(),
        stack,
        segment: KernelGeneral,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.stack().iter().rev().cloned().collect();
    BLS381::from_stack(&output)
}

#[test]
fn test_bls_ops() -> Result<()> {
    let mut rng = rand::thread_rng();
    let x: BLS381 = rng.gen::<BLS381>();
    let y: BLS381 = rng.gen::<BLS381>();

    let output_add = run_bls_ops("test_add_fp381", x, y);
    let output_sub = run_bls_ops("test_sub_fp381", x, y);
    let output_mul = run_bls_ops("test_mul_fp381", x, y);

    assert_eq!(output_add, x + y);
    assert_eq!(output_sub, x - y);
    assert_eq!(output_mul, x * y);

    Ok(())
}

fn run_bls_fp2_ops(label: &str, x: Fp2<BLS381>, y: Fp2<BLS381>) -> Fp2<BLS381> {
    let mut stack = x.to_stack();
    stack.extend(y.to_stack());
    stack.push(U256::from(0xdeadbeefu32));
    let setup = InterpreterMemoryInitialization {
        label: label.to_string(),
        stack,
        segment: KernelGeneral,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.stack().iter().rev().cloned().collect();
    Fp2::<BLS381>::from_stack(&output)
}

#[test]
fn test_bls_fp2_ops() -> Result<()> {
    let mut rng = rand::thread_rng();
    let x: Fp2<BLS381> = rng.gen::<Fp2<BLS381>>();
    let y: Fp2<BLS381> = rng.gen::<Fp2<BLS381>>();

    let output_add = run_bls_fp2_ops("test_add_fp381_2", x, y);
    let output_sub = run_bls_fp2_ops("test_sub_fp381_2", x, y);
    let output_mul = run_bls_fp2_ops("mul_fp381_2", x, y);

    assert_eq!(output_add, x + y);
    assert_eq!(output_sub, x - y);
    assert_eq!(output_mul, x * y);

    Ok(())
}
