use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::{run, run_interpreter, Interpreter};

#[test]
fn test_exp() -> Result<()> {
    // Make sure we can parse and assemble the entire kernel.
    let exp = KERNEL.global_labels["exp"];
    let mut rng = thread_rng();
    let a = U256([0; 4].map(|_| rng.gen()));
    let b = U256([0; 4].map(|_| rng.gen()));

    // Random input
    let initial_stack = vec![0xDEADBEEFu32.into(), b, a];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack.clone());

    let stack_with_kernel = run_interpreter(exp, initial_stack)?.stack();

    let expected_exp = a.overflowing_pow(b).0;
    assert_eq!(stack_with_kernel, vec![expected_exp]);

    // 0 base
    let initial_stack = vec![0xDEADBEEFu32.into(), b, U256::zero()];
    let stack_with_kernel = run_interpreter(exp, initial_stack)?.stack();

    let expected_exp = U256::zero().overflowing_pow(b).0;
    assert_eq!(stack_with_kernel, vec![expected_exp]);

    // 0 exponent
    let initial_stack = vec![0xDEADBEEFu32.into(), U256::zero(), a];
    interpreter.set_is_kernel(true);
    interpreter.set_context(0);
    let stack_with_kernel = run_interpreter(exp, initial_stack)?.stack();

    let expected_exp = 1.into();
    assert_eq!(stack_with_kernel, vec![expected_exp]);

    Ok(())
}
