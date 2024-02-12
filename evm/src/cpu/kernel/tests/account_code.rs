use std::collections::HashMap;

use anyhow::{anyhow, Result};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H160, H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use plonky2::field::types::PrimeField64;
use rand::{thread_rng, Rng};
use smt_utils_hermez::code::{hash_bytecode_u256, hash_contract_bytecode};
use smt_utils_hermez::db::{Db, MemoryDb};
use smt_utils_hermez::keys::{key_balance, key_code, key_code_length, key_nonce, key_storage};
use smt_utils_hermez::smt::Smt;
use smt_utils_hermez::utils::{hashout2u, key2u};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata::{self, GasLimit};
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::nibbles_64;
use crate::generation::mpt::{load_all_mpts, AccountRlp};
use crate::generation::TrieInputs;
use crate::memory::segments::Segment;
use crate::witness::memory::MemoryAddress;
use crate::witness::operation::CONTEXT_SCALING_FACTOR;
use crate::Node;

pub(crate) fn initialize_mpts(interpreter: &mut Interpreter, trie_inputs: &TrieInputs) {
    // Load all MPTs.
    let (trie_root_ptrs, trie_data) =
        load_all_mpts(trie_inputs).expect("Invalid MPT data for preinitialization");

    let state_addr =
        MemoryAddress::new_bundle((GlobalMetadata::StateTrieRoot as usize).into()).unwrap();
    let txn_addr =
        MemoryAddress::new_bundle((GlobalMetadata::TransactionTrieRoot as usize).into()).unwrap();
    let receipts_addr =
        MemoryAddress::new_bundle((GlobalMetadata::ReceiptTrieRoot as usize).into()).unwrap();
    let len_addr =
        MemoryAddress::new_bundle((GlobalMetadata::TrieDataSize as usize).into()).unwrap();

    let to_set = [
        (state_addr, trie_root_ptrs.state_root_ptr.into()),
        (txn_addr, trie_root_ptrs.txn_root_ptr.into()),
        (receipts_addr, trie_root_ptrs.receipt_root_ptr.into()),
        (len_addr, trie_data.len().into()),
    ];

    interpreter.set_memory_multi_addresses(&to_set);

    for (i, data) in trie_data.iter().enumerate() {
        let trie_addr = MemoryAddress::new(0, Segment::TrieData, i);
        interpreter
            .generation_state
            .memory
            .set(trie_addr, data.into());
    }
}

// Test account with a given code hash.
fn test_account(code: &[u8]) -> AccountRlp {
    AccountRlp {
        nonce: U256::from(1111),
        balance: U256::from(2222),
        code_hash: hashout2u(hash_contract_bytecode(code.to_vec())),
        ..Default::default()
    }
}

fn random_code() -> Vec<u8> {
    let mut rng = thread_rng();
    let num_bytes = rng.gen_range(0..1000);
    (0..num_bytes).map(|_| rng.gen()).collect()
}

