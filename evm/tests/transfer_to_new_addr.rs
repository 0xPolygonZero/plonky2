use std::collections::HashMap;

use eth_trie_utils::partial_trie::PartialTrie;
use hex_literal::hex;
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

/// Test a simple token transfer to a new address.
#[test]
#[ignore] // TODO: Won't work until txn parsing, storage, etc. are implemented.
fn test_simple_transfer() -> anyhow::Result<()> {
    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let block_metadata = BlockMetadata::default();

    let txn = hex!("f85f050a82520894000000000000000000000000000000000000000064801ca0fa56df5d988638fad8798e5ef75a1e1125dc7fb55d2ac4bce25776a63f0c2967a02cb47a5579eb5f83a1cabe4662501c0059f1b58e60ef839a1b0da67af6b9fb38");

    let inputs = GenerationInputs {
        signed_txns: vec![txn.to_vec()],
        tries: TrieInputs {
            state_trie: PartialTrie::Empty,
            transactions_trie: PartialTrie::Empty,
            receipts_trie: PartialTrie::Empty,
            storage_tries: vec![],
        },
        contract_code: HashMap::new(),
        block_metadata,
    };

    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut TimingTree::default())?;

    verify_proof(all_stark, proof, &config)
}
