use std::collections::HashMap;

use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::BlockMetadata;
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;

type F = GoldilocksField;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;

/// Execute the empty list of transactions, i.e. a no-op.
#[test]
#[ignore] // TODO: Won't work until witness generation logic is finished.
fn test_empty_txn_list() -> anyhow::Result<()> {
    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let block_metadata = BlockMetadata::default();

    let state_trie = PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 5,
            packed: 0xABCDE.into(),
        },
        value: vec![1, 2, 3],
    };
    let transactions_trie = PartialTrie::Empty;
    let receipts_trie = PartialTrie::Empty;
    let storage_tries = vec![];

    let state_trie_root = state_trie.calc_hash();
    let txns_trie_root = transactions_trie.calc_hash();
    let receipts_trie_root = receipts_trie.calc_hash();

    let inputs = GenerationInputs {
        signed_txns: vec![],
        tries: TrieInputs {
            state_trie,
            transactions_trie,
            receipts_trie,
            storage_tries,
        },
        contract_code: HashMap::new(),
        block_metadata,
    };

    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut TimingTree::default())?;
    assert_eq!(
        proof.public_values.trie_roots_before.state_root,
        state_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_after.state_root,
        state_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_before.transactions_root,
        txns_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_after.transactions_root,
        txns_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_before.receipts_root,
        receipts_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_after.receipts_root,
        receipts_trie_root
    );

    verify_proof(all_stark, proof, &config)
}
