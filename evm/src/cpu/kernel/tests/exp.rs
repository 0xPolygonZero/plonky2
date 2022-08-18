use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::{run, run_with_kernel};

#[test]
fn test_exp() -> Result<()> {
    // Make sure we can parse and assemble the entire kernel.
    let kernel = combined_kernel();
    let exp = kernel.global_labels["exp"];
    let mut rng = thread_rng();
    let a = U256([0; 4].map(|_| rng.gen()));
    let b = U256([0; 4].map(|_| rng.gen()));

    // Random input
    let initial_stack = vec![0xDEADBEEFu32.into(), b, a];
    let stack_with_kernel = run_with_kernel(&kernel, exp, initial_stack)?
        .stack()
        .to_vec();
    let initial_stack = vec![b, a];
    let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP
    let stack_with_opcode = run(&code, 0, initial_stack, &kernel.prover_inputs)?
        .stack()
        .to_vec();
    assert_eq!(stack_with_kernel, stack_with_opcode);

    // 0 base
    let initial_stack = vec![0xDEADBEEFu32.into(), b, U256::zero()];
    let stack_with_kernel = run_with_kernel(&kernel, exp, initial_stack)?
        .stack()
        .to_vec();
    let initial_stack = vec![b, U256::zero()];
    let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP
    let stack_with_opcode = run(&code, 0, initial_stack, &kernel.prover_inputs)?
        .stack()
        .to_vec();
    assert_eq!(stack_with_kernel, stack_with_opcode);

    // 0 exponent
    let initial_stack = vec![0xDEADBEEFu32.into(), U256::zero(), a];
    let stack_with_kernel = run_with_kernel(&kernel, exp, initial_stack)?
        .stack()
        .to_vec();
    let initial_stack = vec![U256::zero(), a];
    let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP
    let stack_with_opcode = run(&code, 0, initial_stack, &kernel.prover_inputs)?
        .stack()
        .to_vec();
    assert_eq!(stack_with_kernel, stack_with_opcode);

    Ok(())
}
