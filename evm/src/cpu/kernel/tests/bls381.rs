use anyhow::Result;
use ethereum_types::U256;
use rand::Rng;

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, InterpreterMemoryInitialization,
};
use crate::extension_tower::{Fp2, Stack, BLS381};
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
