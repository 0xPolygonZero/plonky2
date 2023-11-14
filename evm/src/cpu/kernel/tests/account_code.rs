use std::collections::HashMap;

use anyhow::{anyhow, Result};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata::{self, GasLimit};
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::nibbles_64;
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};
use crate::generation::TrieInputs;
use crate::memory::segments::Segment;
use crate::witness::memory::MemoryAddress;
use crate::Node;

// Test account with a given code hash.
fn test_account(code: &[u8]) -> AccountRlp {
    AccountRlp {
        nonce: U256::from(1111),
        balance: U256::from(2222),
        storage_root: HashedPartialTrie::from(Node::Empty).hash(),
        code_hash: keccak(code),
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
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let mpt_insert_state_trie = KERNEL.global_labels["mpt_insert_state_trie"];
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];
    let mut state_trie: HashedPartialTrie = Default::default();
    let trie_inputs = Default::default();

    interpreter.generation_state.registers.program_counter = load_all_mpts;
    interpreter.push(0xDEADBEEFu32.into());

    interpreter.generation_state.mpt_prover_inputs =
        all_mpt_prover_inputs_reversed(&trie_inputs)
            .map_err(|err| anyhow!("Invalid MPT data: {:?}", err))?;
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    let k = nibbles_64(U256::from_big_endian(
        keccak(address.to_fixed_bytes()).as_bytes(),
    ));
    // Next, execute mpt_insert_state_trie.
    interpreter.generation_state.registers.program_counter = mpt_insert_state_trie;
    let trie_data = interpreter.get_trie_data_mut();
    if trie_data.is_empty() {
        // In the assembly we skip over 0, knowing trie_data[0] = 0 by default.
        // Since we don't explicitly set it to 0, we need to do so here.
        trie_data.push(0.into());
    }
    let value_ptr = trie_data.len();
    trie_data.push(account.nonce);
    trie_data.push(account.balance);
    // In memory, storage_root gets interpreted as a pointer to a storage trie,
    // so we have to ensure the pointer is valid. It's easiest to set it to 0,
    // which works as an empty node, since trie_data[0] = 0 = MPT_TYPE_EMPTY.
    trie_data.push(H256::zero().into_uint());
    trie_data.push(account.code_hash.into_uint());
    let trie_data_len = trie_data.len().into();
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, trie_data_len);
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(value_ptr.into()); // value_ptr
    interpreter.push(k.try_into_u256().unwrap()); // key

    interpreter.run()?;
    assert_eq!(
        interpreter.stack().len(),
        0,
        "Expected empty stack after insert, found {:?}",
        interpreter.stack()
    );

    // Now, execute mpt_hash_state_trie.
    interpreter.generation_state.registers.program_counter = mpt_hash_state_trie;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack after hashing, found {:?}",
        interpreter.stack()
    );
    let hash = H256::from_uint(&interpreter.stack()[0]);

    state_trie.insert(k, rlp::encode(account).to_vec());
    let expected_state_trie_hash = state_trie.hash();
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
    interpreter.pop();
    assert!(interpreter.stack().is_empty());
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(U256::from_big_endian(address.as_bytes()));
    interpreter.generation_state.inputs.contract_code =
        HashMap::from([(keccak(&code), code.clone())]);
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
        [Segment::ContextMetadata as usize]
        .set(GasLimit as usize, U256::from(1000000000000u64));

    let extcodecopy = KERNEL.global_labels["sys_extcodecopy"];

    // Put random data in main memory and the `KernelAccountCode` segment for realism.
    let mut rng = thread_rng();
    for i in 0..2000 {
        interpreter.generation_state.memory.contexts[context].segments
            [Segment::MainMemory as usize]
            .set(i, U256::from(rng.gen::<u8>()));
        interpreter.generation_state.memory.contexts[context].segments
            [Segment::KernelAccountCode as usize]
            .set(i, U256::from(rng.gen::<u8>()));
    }

    // Random inputs
    let dest_offset = rng.gen_range(0..3000);
    let offset = rng.gen_range(0..1500);
    let size = rng.gen_range(0..1500);

    // Test `extcodecopy`
    interpreter.generation_state.registers.program_counter = extcodecopy;
    interpreter.pop();
    assert!(interpreter.stack().is_empty());
    interpreter.push(size.into());
    interpreter.push(offset.into());
    interpreter.push(dest_offset.into());
    interpreter.push(U256::from_big_endian(address.as_bytes()));
    interpreter.push((0xDEADBEEFu64 + (1 << 32)).into()); // kexit_info
    interpreter.generation_state.inputs.contract_code =
        HashMap::from([(keccak(&code), code.clone())]);
    interpreter.run()?;

    assert!(interpreter.stack().is_empty());
    // Check that the code was correctly copied to memory.
    for i in 0..size {
        let memory = interpreter.generation_state.memory.contexts[interpreter.context()].segments
            [Segment::MainMemory as usize]
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
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];

    interpreter.generation_state.registers.program_counter = load_all_mpts;
    interpreter.push(0xDEADBEEFu32.into());

    interpreter.generation_state.mpt_prover_inputs =
        all_mpt_prover_inputs_reversed(&trie_inputs)
            .map_err(|err| anyhow!("Invalid MPT data: {:?}", err))?;
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    // Switch context and initialize memory with the data we need for the tests.
    interpreter.generation_state.registers.program_counter = 0;
    interpreter.set_code(1, code.to_vec());
    interpreter.generation_state.memory.contexts[1].segments[Segment::ContextMetadata as usize]
        .set(
            ContextMetadata::Address as usize,
            U256::from_big_endian(&addr),
        );
    interpreter.generation_state.memory.contexts[1].segments[Segment::ContextMetadata as usize]
        .set(ContextMetadata::GasLimit as usize, 100_000.into());
    *interpreter.context_mut() = 1;
    *interpreter.is_kernel_mut() = false;
    interpreter.generation_state.memory.set(
        MemoryAddress::new(
            1,
            Segment::ContextMetadata,
            ContextMetadata::ParentProgramCounter as usize,
        ),
        0xdeadbeefu32.into(),
    );
    interpreter.generation_state.memory.set(
        MemoryAddress::new(
            1,
            Segment::ContextMetadata,
            ContextMetadata::ParentContext as usize,
        ),
        1.into(),
    );

    Ok(())
}

