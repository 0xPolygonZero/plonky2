use std::collections::HashMap;

use anyhow::anyhow;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H256, U256};
use itertools::enumerate;
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
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::generation::outputs::{get_outputs, GenerationOutputs};
use crate::generation::state::GenerationState;
use crate::memory::segments::Segment;
use crate::proof::{BlockHashes, BlockMetadata, ExtraBlockData, PublicValues, TrieRoots};
use crate::util::h2u;
use crate::witness::memory::{MemoryAddress, MemoryChannel};
use crate::witness::transition::transition;

pub mod mpt;
pub mod outputs;
pub(crate) mod prover_input;
pub(crate) mod rlp;
pub(crate) mod state;
mod trie_extractor;

use self::mpt::{load_all_mpts, TrieRootPtrs};
use crate::witness::util::mem_write_log;

/// Inputs needed for trace generation.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct GenerationInputs {
    pub txn_number_before: U256,
    pub gas_used_before: U256,
    pub gas_used_after: U256,

    // A None would yield an empty proof, otherwise this contains the encoding of a transaction.
    pub signed_txn: Option<Vec<u8>>,
    // Withdrawal pairs `(addr, amount)`. At the end of the txs, `amount` is added to `addr`'s balance. See EIP-4895.
    pub withdrawals: Vec<(Address, U256)>,
    pub tries: TrieInputs,
    /// Expected trie roots after the transactions are executed.
    pub trie_roots_after: TrieRoots,
    /// State trie root of the genesis block.
    pub genesis_state_trie_root: H256,

    /// Mapping between smart contract code hashes and the contract byte code.
    /// All account smart contracts that are invoked will have an entry present.
    pub contract_code: HashMap<H256, Vec<u8>>,

    pub block_metadata: BlockMetadata,

    pub block_hashes: BlockHashes,

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

