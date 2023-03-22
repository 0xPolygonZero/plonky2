use anyhow::Result;
use ethereum_types::{U256, U512};
use rand::Rng;

use crate::cpu::kernel::interpreter::{
    run_interpreter_with_memory, Interpreter, InterpreterMemoryInitialization,
};
use crate::extension_tower::{Stack, BLS381};
use crate::memory::segments::Segment::KernelGeneral;

fn run_and_return_bls(label: String, x: BLS381, y: BLS381) -> BLS381 {
    let mut stack = x.on_stack();
    stack.extend(y.on_stack());
    let setup = InterpreterMemoryInitialization {
        label,
        stack,
        segment: KernelGeneral,
        memory: vec![],
    };
    let interpreter = run_interpreter_with_memory(setup).unwrap();
    let output = interpreter.stack();
    BLS381 {
        val: U512::from(output[1]) + (U512::from(output[0]) << 256),
    }
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
