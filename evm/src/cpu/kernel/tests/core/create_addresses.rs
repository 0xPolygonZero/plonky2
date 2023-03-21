use anyhow::Result;
use ethereum_types::U256;
use hex_literal::hex;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

#[test]
fn test_get_create_address() -> Result<()> {
    let get_create_address = KERNEL.global_labels["get_create_address"];

    // This is copied from OpenEthereum's `test_contract_address`.
    let retaddr = 0xdeadbeefu32.into();
    let nonce = 88.into();
    let sender = U256::from_big_endian(&hex!("0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6"));
    let expected_addr = U256::from_big_endian(&hex!("3f09c73a5ed19289fb9bdc72f1742566df146f56"));

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
    let code_hash = 0.into();
    let salt = 5.into();
    let sender = 0.into();
    let expected_addr = 123.into();

    let initial_stack = vec![retaddr, code_hash, salt, sender];
    let mut interpreter = Interpreter::new_with_kernel(get_create2_address, initial_stack);
    interpreter.run()?;

    assert_eq!(interpreter.stack(), &[expected_addr]);

    Ok(())
}
