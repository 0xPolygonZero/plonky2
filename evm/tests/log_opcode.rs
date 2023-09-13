#![allow(clippy::upper_case_acronyms)]

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use bytes::Bytes;
use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::fixed_recursive_verifier::AllRecursiveCircuits;
use plonky2_evm::generation::mpt::{AccountRlp, LegacyReceiptRlp, LegacyTransactionRlp, LogRlp};
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockHashes, BlockMetadata, ExtraBlockData, PublicValues, TrieRoots};
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;
use plonky2_evm::Node;

type F = GoldilocksField;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;

/// Variation of `add11_yml` testing LOG opcodes.
#[test]
#[ignore] // Too slow to run on CI.
fn test_log_opcodes() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba");
    let sender = hex!("af1276cbb260bb13deddb4209ae99ae6e497f446");
    // Private key: DCDFF53B4F013DBCDC717F89FE3BF4D8B10512AAE282B48E01D7530470382701
    let to = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_state_key = keccak(sender);
    let to_hashed = keccak(to);

    let beneficiary_nibbles = Nibbles::from_bytes_be(beneficiary_state_key.as_bytes()).unwrap();
    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let to_nibbles = Nibbles::from_bytes_be(to_hashed.as_bytes()).unwrap();

    // For the first code transaction code, we consider two LOG opcodes. The first deals with 0 topics and empty data. The second deals with two topics, and data of length 5, stored in memory.
    let code = [
        0x64, 0xA1, 0xB2, 0xC3, 0xD4, 0xE5, 0x60, 0x0, 0x52, // MSTORE(0x0, 0xA1B2C3D4E5)
        0x60, 0x0, 0x60, 0x0, 0xA0, // LOG0(0x0, 0x0)
        0x60, 99, 0x60, 98, 0x60, 5, 0x60, 27, 0xA2, // LOG2(27, 5, 98, 99)
        0x00,
    ];
    println!("contract: {:02x?}", code);
    let code_gas = 3 + 3 + 3 // PUSHs and MSTORE
                 + 3 + 3 + 375 // PUSHs and LOG0
                 + 3 + 3 + 3 + 3 + 375 + 375*2 + 8*5 + 3// PUSHs, LOG2 and memory expansion
    ;
    let gas_used = 21_000 + code_gas;

    let code_hash = keccak(code);

    // Set accounts before the transaction.
    let beneficiary_account_before = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };

    let sender_balance_before = 5000000000000000u64;
    let sender_account_before = AccountRlp {
        balance: sender_balance_before.into(),
        ..AccountRlp::default()
    };
    let to_account_before = AccountRlp {
        balance: 9000000000u64.into(),
        code_hash,
        ..AccountRlp::default()
    };

    // Initialize the state trie with three accounts.
    let mut state_trie_before = HashedPartialTrie::from(Node::Empty);
    state_trie_before.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_before).to_vec(),
    );
    state_trie_before.insert(sender_nibbles, rlp::encode(&sender_account_before).to_vec());
    state_trie_before.insert(to_nibbles, rlp::encode(&to_account_before).to_vec());

    // We now add two receipts with logs and data. This updates the receipt trie as well.
    let log_0 = LogRlp {
        address: hex!("7ef66b77759e12Caf3dDB3E4AFF524E577C59D8D").into(),
        topics: vec![
            hex!("8a22ee899102a366ac8ad0495127319cb1ff2403cfae855f83a89cda1266674d").into(),
            hex!("000000000000000000000000000000000000000000000000000000000000002a").into(),
            hex!("0000000000000000000000000000000000000000000000000000000000bd9fe6").into(),
        ],
        data: hex!("f7af1cc94b1aef2e0fa15f1b4baefa86eb60e78fa4bd082372a0a446d197fb58")
            .to_vec()
            .into(),
    };

    let receipt_0 = LegacyReceiptRlp {
            status: true,
            cum_gas_used: 0x016e5bu64.into(),
            bloom: hex!("00000000000000000000000000000000000000000000000000800000000000000040000000005000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000000000000000080008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000020000000000008000000000000000000000000").to_vec().into(),
            logs: vec![log_0],
        };

    // Insert the first receipt into the initial receipt trie. The initial receipts trie has an initial node with a random nibble.
    let mut receipts_trie = HashedPartialTrie::from(Node::Empty);
    receipts_trie.insert(
        Nibbles::from_str("0x1337").unwrap(),
        rlp::encode(&receipt_0).to_vec(),
    );

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: Node::Empty.into(),
        receipts_trie: receipts_trie.clone(),
        storage_tries: vec![(to_hashed, Node::Empty.into())],
    };

    // Prove a transaction which carries out two LOG opcodes.
    let txn_gas_price = 10;
    let txn = hex!("f860800a830186a094095e7baea6a6c7c4c2dfeb977efac326af552d87808026a0c3040cb042c541f9440771879b6bbf3f91464b265431de87eea1ec3206350eb8a046f5f3d06b8816f19f24ee919fd84bfb736db71df10a72fba4495f479e96f678");

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 0x03e8.into(),
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_gaslimit: 0xffffffffu32.into(),
        block_chain_id: 1.into(),
        block_base_fee: 0xa.into(),
        block_gas_used: 0.into(),
        block_bloom: [0.into(); 8],
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    // Update the state and receipt tries after the transaction, so that we have the correct expected tries:
    // Update accounts
    let beneficiary_account_after = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };

    let sender_balance_after = sender_balance_before - gas_used * txn_gas_price;
    let sender_account_after = AccountRlp {
        balance: sender_balance_after.into(),
        nonce: 1.into(),
        ..AccountRlp::default()
    };
    let to_account_after = AccountRlp {
        balance: 9000000000u64.into(),
        code_hash,
        ..AccountRlp::default()
    };

    // Update the receipt trie.
    let first_log = LogRlp {
        address: to.into(),
        topics: vec![],
        data: Bytes::new(),
    };

    let second_log = LogRlp {
        address: to.into(),
        topics: vec![
            hex!("0000000000000000000000000000000000000000000000000000000000000062").into(), // dec: 98
            hex!("0000000000000000000000000000000000000000000000000000000000000063").into(), // dec: 99
        ],
        data: hex!("a1b2c3d4e5").to_vec().into(),
    };

    let receipt = LegacyReceiptRlp {
        status: true,
        cum_gas_used: gas_used.into(),
        bloom: hex!("00000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000004000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000000000000000400000000000040000000000000000000000000002000000000000000000000000000").to_vec().into(),
        logs: vec![first_log, second_log],
    };

    let receipt_nibbles = Nibbles::from_str("0x80").unwrap(); // RLP(0) = 0x80

    receipts_trie.insert(receipt_nibbles, rlp::encode(&receipt).to_vec());

    // Update the state trie.
    let mut expected_state_trie_after = HashedPartialTrie::from(Node::Empty);
    expected_state_trie_after.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_after).to_vec(),
    );
    expected_state_trie_after.insert(sender_nibbles, rlp::encode(&sender_account_after).to_vec());
    expected_state_trie_after.insert(to_nibbles, rlp::encode(&to_account_after).to_vec());

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: HashedPartialTrie::from(Node::Empty).hash(),
        receipts_root: receipts_trie.hash(),
    };
    let block_bloom_after = [
        U256::from_dec_str("392318858461667547739736838950479151006397215279002157056").unwrap(),
        0.into(),
        U256::from_dec_str(
            "55213970774324510299478046898216203619608871777363092441300193790394368",
        )
        .unwrap(),
        U256::from_dec_str("1361129467683753853853498429727072845824").unwrap(),
        U256::from_dec_str("33554432").unwrap(),
        U256::from_dec_str("98079714615416886934934209737619787760822675856605315072").unwrap(),
        U256::from_dec_str("262144").unwrap(),
        U256::from_dec_str("6739986666787659948666753771754908317446393422488596686587943714816")
            .unwrap(),
    ];
    let inputs = GenerationInputs {
        signed_txns: vec![txn.to_vec()],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        block_metadata,
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: gas_used.into(),
        block_bloom_before: [0.into(); 8],
        block_bloom_after,

        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
        addresses: vec![],
    };

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    // Assert that the proof leads to the correct state and receipt roots.
    assert_eq!(
        proof.public_values.trie_roots_after.state_root,
        expected_state_trie_after.hash()
    );

    assert_eq!(
        proof.public_values.trie_roots_after.receipts_root,
        receipts_trie.hash()
    );

    verify_proof(&all_stark, proof, &config)
}

