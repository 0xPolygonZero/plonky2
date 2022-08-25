use ethereum_types::Address;
use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use crate::all_stark::AllStark;
use crate::cpu::bootstrap_kernel::generate_bootstrap_kernel;
use crate::cpu::columns::NUM_CPU_COLUMNS;
use crate::generation::partial_trie::PartialTrie;
use crate::generation::state::GenerationState;
use crate::util::trace_rows_to_poly_values;

pub(crate) mod memory;
pub mod partial_trie;
pub(crate) mod state;

#[allow(unused)] // TODO: Should be used soon.
pub struct TransactionData {
    pub signed_txn: Vec<u8>,

    /// A partial version of the state trie prior to this transaction. It should include all nodes
    /// that will be accessed by this transaction.
    pub state_trie: PartialTrie,

    /// A partial version of the transaction trie prior to this transaction. It should include all
    /// nodes that will be accessed by this transaction.
    pub transaction_trie: PartialTrie,

    /// A partial version of the receipt trie prior to this transaction. It should include all nodes
    /// that will be accessed by this transaction.
    pub receipt_trie: PartialTrie,

    /// A partial version of each storage trie prior to this transaction. It should include all
    /// storage tries, and nodes therein, that will be accessed by this transaction.
    pub storage_tries: Vec<(Address, PartialTrie)>,
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
        ..
    } = state;
    assert_eq!(current_cpu_row, [F::ZERO; NUM_CPU_COLUMNS].into());

    let cpu_trace = trace_rows_to_poly_values(cpu_rows);
    let keccak_trace = all_stark.keccak_stark.generate_trace(keccak_inputs);
    let logic_trace = all_stark.logic_stark.generate_trace(logic_ops);
    let memory_trace = all_stark.memory_stark.generate_trace(memory.log);
    vec![cpu_trace, keccak_trace, logic_trace, memory_trace]
}

fn generate_txn<F: Field>(_state: &mut GenerationState<F>, _txn: &TransactionData) {
    // TODO
}