// Stolen from `tests/mpt/insert.rs`
// Prepare the interpreter by inserting the account in the state trie.
fn prepare_interpreter(
    interpreter: &mut Interpreter,
    address: Address,
    account: &AccountRlp,
) -> Result<()> {
    let smt_insert_state = KERNEL.global_labels["smt_insert_state"];
    let smt_hash_state = KERNEL.global_labels["smt_hash_state"];
    let mut state_smt = Smt::<MemoryDb>::default();
    let trie_inputs = Default::default();

    initialize_mpts(interpreter, &trie_inputs);

    let k = nibbles_64(U256::from_big_endian(
        keccak(address.to_fixed_bytes()).as_bytes(),
    ));
    // Next, execute mpt_insert_state_trie.
    let trie_data = interpreter.get_trie_data_mut();
    if trie_data.is_empty() {
        // In the assembly we skip over 0, knowing trie_data[0] = 0 by default.
        // Since we don't explicitly set it to 0, we need to do so here.
        trie_data.push(0.into());
        trie_data.push(0.into());
    }
    let trie_data_len = trie_data.len().into();
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, trie_data_len);
    for (key, value) in [
        (key_balance(address), account.balance),
        (key_nonce(address), account.nonce),
        (key_code(address), account.code_hash),
        (key_code_length(address), account.code_length),
    ] {
        if value.is_zero() {
            continue;
        }
        interpreter.generation_state.registers.program_counter = smt_insert_state;
        interpreter
            .push(0xDEADBEEFu32.into())
            .expect("The stack should not overflow");
        interpreter
            .push(value)
            .expect("The stack should not overflow"); // value_ptr
        let keyu = key2u(key);
        interpreter
            .push(keyu)
            .expect("The stack should not overflow"); // key

        interpreter.run()?;
        assert_eq!(
            interpreter.stack().len(),
            0,
            "Expected empty stack after insert, found {:?}",
            interpreter.stack()
        );
        dbg!("done");
    }
    dbg!("OKOK");

    // Now, execute mpt_hash_state_trie.
    interpreter.generation_state.registers.program_counter = smt_hash_state;
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(2.into()) // Initial length of the trie data segment, unused.
        .expect("The stack should not overflow");
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        2,
        "Expected 2 items on stack after hashing, found {:?}",
        interpreter.stack()
    );
    let hash = interpreter.stack()[1];

    set_account(&mut state_smt, address, account, &HashMap::new());
    let expected_state_trie_hash = hashout2u(state_smt.root);
    assert_eq!(hash, expected_state_trie_hash);

    Ok(())
}

#[test]
fn test_extcodesize() -> Result<()> {
    let code = random_code();
    let account = test_account(&code);

    let mut interpreter = Interpreter::new_with_kernel(0, vec![]);
    let address: Address = thread_rng().gen();
    // Prepare the interpreter by inserting the account in the state trie.
    prepare_interpreter(&mut interpreter, address, &account)?;

    let extcodesize = KERNEL.global_labels["extcodesize"];

    // Test `extcodesize`
    interpreter.generation_state.registers.program_counter = extcodesize;
    interpreter.pop().expect("The stack should not be empty");
    interpreter.pop().expect("The stack should not be empty");
    assert!(interpreter.stack().is_empty());
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(U256::from_big_endian(address.as_bytes()))
        .expect("The stack should not overflow");
    interpreter.generation_state.inputs.contract_code = HashMap::from([(
        hashout2u(hash_contract_bytecode(code.clone())),
        code.clone(),
    )]);
    interpreter.run()?;

    assert_eq!(interpreter.stack(), vec![code.len().into()]);

    Ok(())
}

