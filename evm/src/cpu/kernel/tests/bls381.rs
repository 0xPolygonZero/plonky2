use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, InterpreterMemoryInitialization,
};
use crate::extension_tower::{Fp2, Fp6, Stack, BLS381};
use crate::memory::segments::Segment::KernelGeneral;

#[test]
fn test_bls_fp2_mul() -> Result<()> {
    let mut rng = rand::thread_rng();
    let x: Fp2<BLS381> = rng.gen::<Fp2<BLS381>>();
    let y: Fp2<BLS381> = rng.gen::<Fp2<BLS381>>();

    let mut stack = x.to_stack().to_vec();
    stack.extend(y.to_stack().to_vec());
    stack.push(U256::from(0xdeadbeefu32));
    let setup = InterpreterMemoryInitialization {
        label: "mul_fp381_2".to_string(),
        stack,
        segment: KernelGeneral,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let stack: Vec<U256> = interpreter.stack().iter().rev().cloned().collect();
    let output = Fp2::<BLS381>::from_stack(&stack);

    assert_eq!(output, x * y);
    Ok(())
}

fn run_bls_fp6_ops(label: &str, x: Fp6<BLS381>, y: Fp6<BLS381>) -> Fp6<BLS381> {
    let in0 = 0;
    let in1 = 12;
    let out = 24;

    let stack = vec![
        U256::from(in0),
        U256::from(in1),
        U256::from(out),
        U256::from(0xdeadbeefu32),
    ];
    let setup = InterpreterMemoryInitialization {
        label: label.to_string(),
        stack,
        segment: KernelGeneral,
        memory: vec![(in0, x.to_stack()), (in1, y.to_stack())],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output: Vec<U256> = interpreter.extract_kernel_memory(KernelGeneral, out..out + 12);
    Fp6::<BLS381>::from_stack(&output)
}

#[test]
fn test_bls_fp6_ops() -> Result<()> {
    let mut rng = rand::thread_rng();
    let x: Fp6<BLS381> = rng.gen::<Fp6<BLS381>>();
    let y: Fp6<BLS381> = rng.gen::<Fp6<BLS381>>();

    let output_add = run_bls_fp6_ops("add_fp381_6", x, y);
    let output_sub = run_bls_fp6_ops("sub_fp381_6", x, y);
    let output_mul = run_bls_fp6_ops("mul_fp381_6", x, y);

    assert_eq!(output_add, x + y);
    assert_eq!(output_sub, x - y);
    assert_eq!(output_mul, x * y);

    Ok(())
}
