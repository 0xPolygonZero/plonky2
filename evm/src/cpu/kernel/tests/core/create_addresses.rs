use anyhow::Result;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

#[test]
fn test_get_create_address() -> Result<()> {
    let get_create_address = KERNEL.global_labels["get_create_address"];

    // TODO: Replace with real data once we have a real implementation.
    let retaddr = 0xdeadbeefu32.into();
    let nonce = 5.into();
    let sender = 0.into();
    let expected_addr = 123.into();

    let initial_stack = vec![retaddr, nonce, sender];
    let mut interpreter = Interpreter::new_with_kernel(get_create_address, initial_stack);
    interpreter.run()?;

    assert_eq!(interpreter.stack(), &[expected_addr]);

    Ok(())
}

#[test]
fn test_get_create2_address() -> Result<()> {
    let get_create2_address = KERNEL.global_labels["get_create2_address"];

    // TODO: Replace with real data once we have a real implementation.
    let retaddr = 0xdeadbeefu32.into();
    let code_len = 0.into();
    let code_offset = 0.into();
    let code_segment = 0.into();
    let code_context = 0.into();
    let salt = 5.into();
    let sender = 0.into();
    let expected_addr = 123.into();

    let initial_stack = vec![
        retaddr,
        code_len,
        code_offset,
        code_segment,
        code_context,
        salt,
        sender,
    ];
    let mut interpreter = Interpreter::new_with_kernel(get_create2_address, initial_stack);
    interpreter.run()?;

    assert_eq!(interpreter.stack(), &[expected_addr]);

    Ok(())
}
