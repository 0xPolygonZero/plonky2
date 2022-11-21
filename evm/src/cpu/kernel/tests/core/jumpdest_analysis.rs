use anyhow::Result;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::opcodes::{get_opcode, get_push_opcode};

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

    let expected_jumpdest_bits = vec![false, true, false, false, false, true, false, true];

    // Contract creation transaction.
    let initial_stack = vec![0xDEADBEEFu32.into(), code.len().into(), CONTEXT.into()];
    let mut interpreter = Interpreter::new_with_kernel(jumpdest_analysis, initial_stack);
    interpreter.set_code(CONTEXT, code);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);
    assert_eq!(
        interpreter.get_jumpdest_bits(CONTEXT),
        expected_jumpdest_bits
    );

    Ok(())
}
