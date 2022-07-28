use ethereum_types::U256;
use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::all_stark::AllStark;
use crate::cpu::bootstrap_kernel::generate_bootstrap_kernel;
use crate::cpu::columns::NUM_CPU_COLUMNS;
use crate::generation::state::GenerationState;
use crate::util::trace_rows_to_poly_values;

pub(crate) mod memory;
pub(crate) mod state;

/// A piece of data which has been encoded using Recursive Length Prefix (RLP) serialization.
/// See https://ethereum.org/en/developers/docs/data-structures-and-encoding/rlp/
pub type RlpBlob = Vec<u8>;

/// Merkle proofs are encoded using an RLP blob for each node in the path.
pub type RlpMerkleProof = Vec<RlpBlob>;

#[allow(unused)] // TODO: Should be used soon.
pub struct TransactionData {
    pub signed_txn: Vec<u8>,

    /// A Merkle proof for each interaction with the state trie, ordered chronologically.
    pub trie_proofs: Vec<RlpMerkleProof>,
}

#[allow(unused)] // TODO: Should be used soon.
pub fn generate_traces<F: RichField + Extendable<D>, const D: usize>(
    all_stark: &AllStark<F, D>,
    txns: &[TransactionData],
) -> Vec<Vec<PolynomialValues<F>>> {
    let mut state = GenerationState::<F>::default();

    generate_bootstrap_kernel::<F>(&mut state);

    for txn in txns {
        generate_txn(&mut state, txn);
    }

    let GenerationState {
        cpu_rows,
        current_cpu_row,
        memory,
        keccak_inputs,
        logic_ops,
        prover_inputs,
        ..
    } = state;
    assert_eq!(current_cpu_row, [F::ZERO; NUM_CPU_COLUMNS].into());
    assert_eq!(prover_inputs, vec![], "Not all prover inputs were consumed");

    let cpu_trace = trace_rows_to_poly_values(cpu_rows);
    let keccak_trace = all_stark.keccak_stark.generate_trace(keccak_inputs);
    let logic_trace = all_stark.logic_stark.generate_trace(logic_ops);
    let memory_trace = all_stark.memory_stark.generate_trace(memory.log);
    vec![cpu_trace, keccak_trace, logic_trace, memory_trace]
}

fn generate_txn<F: Field>(state: &mut GenerationState<F>, txn: &TransactionData) {
    // TODO: Add transaction RLP to prover_input.

    // Supply Merkle trie proofs as prover inputs.
    for proof in &txn.trie_proofs {
        let proof = proof
            .iter()
            .flat_map(|node_rlp| node_rlp.iter().map(|byte| U256::from(*byte)));
        state.prover_inputs.extend(proof);
    }
}