// Tests proving two transactions, one of which with logs, and aggregating them.
#[test]
#[ignore] // Too slow to run on CI.
fn test_log_with_aggreg() -> anyhow::Result<()> {
    init_logger();

    let code = [
        0x64, 0xA1, 0xB2, 0xC3, 0xD4, 0xE5, 0x60, 0x0, 0x52, // MSTORE(0x0, 0xA1B2C3D4E5)
        0x60, 0x0, 0x60, 0x0, 0xA0, // LOG0(0x0, 0x0)
        0x60, 99, 0x60, 98, 0x60, 5, 0x60, 27, 0xA2, // LOG2(27, 5, 98, 99)
        0x00,
    ];

    let code_gas = 3 + 3 + 3 // PUSHs and MSTORE
                 + 3 + 3 + 375 // PUSHs and LOG0
                 + 3 + 3 + 3 + 3 + 375 + 375*2 + 8*5 // PUSHs and LOG2
                 + 3 // Memory expansion
    ;

    let gas_used = 21_000 + code_gas;

    let code_hash = keccak(code);

    // First transaction.
    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba");
    let sender_first = hex!("af1276cbb260bb13deddb4209ae99ae6e497f446");
    let to_first = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");
    let to = hex!("095e7baea6a6c7c4c2dfeb977efac326af552e89");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_state_key = keccak(sender_first);
    let to_hashed = keccak(to_first);
    let to_hashed_2 = keccak(to);

    let beneficiary_nibbles = Nibbles::from_bytes_be(beneficiary_state_key.as_bytes()).unwrap();
    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let to_nibbles = Nibbles::from_bytes_be(to_hashed.as_bytes()).unwrap();
    let to_second_nibbles = Nibbles::from_bytes_be(to_hashed_2.as_bytes()).unwrap();

    let beneficiary_account_before = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };
    let sender_balance_before = 1000000000000000000u64.into();
    let sender_account_before = AccountRlp {
        balance: sender_balance_before,
        ..AccountRlp::default()
    };
    let to_account_before = AccountRlp {
        ..AccountRlp::default()
    };
    let to_account_second_before = AccountRlp {
        code_hash,
        ..AccountRlp::default()
    };

    // In the first transaction, the sender account sends `txn_value` to `to_account`.
    let gas_price = 10;
    let txn_value = 0xau64;
    let mut state_trie_before = HashedPartialTrie::from(Node::Empty);
    state_trie_before.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_before).to_vec(),
    );
    state_trie_before.insert(sender_nibbles, rlp::encode(&sender_account_before).to_vec());
    state_trie_before.insert(to_nibbles, rlp::encode(&to_account_before).to_vec());
    state_trie_before.insert(
        to_second_nibbles,
        rlp::encode(&to_account_second_before).to_vec(),
    );

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries: vec![],
    };

    let txn = hex!("f85f800a82520894095e7baea6a6c7c4c2dfeb977efac326af552d870a8026a0122f370ed4023a6c253350c6bfb87d7d7eb2cd86447befee99e0a26b70baec20a07100ab1b3977f2b4571202b9f4b68850858caf5469222794600b5ce1cfb348ad");

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 0x03e8.into(),
        block_number: 0.into(),
        block_difficulty: 0x020000.into(),
        block_gaslimit: 0x445566u32.into(),
        block_chain_id: 1.into(),
        block_base_fee: 0xa.into(),
        block_gas_used: (22570 + 21000).into(),
        block_bloom: [
            0.into(),
            0.into(),
            U256::from_dec_str(
                "55213970774324510299479508399853534522527075462195808724319849722937344",
            )
            .unwrap(),
            U256::from_dec_str("1361129467683753853853498429727072845824").unwrap(),
            33554432.into(),
            U256::from_dec_str("9223372036854775808").unwrap(),
            U256::from_dec_str(
                "3618502788666131106986593281521497120414687020801267626233049500247285563392",
            )
            .unwrap(),
            U256::from_dec_str("2722259584404615024560450425766186844160").unwrap(),
        ],
    };

    let beneficiary_account_after = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };

    let sender_balance_after = sender_balance_before - gas_price * 21000 - txn_value;
    let sender_account_after = AccountRlp {
        balance: sender_balance_after,
        nonce: 1.into(),
        ..AccountRlp::default()
    };
    let to_account_after = AccountRlp {
        balance: txn_value.into(),
        ..AccountRlp::default()
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    let mut expected_state_trie_after = HashedPartialTrie::from(Node::Empty);
    expected_state_trie_after.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_after).to_vec(),
    );
    expected_state_trie_after.insert(sender_nibbles, rlp::encode(&sender_account_after).to_vec());
    expected_state_trie_after.insert(to_nibbles, rlp::encode(&to_account_after).to_vec());
    expected_state_trie_after.insert(
        to_second_nibbles,
        rlp::encode(&to_account_second_before).to_vec(),
    );

    // Compute new receipt trie.
    let mut receipts_trie = HashedPartialTrie::from(Node::Empty);
    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: 21000u64.into(),
        bloom: [0x00; 256].to_vec().into(),
        logs: vec![],
    };
    receipts_trie.insert(
        Nibbles::from_str("0x80").unwrap(),
        rlp::encode(&receipt_0).to_vec(),
    );

    let tries_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: HashedPartialTrie::from(Node::Empty).hash(),
        receipts_root: receipts_trie.clone().hash(),
    };

    let inputs_first = GenerationInputs {
        signed_txns: vec![txn.to_vec()],
        tries: tries_before,
        trie_roots_after: tries_after,
        contract_code,
        block_metadata: block_metadata.clone(),
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: 21000u64.into(),
        block_bloom_before: [0.into(); 8],
        block_bloom_after: [0.into(); 8],
        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
        addresses: vec![],
    };

    // Preprocess all circuits.
    let all_circuits = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        &[16..17, 11..13, 17..19, 14..15, 9..11, 12..13, 19..21],
        &config,
    );

    let mut timing = TimingTree::new("prove root first", log::Level::Info);
    let (root_proof_first, first_public_values) =
        all_circuits.prove_root(&all_stark, &config, inputs_first, &mut timing)?;

    timing.filter(Duration::from_millis(100)).print();
    all_circuits.verify_root(root_proof_first.clone())?;

    // The output bloom filter, gas used and transaction number are fed to the next transaction, so the two proofs can be correctly aggregated.
    let block_bloom_second = first_public_values.extra_block_data.block_bloom_after;
    let gas_used_second = first_public_values.extra_block_data.gas_used_after;

    // Prove second transaction. In this second transaction, the code with logs is executed.

    let state_trie_before = expected_state_trie_after;

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: Node::Empty.into(),
        receipts_trie: receipts_trie.clone(),
        storage_tries: vec![],
    };

    // Prove a transaction which carries out two LOG opcodes.
    let txn_gas_price = 10;
    let txn_2 = hex!("f860010a830186a094095e7baea6a6c7c4c2dfeb977efac326af552e89808025a04a223955b0bd3827e3740a9a427d0ea43beb5bafa44a0204bf0a3306c8219f7ba0502c32d78f233e9e7ce9f5df3b576556d5d49731e0678fd5a068cdf359557b5b");

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    // Update the state and receipt tries after the transaction, so that we have the correct expected tries:
    // Update accounts.
    let beneficiary_account_after = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };

    let sender_balance_after = sender_balance_after - gas_used * txn_gas_price;
    let sender_account_after = AccountRlp {
        balance: sender_balance_after,
        nonce: 2.into(),
        ..AccountRlp::default()
    };
    let balance_after = to_account_after.balance;
    let to_account_after = AccountRlp {
        balance: balance_after,
        ..AccountRlp::default()
    };
    let to_account_second_after = AccountRlp {
        balance: to_account_second_before.balance,
        code_hash,
        ..AccountRlp::default()
    };

    // Update the receipt trie.
    let first_log = LogRlp {
        address: to.into(),
        topics: vec![],
        data: Bytes::new(),
    };

    let second_log = LogRlp {
        address: to.into(),
        topics: vec![
            hex!("0000000000000000000000000000000000000000000000000000000000000062").into(), // dec: 98
            hex!("0000000000000000000000000000000000000000000000000000000000000063").into(), // dec: 99
        ],
        data: hex!("a1b2c3d4e5").to_vec().into(),
    };

    let receipt = LegacyReceiptRlp {
        status: true,
        cum_gas_used: (22570 + 21000).into(),
        bloom: hex!("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000001000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000800000000000000008000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000800002000000000000000000000000000").to_vec().into(),
        logs: vec![first_log, second_log],
    };

    let receipt_nibbles = Nibbles::from_str("0x01").unwrap(); // RLP(1) = 0x1

    receipts_trie.insert(receipt_nibbles, rlp::encode(&receipt).to_vec());

    // Update the state trie.
    let mut expected_state_trie_after = HashedPartialTrie::from(Node::Empty);
    expected_state_trie_after.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_after).to_vec(),
    );
    expected_state_trie_after.insert(sender_nibbles, rlp::encode(&sender_account_after).to_vec());
    expected_state_trie_after.insert(to_nibbles, rlp::encode(&to_account_after).to_vec());
    expected_state_trie_after.insert(
        to_second_nibbles,
        rlp::encode(&to_account_second_after).to_vec(),
    );

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: HashedPartialTrie::from(Node::Empty).hash(),
        receipts_root: receipts_trie.hash(),
    };

    let block_bloom_final = [
        0.into(),
        0.into(),
        U256::from_dec_str(
            "55213970774324510299479508399853534522527075462195808724319849722937344",
        )
        .unwrap(),
        U256::from_dec_str("1361129467683753853853498429727072845824").unwrap(),
        U256::from_dec_str("33554432").unwrap(),
        U256::from_dec_str("9223372036854775808").unwrap(),
        U256::from_dec_str(
            "3618502788666131106986593281521497120414687020801267626233049500247285563392",
        )
        .unwrap(),
        U256::from_dec_str("2722259584404615024560450425766186844160").unwrap(),
    ];
    let inputs = GenerationInputs {
        signed_txns: vec![txn_2.to_vec()],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        block_metadata,
        txn_number_before: 1.into(),
        gas_used_before: gas_used_second,
        gas_used_after: receipt.cum_gas_used,
        block_bloom_before: block_bloom_second,
        block_bloom_after: block_bloom_final,
        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
        addresses: vec![],
    };

    let mut timing = TimingTree::new("prove root second", log::Level::Info);
    let (root_proof, public_values) =
        all_circuits.prove_root(&all_stark, &config, inputs, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    all_circuits.verify_root(root_proof.clone())?;

    // Update public values for the aggregation.
    let agg_public_values = PublicValues {
        trie_roots_before: first_public_values.trie_roots_before,
        trie_roots_after: public_values.trie_roots_after,
        extra_block_data: ExtraBlockData {
            txn_number_before: first_public_values.extra_block_data.txn_number_before,
            txn_number_after: public_values.extra_block_data.txn_number_after,
            gas_used_before: first_public_values.extra_block_data.gas_used_before,
            gas_used_after: public_values.extra_block_data.gas_used_after,
            block_bloom_before: first_public_values.extra_block_data.block_bloom_before,
            block_bloom_after: public_values.extra_block_data.block_bloom_after,
        },
        block_metadata: public_values.block_metadata,
        block_hashes: public_values.block_hashes,
    };

    // We can duplicate the proofs here because the state hasn't mutated.
    let (agg_proof, updated_agg_public_values) = all_circuits.prove_aggregation(
        false,
        &root_proof_first,
        false,
        &root_proof,
        agg_public_values,
    )?;
    all_circuits.verify_aggregation(&agg_proof)?;
    let (block_proof, _block_public_values) =
        all_circuits.prove_block(None, &agg_proof, updated_agg_public_values)?;
    all_circuits.verify_block(&block_proof)
}

