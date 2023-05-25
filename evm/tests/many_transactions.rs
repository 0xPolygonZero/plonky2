#![allow(clippy::upper_case_acronyms)]

//use core::slice::SlicePattern;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use bytes::Bytes;
use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::cpu::kernel::opcodes::{get_opcode, get_push_opcode};
use plonky2_evm::fixed_recursive_verifier::AllRecursiveCircuits;
use plonky2_evm::generation::mpt::{AccountRlp, LegacyReceiptRlp, LogRlp};
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockMetadata, ExtraBlockData, PublicValues, TrieRoots};
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;
use plonky2_evm::Node;

type F = GoldilocksField;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;

/// Test the validity of the validty of the state an transaction tries after processing
/// four transactions, where only the first one is valid and other three abort.  
#[test]
#[ignore] // Too slow to run on CI.
fn test_four_transactions() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    let sender = hex!("2c7536e3605d9c16a7a3d7b1898e529396a65c23");
    let to = hex!("a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_state_key = keccak(sender);
    let to_state_key = keccak(to);

    let beneficiary_nibbles = Nibbles::from_bytes_be(beneficiary_state_key.as_bytes()).unwrap();
    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let to_nibbles = Nibbles::from_bytes_be(to_state_key.as_bytes()).unwrap();

    let push1 = get_push_opcode(1);
    let add = get_opcode("ADD");
    let stop = get_opcode("STOP");
    let code = [push1, 3, push1, 4, add, stop];
    let code_gas = 3 + 3 + 3;
    let code_hash = keccak(code);

    let beneficiary_account_before = AccountRlp::default();
    let sender_account_before = AccountRlp {
        nonce: 5.into(),

        balance: eth_to_wei(100_000.into()),

        ..AccountRlp::default()
    };
    let to_account_before = AccountRlp {
        code_hash,
        ..AccountRlp::default()
    };

    let state_trie_before = {
        let mut children = core::array::from_fn(|_| Node::Empty.into());
        children[sender_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: sender_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&sender_account_before).to_vec(),
        }
        .into();
        children[to_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: to_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&to_account_before).to_vec(),
        }
        .into();
        Node::Branch {
            children,
            value: vec![],
        }
    }
    .into();

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: Node::Empty.into(),

        receipts_trie: Node::Empty.into(),

        storage_tries: vec![],
    };

    // Generated using a little py-evm script.

    let txn1 = hex!("f861050a8255f094a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0648242421ba02c89eb757d9deeb1f5b3859a9d4d679951ef610ac47ad4608dc142beb1b7e313a05af7e9fbab825455d36c36c7f4cfcafbeafa9a77bdff936b52afb36d4fe4bcdd");
    let value = U256::from(100u32);
    let txn2: [u8; 101] = hex!("f863800a83061a8094095e7baea6a6c7c4c2dfeb977efac326af552d87830186a0801ba0ffb600e63115a7362e7811894a91d8ba4330e526f22121c994c4692035dfdfd5a06198379fcac8de3dbfac48b165df4bf88e2088f294b61efb9a65fe2281c76e16");
    let txn3 = hex!("f861800a8405f5e10094100000000000000000000000000000000000000080801ba07e09e26678ed4fac08a249ebe8ed680bf9051a5e14ad223e4b2b9d26e0208f37a05f6e3f188e3e6eab7d7d3b6568f5eac7d687b08d307d3154ccd8c87b4630509b");
    let txn4 = hex!("f866800a82520894095e7baea6a6c7c4c2dfeb977efac326af552d878711c37937e080008026a01fcd0ce88ac7600698a771f206df24b70e67981b6f107bd7c1c24ea94f113bcba00d87cc5c7afc2988e4ff200b5a0c7016b0d5498bbc692065ca983fcbbfe02555");

    let txdata_gas = 2 * 16;
    let gas_used = 21_000 + code_gas + txdata_gas;

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),

        block_timestamp: 0x03e8.into(),

        block_number: 1.into(),

        block_difficulty: 0x020000.into(),

        block_gaslimit: 0x445566u64.into(),

        block_chain_id: 1.into(),

        block_gas_used: gas_used.into(),

        ..BlockMetadata::default()
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    // Update trie roots after the 4 transactions.
    // State trie.
    let expected_state_trie_after: HashedPartialTrie = {
        let beneficiary_account_after = AccountRlp {
            balance: beneficiary_account_before.balance + gas_used * 10,
            ..beneficiary_account_before
        };
        let sender_account_after = AccountRlp {
            balance: sender_account_before.balance - value - gas_used * 10,
            nonce: sender_account_before.nonce + 1,
            ..sender_account_before
        };
        let to_account_after = AccountRlp {
            balance: to_account_before.balance + value,
            ..to_account_before
        };

        let mut children = core::array::from_fn(|_| Node::Empty.into());
        children[beneficiary_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: beneficiary_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&beneficiary_account_after).to_vec(),
        }
        .into();
        children[sender_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: sender_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&sender_account_after).to_vec(),
        }
        .into();
        children[to_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: to_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&to_account_after).to_vec(),
        }
        .into();
        Node::Branch {
            children,
            value: vec![],
        }
    }
    .into();

    // Transactions trie.
    let mut expected_transactions_trie: HashedPartialTrie = Node::Leaf {
        nibbles: Nibbles::from_str("0x80").unwrap(),
        value: txn1.to_vec(),
    }
    .into();
    expected_transactions_trie.insert(Nibbles::from_str("0x01").unwrap(), txn2.to_vec());
    expected_transactions_trie.insert(Nibbles::from_str("0x02").unwrap(), txn3.to_vec());
    expected_transactions_trie.insert(Nibbles::from_str("0x03").unwrap(), txn4.to_vec());

    // Receipts trie.
    let mut receipts_trie = HashedPartialTrie::from(Node::Empty);
    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: gas_used.into(),
        bloom: [0x00; 256].to_vec().into(),
        logs: vec![],
    };
    let receipt_1 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: gas_used.into(),
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
    receipts_trie.insert(
        Nibbles::from_str("0x02").unwrap(),
        rlp::encode(&receipt_1).to_vec(),
    );
    receipts_trie.insert(
        Nibbles::from_str("0x03").unwrap(),
        rlp::encode(&receipt_1).to_vec(),
    );

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: expected_transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };
    let inputs = GenerationInputs {
        signed_txns: vec![txn1.to_vec(), txn2.to_vec(), txn3.to_vec(), txn4.to_vec()],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        block_metadata: block_metadata.clone(),
        addresses: vec![],
        block_bloom_before: [0.into(); 8],
        gas_used_before: 0.into(),
        gas_used_after: gas_used.into(),
        txn_number_before: 0.into(),
        block_bloom_after: [0.into(); 8],
    };

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    verify_proof(&all_stark, proof, &config)
}

