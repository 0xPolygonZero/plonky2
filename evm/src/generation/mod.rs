use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::all_stark::AllStark;
use crate::cpu::bootstrap_kernel::generate_bootstrap_kernel;
use crate::cpu::columns::NUM_CPU_COLUMNS;
use crate::generation::state::GenerationState;
use crate::util::trace_rows_to_poly_values;

mod memory;
pub(crate) mod state;

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
        logic_ops: logic_inputs,
        ..
    } = state;
    assert_eq!(current_cpu_row, [F::ZERO; NUM_CPU_COLUMNS]);

    let cpu_trace = trace_rows_to_poly_values(cpu_rows);
    let keccak_trace = all_stark.keccak_stark.generate_trace(keccak_inputs);
    let logic_trace = all_stark.logic_stark.generate_trace(logic_inputs);
    let memory_trace = all_stark.memory_stark.generate_trace(memory.log);
    vec![cpu_trace, keccak_trace, logic_trace, memory_trace]
}

fn generate_txn<F: Field>(_state: &mut GenerationState<F>, _txn: &TransactionData) {
    todo!()
}