/// Values taken from the block 1000000 of Goerli: https://goerli.etherscan.io/txs?block=1000000
#[test]
fn test_txn_and_receipt_trie_hash() -> anyhow::Result<()> {
    // This test checks that inserting into the transaction and receipt `HashedPartialTrie`s works as expected.
    let mut example_txn_trie = HashedPartialTrie::from(Node::Empty);

    // We consider two transactions, with one log each.
    let transaction_0 = LegacyTransactionRlp {
        nonce: 157823u64.into(),
        gas_price: 1000000000u64.into(),
        gas: 250000u64.into(),
        to: hex!("7ef66b77759e12Caf3dDB3E4AFF524E577C59D8D").into(),
        value: 0u64.into(),
        data: hex!("e9c6c176000000000000000000000000000000000000000000000000000000000000002a0000000000000000000000000000000000000000000000000000000000bd9fe6f7af1cc94b1aef2e0fa15f1b4baefa86eb60e78fa4bd082372a0a446d197fb58")
            .to_vec()
            .into(),
        v: 0x1c.into(),
        r: hex!("d0eeac4841caf7a894dd79e6e633efc2380553cdf8b786d1aa0b8a8dee0266f4").into(),
        s: hex!("740710eed9696c663510b7fb71a553112551121595a54ec6d2ec0afcec72a973").into(),
    };

    // Insert the first transaction into the transaction trie.
    example_txn_trie.insert(
        Nibbles::from_str("0x80").unwrap(), // RLP(0) = 0x80
        rlp::encode(&transaction_0).to_vec(),
    );

    let transaction_1 = LegacyTransactionRlp {
        nonce: 157824u64.into(),
        gas_price: 1000000000u64.into(),
        gas: 250000u64.into(),
        to: hex!("7ef66b77759e12Caf3dDB3E4AFF524E577C59D8D").into(),
        value: 0u64.into(),
        data: hex!("e9c6c176000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000004920eaa814f7df6a2203dc0e472e8828be95957c6b329fee8e2b1bb6f044c1eb4fc243")
            .to_vec()
            .into(),
        v: 0x1b.into(),
        r: hex!("a3ff39967683fc684dc7b857d6f62723e78804a14b091a058ad95cc1b8a0281f").into(),
        s: hex!("51b156e05f21f499fa1ae47ebf536b15a237208f1d4a62e33956b6b03cf47742").into(),
    };

    // Insert the second transaction into the transaction trie.
    example_txn_trie.insert(
        Nibbles::from_str("0x01").unwrap(),
        rlp::encode(&transaction_1).to_vec(),
    );

    // Receipts:
    let mut example_receipt_trie = HashedPartialTrie::from(Node::Empty);

    let log_0 = LogRlp {
        address: hex!("7ef66b77759e12Caf3dDB3E4AFF524E577C59D8D").into(),
        topics: vec![
            hex!("8a22ee899102a366ac8ad0495127319cb1ff2403cfae855f83a89cda1266674d").into(),
            hex!("000000000000000000000000000000000000000000000000000000000000002a").into(),
            hex!("0000000000000000000000000000000000000000000000000000000000bd9fe6").into(),
        ],
        data: hex!("f7af1cc94b1aef2e0fa15f1b4baefa86eb60e78fa4bd082372a0a446d197fb58")
            .to_vec()
            .into(),
    };

    let receipt_0 = LegacyReceiptRlp {
            status: true,
            cum_gas_used: 0x016e5bu64.into(),
            bloom: hex!("00000000000000000000000000000000000000000000000000800000000000000040000000005000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000000000000000080008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000020000000000008000000000000000000000000").to_vec().into(),
            logs: vec![log_0],
        };

    // Insert the first receipt into the receipt trie.
    example_receipt_trie.insert(
        Nibbles::from_str("0x80").unwrap(), // RLP(0) is 0x80
        rlp::encode(&receipt_0).to_vec(),
    );

    let log_1 = LogRlp {
        address: hex!("7ef66b77759e12Caf3dDB3E4AFF524E577C59D8D").into(),
        topics: vec![
            hex!("8a22ee899102a366ac8ad0495127319cb1ff2403cfae855f83a89cda1266674d").into(),
            hex!("0000000000000000000000000000000000000000000000000000000000000004").into(),
            hex!("00000000000000000000000000000000000000000000000000000000004920ea").into(),
        ],
        data: hex!("a814f7df6a2203dc0e472e8828be95957c6b329fee8e2b1bb6f044c1eb4fc243")
            .to_vec()
            .into(),
    };

    let receipt_1 = LegacyReceiptRlp {
            status: true,
            cum_gas_used: 0x02dcb6u64.into(),
            bloom: hex!("00000000000000000000000000000000000000000000000000800000000000000040000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000008000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000400000000000000000000000000000002000040000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000008000000000000000000000000").to_vec().into(),
            logs: vec![log_1],
        };

    // Insert the second receipt into the receipt trie.
    example_receipt_trie.insert(
        Nibbles::from_str("0x01").unwrap(),
        rlp::encode(&receipt_1).to_vec(),
    );

    // Check that the trie hashes are correct.
    assert_eq!(
        example_txn_trie.hash(),
        hex!("3ab7120d12e1fc07303508542602beb7eecfe8f262b83fd71eefe7d6205242ce").into()
    );

    assert_eq!(
        example_receipt_trie.hash(),
        hex!("da46cdd329bfedace32da95f2b344d314bc6f55f027d65f9f4ac04ee425e1f98").into()
    );

    Ok(())
}

