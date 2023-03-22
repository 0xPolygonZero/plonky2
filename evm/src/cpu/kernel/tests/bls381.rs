use anyhow::Result;
use ethereum_types::{U256, U512};
use rand::Rng;

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, Interpreter, InterpreterMemoryInitialization,
};
use crate::extension_tower::{Stack, BLS381};
use crate::memory::segments::Segment::KernelGeneral;

fn extract_stack(interpreter: Interpreter<'static>) -> Vec<U256> {
    interpreter
        .stack()
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<U256>>()
}

fn combine_u256s(hi: U256, lo: U256) -> U512 {
    U512::from(lo) + (U512::from(hi) << 256)
}

#[test]
fn test_bls_ops() -> Result<()> {
    let mut rng = rand::thread_rng();
    let x: BLS381 = rng.gen::<BLS381>();
    let y: BLS381 = rng.gen::<BLS381>();

    let mut stack = x.on_stack();
    stack.extend(y.on_stack());

    let setup = InterpreterMemoryInitialization {
        label: "test_mul_fp381".to_string(),
        stack,
        segment: KernelGeneral,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output = extract_stack(interpreter);
    println!("{:#?}", output);
    let output_512 = combine_u256s(output[1], output[0]);
    let expected = x * y;

    assert_eq!(expected.val, output_512);

    Ok(())
}
