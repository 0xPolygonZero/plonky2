use std::collections::HashMap;

use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::{Address, BigEndianHash, H256};
use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use serde::{Deserialize, Serialize};
use GlobalMetadata::{
    ReceiptTrieRootDigestAfter, ReceiptTrieRootDigestBefore, StateTrieRootDigestAfter,
    StateTrieRootDigestBefore, TransactionTrieRootDigestAfter, TransactionTrieRootDigestBefore,
};

use crate::all_stark::{AllStark, NUM_TABLES};
use crate::config::StarkConfig;
use crate::cpu::bootstrap_kernel::generate_bootstrap_kernel;
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::generation::state::GenerationState;
use crate::memory::segments::Segment;
use crate::proof::{BlockMetadata, PublicValues, TrieRoots};
use crate::witness::memory::MemoryAddress;
use crate::witness::transition::transition;

pub mod mpt;
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
    let mut state = GenerationState::<F>::new(inputs.clone(), &KERNEL.code);

    generate_bootstrap_kernel::<F>(&mut state);

    timed!(timing, "simulate CPU", simulate_cpu(&mut state));

    log::info!(
        "Trace lengths (before padding): {:?}",
        state.traces.checkpoint()
    );

    let read_metadata = |field| {
        state.memory.get(MemoryAddress::new(
            0,
            Segment::GlobalMetadata,
            field as usize,
        ))
    };

    let trie_roots_before = TrieRoots {
        state_root: H256::from_uint(&read_metadata(StateTrieRootDigestBefore)),
        transactions_root: H256::from_uint(&read_metadata(TransactionTrieRootDigestBefore)),
        receipts_root: H256::from_uint(&read_metadata(ReceiptTrieRootDigestBefore)),
    };
    let trie_roots_after = TrieRoots {
        state_root: H256::from_uint(&read_metadata(StateTrieRootDigestAfter)),
        transactions_root: H256::from_uint(&read_metadata(TransactionTrieRootDigestAfter)),
        receipts_root: H256::from_uint(&read_metadata(ReceiptTrieRootDigestAfter)),
    };

    let public_values = PublicValues {
        trie_roots_before,
        trie_roots_after,
        block_metadata: inputs.block_metadata,
    };

    let tables = timed!(
        timing,
        "convert trace data to tables",
        state.traces.into_tables(all_stark, config, timing)
    );
    (tables, public_values)
}

fn simulate_cpu<F: RichField + Extendable<D>, const D: usize>(state: &mut GenerationState<F>) {
    let halt_pc0 = KERNEL.global_labels["halt_pc0"];
    let halt_pc1 = KERNEL.global_labels["halt_pc1"];

    let mut already_in_halt_loop = false;
    loop {
        // If we've reached the kernel's halt routine, and our trace length is a power of 2, stop.
        let pc = state.registers.program_counter;
        let in_halt_loop = pc == halt_pc0 || pc == halt_pc1;
        if in_halt_loop && !already_in_halt_loop {
            log::info!("CPU halted after {} cycles", state.traces.clock());
        }
        already_in_halt_loop |= in_halt_loop;

        transition(state);

        if already_in_halt_loop && state.traces.clock().is_power_of_two() {
            log::info!("CPU trace padded to {} cycles", state.traces.clock());
            break;
        }
    }
}
