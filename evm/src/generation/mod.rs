use std::collections::HashMap;

use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H256, U256};
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
use crate::generation::outputs::{get_outputs, GenerationOutputs};
use crate::generation::state::GenerationState;
use crate::memory::segments::Segment;
use crate::proof::{BlockMetadata, PublicValues, TrieRoots};
use crate::witness::memory::{MemoryAddress, MemoryChannel};
use crate::witness::transition::transition;

pub mod mpt;
pub mod outputs;
pub(crate) mod prover_input;
pub(crate) mod rlp;
pub(crate) mod state;
mod trie_extractor;

use crate::witness::util::mem_write_log;

/// Inputs needed for trace generation.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct GenerationInputs {
    pub signed_txns: Vec<Vec<u8>>,
    pub tries: TrieInputs,
    /// Expected trie roots after the transactions are executed.
    pub trie_roots_after: TrieRoots,

    /// Mapping between smart contract code hashes and the contract byte code.
    /// All account smart contracts that are invoked will have an entry present.
    pub contract_code: HashMap<H256, Vec<u8>>,

    pub block_metadata: BlockMetadata,

    /// A list of known addresses in the input state trie (which itself doesn't hold addresses,
    /// only state keys). This is only useful for debugging, so that we can return addresses in the
    /// post-state rather than state keys. (See `GenerationOutputs`, and in particular
    /// `AddressOrStateKey`.) If the caller is not interested in the post-state, this can be left
    /// empty.
    pub addresses: Vec<Address>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct TrieInputs {
    /// A partial version of the state trie prior to these transactions. It should include all nodes
    /// that will be accessed by these transactions.
    pub state_trie: HashedPartialTrie,

    /// A partial version of the transaction trie prior to these transactions. It should include all
    /// nodes that will be accessed by these transactions.
    pub transactions_trie: HashedPartialTrie,

    /// A partial version of the receipt trie prior to these transactions. It should include all nodes
    /// that will be accessed by these transactions.
    pub receipts_trie: HashedPartialTrie,

    /// A partial version of each storage trie prior to these transactions. It should include all
    /// storage tries, and nodes therein, that will be accessed by these transactions.
    pub storage_tries: Vec<(H256, HashedPartialTrie)>,
}

fn apply_metadata_and_trie_memops<F: RichField + Extendable<D>, const D: usize>(
    state: &mut GenerationState<F>,
    inputs: &GenerationInputs,
) {
    let metadata = &inputs.block_metadata;
    let tries = &inputs.tries;
    let trie_roots_after = &inputs.trie_roots_after;
    let h2u = |h: H256| U256::from_big_endian(&h.0);
    let fields = [
        (
            GlobalMetadata::BlockBeneficiary,
            U256::from_big_endian(&metadata.block_beneficiary.0),
        ),
        (GlobalMetadata::BlockTimestamp, metadata.block_timestamp),
        (GlobalMetadata::BlockNumber, metadata.block_number),
        (GlobalMetadata::BlockDifficulty, metadata.block_difficulty),
        (GlobalMetadata::BlockGasLimit, metadata.block_gaslimit),
        (GlobalMetadata::BlockChainId, metadata.block_chain_id),
        (GlobalMetadata::BlockBaseFee, metadata.block_base_fee),
        (
            GlobalMetadata::StateTrieRootDigestBefore,
            h2u(tries.state_trie.hash()),
        ),
        (
            GlobalMetadata::TransactionTrieRootDigestBefore,
            h2u(tries.transactions_trie.hash()),
        ),
        (
            GlobalMetadata::ReceiptTrieRootDigestBefore,
            h2u(tries.receipts_trie.hash()),
        ),
        (
            GlobalMetadata::StateTrieRootDigestAfter,
            h2u(trie_roots_after.state_root),
        ),
        (
            GlobalMetadata::TransactionTrieRootDigestAfter,
            h2u(trie_roots_after.transactions_root),
        ),
        (
            GlobalMetadata::ReceiptTrieRootDigestAfter,
            h2u(trie_roots_after.receipts_root),
        ),
    ];

    let channel = MemoryChannel::GeneralPurpose(0);
    let ops = fields.map(|(field, val)| {
        mem_write_log(
            channel,
            MemoryAddress::new(0, Segment::GlobalMetadata, field as usize),
            state,
            val,
        )
    });

    state.memory.apply_ops(&ops);
    state.traces.memory_ops.extend(ops);
}

pub fn generate_traces<F: RichField + Extendable<D>, const D: usize>(
    all_stark: &AllStark<F, D>,
    inputs: GenerationInputs,
    config: &StarkConfig,
    timing: &mut TimingTree,
) -> anyhow::Result<(
    [Vec<PolynomialValues<F>>; NUM_TABLES],
    PublicValues,
    GenerationOutputs,
)> {
    let mut state = GenerationState::<F>::new(inputs.clone(), &KERNEL.code);

    apply_metadata_and_trie_memops(&mut state, &inputs);

    generate_bootstrap_kernel::<F>(&mut state);

    timed!(timing, "simulate CPU", simulate_cpu(&mut state)?);

    assert!(
        state.mpt_prover_inputs.is_empty(),
        "All MPT data should have been consumed"
    );

    log::info!(
        "Trace lengths (before padding): {:?}",
        state.traces.checkpoint()
    );

    let outputs = get_outputs(&mut state);

    let read_metadata = |field| state.memory.read_global_metadata(field);
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
    Ok((tables, public_values, outputs))
}

fn simulate_cpu<F: RichField + Extendable<D>, const D: usize>(
    state: &mut GenerationState<F>,
) -> anyhow::Result<()> {
    let halt_pc0 = KERNEL.global_labels["halt_pc0"];
    let halt_pc1 = KERNEL.global_labels["halt_pc1"];

    let mut already_in_halt_loop = false;
    loop {
        // If we've reached the kernel's halt routine, and our trace length is a power of 2, stop.
        let pc = state.registers.program_counter;
        let in_halt_loop = state.registers.is_kernel && (pc == halt_pc0 || pc == halt_pc1);
        if in_halt_loop && !already_in_halt_loop {
            log::info!("CPU halted after {} cycles", state.traces.clock());
        }
        already_in_halt_loop |= in_halt_loop;

        transition(state)?;

        if already_in_halt_loop && state.traces.clock().is_power_of_two() {
            log::info!("CPU trace padded to {} cycles", state.traces.clock());
            break;
        }
    }

    Ok(())
}