/// Tests proving two transactions, one of which with logs, and aggregating them.
#[test]
#[ignore] // Too slow to run on CI.
fn test_aggreg_txns_and_receipts() -> anyhow::Result<()> {
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

    // First txn
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
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_gaslimit: 0x445566u32.into(),
        block_chain_id: 1.into(),
        block_base_fee: 0xa.into(),
        block_gas_used: (22570 + 2 * 21000).into(),
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

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    // Compute trie roots after the trransaction.
    // State trie.
    let beneficiary_account_after = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };

    let sender_balance_after = sender_balance_before - gas_price * 21000 - txn_value;
    let sender_account_after = AccountRlp {
        balance: sender_balance_after.into(),
        nonce: 1.into(),
        ..AccountRlp::default()
    };
    let to_account_after = AccountRlp {
        balance: txn_value.into(),
        ..AccountRlp::default()
    };
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

    // Transaction trie.
    let mut expected_transactions_trie: HashedPartialTrie = Node::Leaf {
        nibbles: Nibbles::from_str("0x80").unwrap(),
        value: txn.to_vec(),
    }
    .into();

    // Receipt trie.
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

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: expected_transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };

    let inputs_first = GenerationInputs {
        signed_txns: vec![txn.to_vec()],
        tries: tries_before,
        trie_roots_after,
        contract_code: contract_code.clone(),
        block_metadata: block_metadata.clone(),
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: 21000.into(),
        block_bloom_before: [0.into(); 8],
        block_bloom_after: [0.into(); 8],
        addresses: vec![],
    };

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs_first.clone(), &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    // The output bloom filter, gas used and transaction number are fed to the next transaction, so the two proofs can be correctly aggregated.
    let block_bloom_second = proof.public_values.extra_block_data.block_bloom_after;
    let txn_second = proof.public_values.extra_block_data.txn_number_after;
    let gas_used_second = proof.public_values.extra_block_data.gas_used_after;

    verify_proof(&all_stark, proof.clone(), &config)?;

    // Create the aggregation circuits
    let all_circuits_first = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        &[9..17, 9..19, 9..15, 9..11, 9..14, 9..21],
        &config,
    );

    let mut timing = TimingTree::new("prove", log::Level::Info);
    let (root_proof_first, first_public_values) =
        all_circuits_first.prove_root(&all_stark, &config, inputs_first, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    // Prove second transaction. In this second transaction, the code with logs is executed.

    let state_trie_before = expected_state_trie_after;

    let transactions_trie_before = expected_transactions_trie.clone();

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: transactions_trie_before,
        receipts_trie: receipts_trie.clone(),
        storage_tries: vec![],
    };

    // Update the state and receipt tries after the second and third transactions, so that we have the correct expected tries:
    // Update accounts
    let beneficiary_account_after = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };

    let sender_balance_after =
        sender_balance_after - gas_used * gas_price - gas_price * 21000 - txn_value;
    let sender_account_after = AccountRlp {
        balance: sender_balance_after.into(),
        nonce: 2.into(),
        ..AccountRlp::default()
    };
    let balance_after = to_account_after.balance + txn_value;
    let to_account_after = AccountRlp {
        balance: balance_after.into(),
        ..AccountRlp::default()
    };
    let to_account_second_after = AccountRlp {
        balance: to_account_second_before.balance.into(),
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
    let receipt_2 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: (22570 + 2 * 21000).into(),
        bloom: [0x00; 256].to_vec().into(),
        logs: vec![],
    };
    receipts_trie.insert(
        Nibbles::from_str("0x02").unwrap(),
        rlp::encode(&receipt_2).to_vec(),
    );

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

    // Transaction which carries out two LOG opcodes.
    let txn_2 = hex!("f860010a830186a094095e7baea6a6c7c4c2dfeb977efac326af552e89808025a04a223955b0bd3827e3740a9a427d0ea43beb5bafa44a0204bf0a3306c8219f7ba0502c32d78f233e9e7ce9f5df3b576556d5d49731e0678fd5a068cdf359557b5b");
    // Update transactions trie
    expected_transactions_trie.insert(Nibbles::from_str("0x01").unwrap(), txn_2.to_vec());
    expected_transactions_trie.insert(Nibbles::from_str("0x02").unwrap(), txn.to_vec());

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: expected_transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };

    let inputs = GenerationInputs {
        signed_txns: vec![txn_2.to_vec()],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        block_metadata: block_metadata.clone(),
        txn_number_before: txn_second,
        gas_used_before: gas_used_second,
        gas_used_after: gas_used_second + U256::from(21000 + 22570),
        block_bloom_before: block_bloom_second,
        block_bloom_after: block_metadata.block_bloom,
        addresses: vec![],
    };
    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs.clone(), &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    verify_proof(&all_stark, proof, &config)?;
    let config = StarkConfig::standard_fast_config();
    let all_circuits = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        &[9..17, 9..19, 9..15, 9..11, 9..14, 9..21],
        &config,
    );
    let mut timing = TimingTree::new("prove", log::Level::Info);
    let (root_proof, public_values) =
        all_circuits.prove_root(&all_stark, &config, inputs, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    all_circuits.verify_root(root_proof.clone())?;

    let aggreg_public_values = PublicValues {
        trie_roots_before: first_public_values.trie_roots_before,
        trie_roots_after: public_values.trie_roots_after,
        block_metadata: first_public_values.block_metadata,
        extra_block_data: ExtraBlockData {
            txn_number_before: first_public_values.extra_block_data.txn_number_before,
            txn_number_after: public_values.extra_block_data.txn_number_after,
            gas_used_before: first_public_values.extra_block_data.gas_used_before,
            gas_used_after: public_values.extra_block_data.gas_used_after,
            block_bloom_before: first_public_values.extra_block_data.block_bloom_before,
            block_bloom_after: public_values.extra_block_data.block_bloom_after,
        },
    };
    // We can duplicate the proofs here because the state hasn't mutated.
    let (agg_proof, post_aggreg_public_values) = all_circuits_first.prove_aggregation(
        false,
        &root_proof_first,
        false,
        &root_proof,
        aggreg_public_values,
    )?;
    all_circuits_first.verify_aggregation(&agg_proof)?;
    let (block_proof, _block_public_values) =
        all_circuits.prove_block(None, &agg_proof, post_aggreg_public_values)?;
    all_circuits.verify_block(&block_proof)
}

fn eth_to_wei(eth: U256) -> U256 {
    // 1 ether = 10^18 wei.
    eth * U256::from(10).pow(18.into())
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
