use std::str::FromStr;

use anyhow::Result;
use ethereum_types::{H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;

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

struct Create2TestCase {
    code_hash: H256,
    salt: U256,
    sender: U256,
    expected_addr: U256,
}

/// Taken from https://eips.ethereum.org/EIPS/eip-1014
fn create2_test_cases() -> Vec<Create2TestCase> {
    vec![
        Create2TestCase {
            code_hash: keccak(hex!("00")),
            salt: U256::zero(),
            sender: U256::zero(),
            expected_addr: U256::from_str("0x4D1A2e2bB4F88F0250f26Ffff098B0b30B26BF38").unwrap(),
        },
        Create2TestCase {
            code_hash: keccak(hex!("00")),
            salt: U256::zero(),
            sender: U256::from_str("0xdeadbeef00000000000000000000000000000000").unwrap(),
            expected_addr: U256::from_str("0xB928f69Bb1D91Cd65274e3c79d8986362984fDA3").unwrap(),
        },
        Create2TestCase {
            code_hash: keccak(hex!("00")),
            salt: U256::from_str(
                "0x000000000000000000000000feed000000000000000000000000000000000000",
            )
            .unwrap(),
            sender: U256::from_str("0xdeadbeef00000000000000000000000000000000").unwrap(),
            expected_addr: U256::from_str("0xD04116cDd17beBE565EB2422F2497E06cC1C9833").unwrap(),
        },
        Create2TestCase {
            code_hash: keccak(hex!("deadbeef")),
            salt: U256::zero(),
            sender: U256::zero(),
            expected_addr: U256::from_str("0x70f2b2914A2a4b783FaEFb75f459A580616Fcb5e").unwrap(),
        },
        Create2TestCase {
            code_hash: keccak(hex!("deadbeef")),
            salt: U256::from_str(
                "0x00000000000000000000000000000000000000000000000000000000cafebabe",
            )
            .unwrap(),
            sender: U256::from_str("0x00000000000000000000000000000000deadbeef").unwrap(),
            expected_addr: U256::from_str("0x60f3f640a8508fC6a86d45DF051962668E1e8AC7").unwrap(),
        },
        Create2TestCase {
            code_hash: keccak(hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef")),
            salt: U256::from_str(
                "0x00000000000000000000000000000000000000000000000000000000cafebabe",
            )
            .unwrap(),
            sender: U256::from_str("0x00000000000000000000000000000000deadbeef").unwrap(),
            expected_addr: U256::from_str("0x1d8bfDC5D46DC4f61D6b6115972536eBE6A8854C").unwrap(),
        },
        Create2TestCase {
            code_hash: keccak(hex!("")),
            salt: U256::zero(),
            sender: U256::zero(),
            expected_addr: U256::from_str("0xE33C0C7F7df4809055C3ebA6c09CFe4BaF1BD9e0").unwrap(),
        },
    ]
}

#[test]
fn test_get_create2_address() -> Result<()> {
    let get_create2_address = KERNEL.global_labels["get_create2_address"];

    let retaddr = 0xdeadbeefu32.into();

    for Create2TestCase {
        code_hash,
        salt,
        sender,
        expected_addr,
    } in create2_test_cases()
    {
        let initial_stack = vec![retaddr, salt, U256::from(code_hash.0), sender];
        let mut interpreter = Interpreter::new_with_kernel(get_create2_address, initial_stack);
        interpreter.run()?;

        assert_eq!(interpreter.stack(), &[expected_addr]);
    }

    Ok(())
}