fn apply_metadata_and_tries_memops<F: RichField + Extendable<D>, const D: usize>(
    state: &mut GenerationState<F>,
    inputs: &GenerationInputs,
) {
    let metadata = &inputs.block_metadata;
    let tries = &inputs.tries;
    let trie_roots_after = &inputs.trie_roots_after;
    let fields = [
        (
            GlobalMetadata::BlockBeneficiary,
            U256::from_big_endian(&metadata.block_beneficiary.0),
        ),
        (GlobalMetadata::BlockTimestamp, metadata.block_timestamp),
        (GlobalMetadata::BlockNumber, metadata.block_number),
        (GlobalMetadata::BlockDifficulty, metadata.block_difficulty),
        (
            GlobalMetadata::BlockRandom,
            metadata.block_random.into_uint(),
        ),
        (GlobalMetadata::BlockGasLimit, metadata.block_gaslimit),
        (GlobalMetadata::BlockChainId, metadata.block_chain_id),
        (GlobalMetadata::BlockBaseFee, metadata.block_base_fee),
        (
            GlobalMetadata::BlockCurrentHash,
            h2u(inputs.block_hashes.cur_hash),
        ),
        (GlobalMetadata::BlockGasUsed, metadata.block_gas_used),
        (GlobalMetadata::BlockGasUsedBefore, inputs.gas_used_before),
        (GlobalMetadata::BlockGasUsedAfter, inputs.gas_used_after),
        (GlobalMetadata::TxnNumberBefore, inputs.txn_number_before),
        (
            GlobalMetadata::TxnNumberAfter,
            inputs.txn_number_before + if inputs.signed_txn.is_some() { 1 } else { 0 },
        ),
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
        (GlobalMetadata::KernelHash, h2u(KERNEL.code_hash)),
        (GlobalMetadata::KernelLen, KERNEL.code.len().into()),
        (GlobalMetadata::TrieDataSize, state.trie_data_len.into()),
        (
            GlobalMetadata::StateTrieRoot,
            state.trie_root_ptrs.state_root_ptr.into(),
        ),
        (
            GlobalMetadata::TransactionTrieRoot,
            state.trie_root_ptrs.txn_root_ptr.into(),
        ),
        (
            GlobalMetadata::ReceiptTrieRoot,
            state.trie_root_ptrs.receipt_root_ptr.into(),
        ),
    ];

    let channel = MemoryChannel::GeneralPurpose(0);
    let mut ops = fields
        .map(|(field, val)| {
            mem_write_log(
                channel,
                MemoryAddress::new(0, Segment::GlobalMetadata, field as usize),
                state,
                val,
            )
        })
        .to_vec();

    // Write the block's final block bloom filter.
    ops.extend((0..8).map(|i| {
        mem_write_log(
            channel,
            MemoryAddress::new(0, Segment::GlobalBlockBloom, i),
            state,
            metadata.block_bloom[i],
        )
    }));

    // Write previous block hashes.
    ops.extend(
        (0..256)
            .map(|i| {
                mem_write_log(
                    channel,
                    MemoryAddress::new(0, Segment::BlockHashes, i),
                    state,
                    h2u(inputs.block_hashes.prev_hashes[i]),
                )
            })
            .collect::<Vec<_>>(),
    );

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
    let mut state = GenerationState::<F>::new(inputs.clone(), &KERNEL.code)
        .map_err(|err| anyhow!("Failed to parse all the initial prover inputs: {:?}", err))?;

    apply_metadata_and_tries_memops(&mut state, &inputs);

    timed!(timing, "simulate CPU", simulate_cpu(&mut state)?);

    log::info!(
        "Trace lengths (before padding): {:?}",
        state.traces.get_lengths()
    );

    let outputs = get_outputs(&mut state)
        .map_err(|err| anyhow!("Failed to generate post-state info: {:?}", err))?;

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

    let gas_used_after = read_metadata(GlobalMetadata::BlockGasUsedAfter);
    let txn_number_after = read_metadata(GlobalMetadata::TxnNumberAfter);

    let trie_root_ptrs = state.trie_root_ptrs;
    let extra_block_data = ExtraBlockData {
        genesis_state_trie_root: inputs.genesis_state_trie_root,
        txn_number_before: inputs.txn_number_before,
        txn_number_after,
        gas_used_before: inputs.gas_used_before,
        gas_used_after,
        trie_data_len: state.trie_data_len.into(),
        trie_root_ptrs: [
            trie_root_ptrs.state_root_ptr.into(),
            trie_root_ptrs.txn_root_ptr.into(),
            trie_root_ptrs.receipt_root_ptr.into(),
        ],
    };

    let public_values = PublicValues {
        trie_roots_before,
        trie_roots_after,
        block_metadata: inputs.block_metadata,
        block_hashes: inputs.block_hashes,
        extra_block_data,
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
    let halt_pc = KERNEL.global_labels["halt"];

    loop {
        // If we've reached the kernel's halt routine, and our trace length is a power of 2, stop.
        let pc = state.registers.program_counter;
        let halt = state.registers.is_kernel && pc == halt_pc;
        if halt {
            log::info!("CPU halted after {} cycles", state.traces.clock());

            // Padding
            let mut row = CpuColumnsView::<F>::default();
            row.clock = F::from_canonical_usize(state.traces.clock());
            row.context = F::from_canonical_usize(state.registers.context);
            row.program_counter = F::from_canonical_usize(pc);
            row.is_kernel_mode = F::ONE;
            row.gas = F::from_canonical_u64(state.registers.gas_used);
            row.stack_len = F::from_canonical_usize(state.registers.stack_len);

            loop {
                state.traces.push_cpu(row);
                row.clock += F::ONE;
                if state.traces.clock().is_power_of_two() {
                    break;
                }
            }

            log::info!("CPU trace padded to {} cycles", state.traces.clock());

            return Ok(());
        }

        transition(state)?;
    }
}