/// Tests an SSTORE within a code similar to the contract code in add11_yml.
#[test]
fn sstore() -> Result<()> {
    // We take the same `to` account as in add11_yml.
    let addr = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    let addr_hashed = keccak(addr);

    let addr_nibbles = Nibbles::from_bytes_be(addr_hashed.as_bytes()).unwrap();

    let code = [0x60, 0x01, 0x60, 0x01, 0x01, 0x60, 0x00, 0x55, 0x00];
    let code_hash = keccak(code);

    let account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        code_hash,
        ..AccountRlp::default()
    };

    let mut state_trie_before = HashedPartialTrie::from(Node::Empty);

    state_trie_before.insert(addr_nibbles, rlp::encode(&account_before).to_vec());

    let trie_inputs = TrieInputs {
        state_trie: state_trie_before.clone(),
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries: vec![(addr_hashed, Node::Empty.into())],
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

    // The code should have added an element to the storage of `to_account`. We run
    // `mpt_hash_state_trie` to check that.
    let account_after = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        code_hash,
        storage_root: HashedPartialTrie::from(Node::Leaf {
            nibbles: Nibbles::from_h256_be(keccak([0u8; 32])),
            value: vec![2],
        })
        .hash(),
        ..AccountRlp::default()
    };
    // Now, execute mpt_hash_state_trie.
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];
    interpreter.generation_state.registers.program_counter = mpt_hash_state_trie;
    *interpreter.context_mut() = 0;
    interpreter.generation_state.registers.context = 0;
    interpreter.generation_state.registers.is_kernel = true;
    *interpreter.is_kernel_mut() = true;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack after hashing, found {:?}",
        interpreter.stack()
    );

    let hash = H256::from_uint(&interpreter.stack()[0]);

    let mut expected_state_trie_after = HashedPartialTrie::from(Node::Empty);
    expected_state_trie_after.insert(addr_nibbles, rlp::encode(&account_after).to_vec());

    let expected_state_trie_hash = expected_state_trie_after.hash();
    assert_eq!(hash, expected_state_trie_hash);
    Ok(())
}

/// Tests an SLOAD within a code similar to the contract code in add11_yml.
#[test]
fn sload() -> Result<()> {
    // We take the same `to` account as in add11_yml.
    let addr = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    let addr_hashed = keccak(addr);

    let addr_nibbles = Nibbles::from_bytes_be(addr_hashed.as_bytes()).unwrap();

    // This code is similar to the one in add11_yml's contract, but we pop the added value
    // and carry out an SLOAD instead of an SSTORE.
    let code = [0x60, 0x01, 0x60, 0x01, 0x01, 0x50, 0x60, 0x00, 0x54, 0x00];
    let code_hash = keccak(code);

    let account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        code_hash,
        ..AccountRlp::default()
    };

    let mut state_trie_before = HashedPartialTrie::from(Node::Empty);

    state_trie_before.insert(addr_nibbles, rlp::encode(&account_before).to_vec());

    let trie_inputs = TrieInputs {
        state_trie: state_trie_before.clone(),
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries: vec![(addr_hashed, Node::Empty.into())],
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

    // We check that no value was found.
    let value = interpreter.pop();
    assert_eq!(value, 0.into());
    // Now, execute mpt_hash_state_trie. We check that the state trie has not changed.
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];
    interpreter.generation_state.registers.program_counter = mpt_hash_state_trie;
    *interpreter.context_mut() = 0;
    interpreter.generation_state.registers.context = 0;
    interpreter.generation_state.registers.is_kernel = true;
    *interpreter.is_kernel_mut() = true;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack after hashing, found {:?}",
        interpreter.stack()
    );

    let hash = H256::from_uint(&interpreter.stack()[0]);

    let expected_state_trie_hash = state_trie_before.hash();
    assert_eq!(hash, expected_state_trie_hash);
    Ok(())
}
