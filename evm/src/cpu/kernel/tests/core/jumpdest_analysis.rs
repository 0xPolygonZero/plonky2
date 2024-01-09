use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::opcodes::{get_opcode, get_push_opcode};
use crate::witness::operation::CONTEXT_SCALING_FACTOR;

#[test]
fn test_jumpdest_analysis() -> Result<()> {
    let jumpdest_analysis = KERNEL.global_labels["jumpdest_analysis"];
    const CONTEXT: usize = 3; // arbitrary

    let add = get_opcode("ADD");
    let push2 = get_push_opcode(2);
    let jumpdest = get_opcode("JUMPDEST");

    #[rustfmt::skip]
    let code: Vec<u8> = vec![
        add,
        jumpdest,
        push2,
        jumpdest, // part of PUSH2
        jumpdest, // part of PUSH2
        jumpdest,
        add,
        jumpdest,
    ];

    let jumpdest_bits = vec![false, true, false, false, false, true, false, true];

    // Contract creation transaction.
    let initial_stack = vec![
        0xDEADBEEFu32.into(),
        code.len().into(),
        U256::from(CONTEXT) << CONTEXT_SCALING_FACTOR,
    ];
    let mut interpreter = Interpreter::new_with_kernel(jumpdest_analysis, initial_stack);
    interpreter.set_code(CONTEXT, code);
    interpreter.set_jumpdest_bits(CONTEXT, jumpdest_bits);

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    Ok(())
}