#[test]
#[ignore] // Too slow to run on CI.
fn test_two_txn() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba");
    let sender = hex!("af1276cbb260bb13deddb4209ae99ae6e497f446");
    // Private key: DCDFF53B4F013DBCDC717F89FE3BF4D8B10512AAE282B48E01D7530470382701
    let to = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_state_key = keccak(sender);
    let to_hashed = keccak(to);

    let beneficiary_nibbles = Nibbles::from_bytes_be(beneficiary_state_key.as_bytes()).unwrap();
    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let to_nibbles = Nibbles::from_bytes_be(to_hashed.as_bytes()).unwrap();

    // Set accounts before the transaction.
    let beneficiary_account_before = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };

    let sender_balance_before = 50000000000000000u64;
    let sender_account_before = AccountRlp {
        balance: sender_balance_before.into(),
        ..AccountRlp::default()
    };
    let to_account_before = AccountRlp {
        ..AccountRlp::default()
    };

    // Initialize the state trie with three accounts.
    let mut state_trie_before = HashedPartialTrie::from(Node::Empty);
    state_trie_before.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_before).to_vec(),
    );
    state_trie_before.insert(sender_nibbles, rlp::encode(&sender_account_before).to_vec());
    state_trie_before.insert(to_nibbles, rlp::encode(&to_account_before).to_vec());

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries: vec![(to_hashed, Node::Empty.into())],
    };

    // Prove two simple transfers.
    let gas_price = 10;
    let txn_value = 0x11c37937e08000u64;
    let txn_0 = hex!("f866800a82520894095e7baea6a6c7c4c2dfeb977efac326af552d878711c37937e080008026a01fcd0ce88ac7600698a771f206df24b70e67981b6f107bd7c1c24ea94f113bcba00d87cc5c7afc2988e4ff200b5a0c7016b0d5498bbc692065ca983fcbbfe02555");
    let txn_1 = hex!("f866010a82520894095e7baea6a6c7c4c2dfeb977efac326af552d878711c37937e080008026a0d8123f5f537bd3a67283f67eb136f7accdfc4ef012cfbfd3fb1d0ac7fd01b96fa004666d9feef90a1eb568570374dd19977d4da231b289d769e6f95105c06fd672");

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 0x03e8.into(),
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_gaslimit: 0xffffffffu32.into(),
        block_chain_id: 1.into(),
        block_base_fee: 0xa.into(),
        block_gas_used: 0.into(),
        block_bloom: [0.into(); 8],
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);

    // Update accounts
    let beneficiary_account_after = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };

    let sender_balance_after = sender_balance_before - gas_price * 21000 * 2 - txn_value * 2;
    let sender_account_after = AccountRlp {
        balance: sender_balance_after.into(),
        nonce: 2.into(),
        ..AccountRlp::default()
    };
    let to_account_after = AccountRlp {
        balance: (2 * txn_value).into(),
        ..AccountRlp::default()
    };

    // Update the state trie.
    let mut expected_state_trie_after = HashedPartialTrie::from(Node::Empty);
    expected_state_trie_after.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_after).to_vec(),
    );
    expected_state_trie_after.insert(sender_nibbles, rlp::encode(&sender_account_after).to_vec());
    expected_state_trie_after.insert(to_nibbles, rlp::encode(&to_account_after).to_vec());

    // Compute new receipt trie.
    let mut receipts_trie = HashedPartialTrie::from(Node::Empty);

    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: 21000u64.into(),
        bloom: [0x00; 256].to_vec().into(),
        logs: vec![],
    };

    let receipt_1 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: 42000u64.into(),
        bloom: [0x00; 256].to_vec().into(),
        logs: vec![],
    };

    receipts_trie.insert(
        Nibbles::from_str("0x80").unwrap(),
        rlp::encode(&receipt_0).to_vec(),
    );

    receipts_trie.insert(
        Nibbles::from_str("0x01").unwrap(),
        rlp::encode(&receipt_1).to_vec(),
    );

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: HashedPartialTrie::from(Node::Empty).hash(),
        receipts_root: receipts_trie.hash(),
    };
    let inputs = GenerationInputs {
        signed_txns: vec![txn_0.to_vec(), txn_1.to_vec()],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        block_metadata,
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: 42000u64.into(),
        block_bloom_before: [0.into(); 8],
        block_bloom_after: [0.into(); 8],
        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
        addresses: vec![],
    };

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    // Assert trie roots.
    assert_eq!(
        proof.public_values.trie_roots_after.state_root,
        expected_state_trie_after.hash()
    );

    assert_eq!(
        proof.public_values.trie_roots_after.receipts_root,
        receipts_trie.hash()
    );

    verify_proof(&all_stark, proof, &config)
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
