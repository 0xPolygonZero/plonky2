use std::collections::HashMap;

use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::{Address, BigEndianHash, H256};
use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::util::timing::TimingTree;
use serde::{Deserialize, Serialize};

use crate::all_stark::{AllStark, NUM_TABLES};
use crate::config::StarkConfig;
use crate::cpu::bootstrap_kernel::generate_bootstrap_kernel;
use crate::cpu::columns::NUM_CPU_COLUMNS;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::generation::state::GenerationState;
use crate::memory::segments::Segment;
use crate::memory::NUM_CHANNELS;
use crate::proof::{BlockMetadata, PublicValues, TrieRoots};
use crate::util::trace_rows_to_poly_values;

pub(crate) mod memory;
pub(crate) mod mpt;
pub(crate) mod prover_input;
pub(crate) mod rlp;
pub(crate) mod state;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Inputs needed for trace generation.
pub struct GenerationInputs {
    pub signed_txns: Vec<Vec<u8>>,

    pub tries: TrieInputs,

    /// Mapping between smart contract code hashes and the contract byte code.
    /// All account smart contracts that are invoked will have an entry present.
    pub contract_code: HashMap<H256, Vec<u8>>,

    pub block_metadata: BlockMetadata,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct TrieInputs {
    /// A partial version of the state trie prior to these transactions. It should include all nodes
    /// that will be accessed by these transactions.
    pub state_trie: PartialTrie,

    /// A partial version of the transaction trie prior to these transactions. It should include all
    /// nodes that will be accessed by these transactions.
    pub transactions_trie: PartialTrie,

    /// A partial version of the receipt trie prior to these transactions. It should include all nodes
    /// that will be accessed by these transactions.
    pub receipts_trie: PartialTrie,

    /// A partial version of each storage trie prior to these transactions. It should include all
    /// storage tries, and nodes therein, that will be accessed by these transactions.
    pub storage_tries: Vec<(Address, PartialTrie)>,
}

pub(crate) fn generate_traces<F: RichField + Extendable<D>, const D: usize>(
    all_stark: &AllStark<F, D>,
    inputs: GenerationInputs,
    config: &StarkConfig,
    timing: &mut TimingTree,
) -> ([Vec<PolynomialValues<F>>; NUM_TABLES], PublicValues) {
    let mut state = GenerationState::<F>::new(inputs.clone());

    generate_bootstrap_kernel::<F>(&mut state);

    for txn in &inputs.signed_txns {
        generate_txn(&mut state, txn);
    }

    // TODO: Pad to a power of two, ending in the `halt` kernel function.

    let cpu_rows = state.cpu_rows.len();
    let mem_end_timestamp = cpu_rows * NUM_CHANNELS;
    let mut read_metadata = |field| {
        state.get_mem(
            0,
            Segment::GlobalMetadata,
            field as usize,
            mem_end_timestamp,
        )
    };

    let trie_roots_before = TrieRoots {
        state_root: H256::from_uint(&read_metadata(GlobalMetadata::StateTrieRootDigestBefore)),
        transactions_root: H256::from_uint(&read_metadata(
            GlobalMetadata::TransactionTrieRootDigestBefore,
        )),
        receipts_root: H256::from_uint(&read_metadata(GlobalMetadata::ReceiptTrieRootDigestBefore)),
    };
    let trie_roots_after = TrieRoots {
        state_root: H256::from_uint(&read_metadata(GlobalMetadata::StateTrieRootDigestAfter)),
        transactions_root: H256::from_uint(&read_metadata(
            GlobalMetadata::TransactionTrieRootDigestAfter,
        )),
        receipts_root: H256::from_uint(&read_metadata(GlobalMetadata::ReceiptTrieRootDigestAfter)),
    };

    let GenerationState {
        cpu_rows,
        current_cpu_row,
        memory,
        keccak_inputs,
        keccak_memory_inputs,
        logic_ops,
        ..
    } = state;
    assert_eq!(current_cpu_row, [F::ZERO; NUM_CPU_COLUMNS].into());

    let cpu_trace = trace_rows_to_poly_values(cpu_rows);
    let keccak_trace = all_stark.keccak_stark.generate_trace(keccak_inputs, timing);
    let keccak_memory_trace = all_stark.keccak_memory_stark.generate_trace(
        keccak_memory_inputs,
        config.fri_config.num_cap_elements(),
        timing,
    );
    let logic_trace = all_stark.logic_stark.generate_trace(logic_ops, timing);
    let memory_trace = all_stark.memory_stark.generate_trace(memory.log, timing);
    let traces = [
        cpu_trace,
        keccak_trace,
        keccak_memory_trace,
        logic_trace,
        memory_trace,
    ];

    let public_values = PublicValues {
        trie_roots_before,
        trie_roots_after,
        block_metadata: inputs.block_metadata,
    };

    (traces, public_values)
}

fn generate_txn<F: Field>(_state: &mut GenerationState<F>, _signed_txn: &[u8]) {
    // TODO
}
