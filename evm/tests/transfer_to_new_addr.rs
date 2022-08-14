use hex_literal::hex;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::generation::{generate_traces, TransactionData};
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

    let txn = TransactionData {
        signed_txn: hex!("f85f050a82520894000000000000000000000000000000000000000064801ca0fa56df5d988638fad8798e5ef75a1e1125dc7fb55d2ac4bce25776a63f0c2967a02cb47a5579eb5f83a1cabe4662501c0059f1b58e60ef839a1b0da67af6b9fb38").to_vec(),
        trie_proofs: vec![
            vec![
                hex!("f874a1202f93d0dfb1562c03c825a33eec4438e468c17fff649ae844c004065985ae2945b850f84e058a152d02c7e14af6800000a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470").to_vec(),
            ],
            vec![
                hex!("f8518080a0d36b8b6b60021940d5553689fb33e5d45e649dd8f4f211d26566238a83169da58080a0c62aa627943b70321f89a8b2fea274ecd47116e62042077dcdc0bdca7c1f66738080808080808080808080").to_vec(),
                hex!("f873a03f93d0dfb1562c03c825a33eec4438e468c17fff649ae844c004065985ae2945b850f84e068a152d02c7e14af67ccb4ca056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421a0c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470").to_vec(),
            ],
        ]
    };

    let traces = generate_traces(&all_stark, &[txn]);

    let config = StarkConfig::standard_fast_config();
    let proof = prove::<F, C, D>(
        &all_stark,
        &config,
        traces,
        vec![vec![]; 4],
        &mut TimingTree::default(),
    )?;

    verify_proof(all_stark, proof, &config)
}
