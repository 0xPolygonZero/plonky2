use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, InterpreterMemoryInitialization,
};
use crate::extension_tower::{Fp2, Stack, BLS381};
use crate::memory::segments::Segment::KernelGeneral;

fn run_and_return_bls(label: String, x: BLS381, y: BLS381) -> BLS381 {
    let mut stack = x.to_stack();
    stack.extend(y.to_stack());
    let setup = InterpreterMemoryInitialization {
        label,
        stack,
        segment: KernelGeneral,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output = interpreter.stack();
    BLS381::from_stack(output)
}

#[test]
fn test_bls_ops() -> Result<()> {
    let mut rng = rand::thread_rng();
    let x: BLS381 = rng.gen::<BLS381>();
    let y: BLS381 = rng.gen::<BLS381>();

    let output_add = run_and_return_bls("test_add_fp381".to_string(), x, y);
    let output_mul = run_and_return_bls("test_mul_fp381".to_string(), x, y);
    let output_sub = run_and_return_bls("test_sub_fp381".to_string(), x, y);

    assert_eq!(output_add, x + y);
    assert_eq!(output_mul, x * y);
    assert_eq!(output_sub, x - y);

    Ok(())
}

fn run_and_return_bls_fp2(label: String, x: Fp2<BLS381>, y: Fp2<BLS381>) -> Fp2<BLS381> {
    let mut stack = x.to_stack();
    stack.extend(y.to_stack());
    stack.push(U256::from(0xdeadbeefu32));
    let setup = InterpreterMemoryInitialization {
        label,
        stack,
        segment: KernelGeneral,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output = interpreter.stack();
    Fp2::from_stack(output)
}

#[test]
fn test_bls_fp2() -> Result<()> {
    let mut rng = rand::thread_rng();
    let x: Fp2<BLS381> = rng.gen::<Fp2<BLS381>>();
    let y: Fp2<BLS381> = rng.gen::<Fp2<BLS381>>();

    let output_add = run_and_return_bls_fp2("add_fp381_2".to_string(), x, y);
    let output_mul = run_and_return_bls_fp2("mul_fp381_2".to_string(), x, y);
    let output_sub = run_and_return_bls_fp2("sub_fp381_2".to_string(), x, y);

    assert_eq!(output_add, x + y);
    assert_eq!(output_mul, x * y);
    assert_eq!(output_sub, x - y);

    Ok(())
}