#[test]
fn test_extcodecopy() -> Result<()> {
    let code = random_code();
    let account = test_account(&code);

    let mut interpreter = Interpreter::new_with_kernel(0, vec![]);
    let address: Address = thread_rng().gen();
    // Prepare the interpreter by inserting the account in the state trie.
    prepare_interpreter(&mut interpreter, address, &account)?;

    let context = interpreter.context();
    interpreter.generation_state.memory.contexts[context].segments
        [Segment::ContextMetadata.unscale()]
    .set(GasLimit.unscale(), U256::from(1000000000000u64));

    let extcodecopy = KERNEL.global_labels["sys_extcodecopy"];

    // Put random data in main memory and the `KernelAccountCode` segment for realism.
    let mut rng = thread_rng();
    for i in 0..2000 {
        interpreter.generation_state.memory.contexts[context].segments
            [Segment::MainMemory.unscale()]
        .set(i, U256::from(rng.gen::<u8>()));
        interpreter.generation_state.memory.contexts[context].segments
            [Segment::KernelAccountCode.unscale()]
        .set(i, U256::from(rng.gen::<u8>()));
    }

    // Random inputs
    let dest_offset = rng.gen_range(0..3000);
    let offset = rng.gen_range(0..1500);
    let size = rng.gen_range(0..1500);

    // Test `extcodecopy`
    interpreter.generation_state.registers.program_counter = extcodecopy;
    interpreter.pop().expect("The stack should not be empty");
    interpreter.pop().expect("The stack should not be empty");
    assert!(interpreter.stack().is_empty());
    interpreter
        .push(size.into())
        .expect("The stack should not overflow");
    interpreter
        .push(offset.into())
        .expect("The stack should not overflow");
    interpreter
        .push(dest_offset.into())
        .expect("The stack should not overflow");
    interpreter
        .push(U256::from_big_endian(address.as_bytes()))
        .expect("The stack should not overflow");
    interpreter
        .push((0xDEADBEEFu64 + (1 << 32)).into())
        .expect("The stack should not overflow"); // kexit_info
    interpreter.generation_state.inputs.contract_code = HashMap::from([(
        hashout2u(hash_contract_bytecode(code.clone())),
        code.clone(),
    )]);
    interpreter.run()?;

    assert!(interpreter.stack().is_empty());
    // Check that the code was correctly copied to memory.
    for i in 0..size {
        let memory = interpreter.generation_state.memory.contexts[context].segments
            [Segment::MainMemory.unscale()]
        .get(dest_offset + i);
        assert_eq!(
            memory,
            code.get(offset + i).copied().unwrap_or_default().into()
        );
    }

    Ok(())
}

/// Prepare the interpreter for storage tests by inserting all necessary accounts
/// in the state trie, adding the code we want to context 1 and switching the context.
fn prepare_interpreter_all_accounts(
    interpreter: &mut Interpreter,
    trie_inputs: TrieInputs,
    addr: [u8; 20],
    code: &[u8],
) -> Result<()> {
    // Load all MPTs.
    initialize_mpts(interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    // Switch context and initialize memory with the data we need for the tests.
    interpreter.generation_state.registers.program_counter = 0;
    interpreter.set_code(1, code.to_vec());
    interpreter.set_context_metadata_field(
        1,
        ContextMetadata::Address,
        U256::from_big_endian(&addr),
    );
    interpreter.set_context_metadata_field(1, ContextMetadata::GasLimit, 100_000.into());
    interpreter.set_context(1);
    interpreter.set_is_kernel(false);
    interpreter.set_context_metadata_field(
        1,
        ContextMetadata::ParentProgramCounter,
        0xdeadbeefu32.into(),
    );
    interpreter.set_context_metadata_field(
        1,
        ContextMetadata::ParentContext,
        U256::one() << CONTEXT_SCALING_FACTOR, // ctx = 1
    );

    Ok(())
}

/// Tests an SSTORE within a code similar to the contract code in add11_yml.
#[test]
fn sstore() -> Result<()> {
    // We take the same `to` account as in add11_yml.
    let addr = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    let code = [0x60, 0x01, 0x60, 0x01, 0x01, 0x60, 0x00, 0x55, 0x00];
    let code_hash = hash_bytecode_u256(code.to_vec());

    let account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        code_hash,
        ..AccountRlp::default()
    };

    let mut state_smt_before = Smt::<MemoryDb>::default();
    set_account(
        &mut state_smt_before,
        H160(addr),
        &account_before,
        &HashMap::new(),
    );

    let trie_inputs = TrieInputs {
        state_smt: state_smt_before.serialize(),
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
    };

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);

    // Prepare the interpreter by inserting the account in the state trie.
    prepare_interpreter_all_accounts(&mut interpreter, trie_inputs, addr, &code)?;

    interpreter.run()?;

    // The first two elements in the stack are `success` and `leftover_gas`,
    // returned by the `sys_stop` opcode.
    interpreter.pop();
    interpreter.pop();

    // Now, execute smt_hash_state.
    let smt_hash_state = KERNEL.global_labels["smt_hash_state"];
    interpreter.generation_state.registers.program_counter = smt_hash_state;
    interpreter.set_is_kernel(true);
    interpreter.set_context(0);
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(1.into()) // Initial length of the trie data segment, unused.
        .expect("The stack should not overflow");
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        2,
        "Expected 2 items on stack after hashing, found {:?}",
        interpreter.stack()
    );

    let hash = interpreter.stack()[1];

    let mut expected_state_smt_after = Smt::<MemoryDb>::default();
    set_account(
        &mut expected_state_smt_after,
        H160(addr),
        &account_before,
        &[(0.into(), 2.into())].into(),
    );

    let expected_state_trie_hash = hashout2u(expected_state_smt_after.root);
    assert_eq!(hash, expected_state_trie_hash);
    Ok(())
}

