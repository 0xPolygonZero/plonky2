use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2::plonk::proof::ProofWithPublicInputs;
use plonky2::util::timing::TimingTree;

use crate::all_stark::AllStark;
use crate::config::StarkConfig;
use crate::fixed_recursive_verifier::AllRecursiveCircuits;
use crate::generation::mpt::{AccountRlp, LegacyReceiptRlp, LogRlp};
use crate::generation::{GenerationInputs, TrieInputs};
use crate::proof::{BlockHashes, BlockMetadata, ExtraBlockData, PublicValues, TrieRoots};
use crate::Node;

// Taken from log_opcode test.
pub fn get_sample_circuits_and_proof<F, C, const D: usize>() -> Result<(
    AllRecursiveCircuits<F, C, D>,
    ProofWithPublicInputs<F, C, D>,
)>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    C::Hasher: AlgebraicHasher<F>,
{
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
    let genesis_state_trie_root =  state_trie_before.hash();

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
        block_random: Default::default(),
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

    let mut transactions_trie: HashedPartialTrie = Node::Leaf {
        nibbles: Nibbles::from_str("0x80").unwrap(),
        value: txn.to_vec(),
    }
    .into();

    let tries_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.clone().hash(),
    };

    let inputs_first = GenerationInputs {
        signed_txns: vec![txn.to_vec()],
        tries: tries_before,
        trie_roots_after: tries_after,
        contract_code,
        genesis_state_trie_root,
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

    let timing = TimingTree::new("verify root first", log::Level::Info);
    timing.filter(Duration::from_millis(100)).print();
    all_circuits.verify_root(root_proof_first.clone())?;

    // The output bloom filter, gas used and transaction number are fed to the next transaction, so the two proofs can be correctly aggregated.
    let block_bloom_second = first_public_values.extra_block_data.block_bloom_after;
    let gas_used_second = first_public_values.extra_block_data.gas_used_after;

    // Prove second transaction. In this second transaction, the code with logs is executed.

    let state_trie_before = expected_state_trie_after;

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: transactions_trie.clone(),
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

    transactions_trie.insert(Nibbles::from_str("0x01").unwrap(), txn_2.to_vec());

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: transactions_trie.hash(),
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
        genesis_state_trie_root,
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
            genesis_state_trie_root: first_public_values.extra_block_data.genesis_state_trie_root,
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
    all_circuits.verify_block(&block_proof)?;

    Ok((all_circuits, block_proof))
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::plonk::config::PoseidonGoldilocksConfig;

    use super::get_sample_circuits_and_proof;

    type F = GoldilocksField;
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;

    #[test]
    fn test_get_sample_circuits_and_proof() {
        let (all_circuits, block_proof) = get_sample_circuits_and_proof::<F, C, D>().unwrap();
        all_circuits.verify_block(&block_proof).unwrap();
    }
}
