use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run;

#[test]
fn test_exp() -> Result<()> {
    // Make sure we can parse and assemble the entire kernel.
    let kernel = combined_kernel();
    let exp = kernel.global_labels["exp"];
    let mut rng = thread_rng();
    let a = U256([0; 4].map(|_| rng.gen()));
    let b = U256([0; 4].map(|_| rng.gen()));

    // Random input
    let initial_stack = vec![U256::from_str("0xdeadbeef")?, b, a];
    let stack_with_kernel = run(&kernel.code, exp, initial_stack)?.stack;
    let initial_stack = vec![b, a];
    let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP
    let stack_with_opcode = run(&code, 0, initial_stack)?.stack;
    assert_eq!(stack_with_kernel, stack_with_opcode);

    // 0 base
    let initial_stack = vec![U256::from_str("0xdeadbeef")?, b, U256::zero()];
    let stack_with_kernel = run(&kernel.code, exp, initial_stack)?.stack;
    let initial_stack = vec![b, U256::zero()];
    let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP
    let stack_with_opcode = run(&code, 0, initial_stack)?.stack;
    assert_eq!(stack_with_kernel, stack_with_opcode);

    // 0 exponent
    let initial_stack = vec![U256::from_str("0xdeadbeef")?, U256::zero(), a];
    let stack_with_kernel = run(&kernel.code, exp, initial_stack)?.stack;
    let initial_stack = vec![U256::zero(), a];
    let code = [0xa, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56]; // EXP, PUSH4 deadbeef, JUMP
    let stack_with_opcode = run(&code, 0, initial_stack)?.stack;
    assert_eq!(stack_with_kernel, stack_with_opcode);

    Ok(())
}