/// Tests an SLOAD within a code similar to the contract code in add11_yml.
#[test]
fn sload() -> Result<()> {
    // We take the same `to` account as in add11_yml.
    let addr = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    // This code is similar to the one in add11_yml's contract, but we pop the added value
    // and carry out an SLOAD instead of an SSTORE. We also add a PUSH at the end.
    let code = [
        0x60, 0x01, 0x60, 0x01, 0x01, 0x50, 0x60, 0x00, 0x54, 0x60, 0x03, 0x00,
    ];
    let code_hash = hash_bytecode_u256(code.to_vec());

    let account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        code_hash,
        ..AccountRlp::default()
    };

    let mut state_smt_before = Smt::<MemoryDb>::default();
    set_account(
        &mut state_smt_before,
        H160(addr),
        &account_before,
        &HashMap::new(),
    );

    let trie_inputs = TrieInputs {
        state_smt: state_smt_before.serialize(),
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
    };

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);

    // Prepare the interpreter by inserting the account in the state trie.
    prepare_interpreter_all_accounts(&mut interpreter, trie_inputs, addr, &code)?;
    interpreter.run()?;

    // The first two elements in the stack are `success` and `leftover_gas`,
    // returned by the `sys_stop` opcode.
    interpreter
        .pop()
        .expect("The stack length should not be empty.");
    interpreter
        .pop()
        .expect("The stack length should not be empty.");

    // The SLOAD in the provided code should return 0, since
    // the storage trie is empty. The last step in the code
    // pushes the value 3.
    assert_eq!(interpreter.stack(), vec![0x0.into(), 0x3.into()]);
    interpreter
        .pop()
        .expect("The stack length should not be empty.");
    interpreter
        .pop()
        .expect("The stack length should not be empty.");

    // Now, execute smt_hash_state.
    let smt_hash_state = KERNEL.global_labels["smt_hash_state"];
    interpreter.generation_state.registers.program_counter = smt_hash_state;
    interpreter.set_is_kernel(true);
    interpreter.set_context(0);
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow.");
    interpreter
        .push(2.into()) // Initial length of the trie data segment, unused.
        .expect("The stack should not overflow.");
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        2,
        "Expected 2 items on stack after hashing, found {:?}",
        interpreter.stack()
    );

    let trie_data_segment_len = interpreter.stack()[0];
    dbg!(interpreter.get_memory_segment(Segment::TrieData));
    assert_eq!(
        trie_data_segment_len,
        interpreter
            .get_memory_segment(Segment::TrieData)
            .len()
            .into()
    );

    let hash = interpreter.stack()[1];

    let expected_state_trie_hash = hashout2u(state_smt_before.root);
    assert_eq!(hash, expected_state_trie_hash);
    Ok(())
}

pub(crate) fn set_account<D: Db>(
    smt: &mut Smt<D>,
    addr: Address,
    account: &AccountRlp,
    storage: &HashMap<U256, U256>,
) {
    smt.set(key_balance(addr), account.balance);
    smt.set(key_nonce(addr), account.nonce);
    smt.set(key_code(addr), account.code_hash);
    smt.set(key_code_length(addr), account.code_length);
    for (&k, &v) in storage {
        smt.set(key_storage(addr, k), v);
    }
}
