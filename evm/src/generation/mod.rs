use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::anyhow;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H256, U256};
use itertools::enumerate;
use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
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
use crate::cpu::kernel::assembler::Kernel;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::opcodes::get_opcode;
use crate::generation::state::GenerationState;
use crate::generation::trie_extractor::{get_receipt_trie, get_state_trie, get_txn_trie};
use crate::memory::segments::Segment;
use crate::proof::{BlockHashes, BlockMetadata, ExtraBlockData, PublicValues, TrieRoots};
use crate::prover::check_abort_signal;
use crate::util::{h2u, u256_to_u8, u256_to_usize};
use crate::witness::errors::{ProgramError, ProverInputError};
use crate::witness::memory::{MemoryAddress, MemoryChannel};
use crate::witness::transition::transition;

pub mod mpt;
pub(crate) mod prover_input;
pub(crate) mod rlp;
pub(crate) mod state;
mod trie_extractor;

use self::mpt::{load_all_mpts, TrieRootPtrs};
use crate::witness::util::{mem_write_log, stack_peek};

/// Inputs needed for trace generation.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct GenerationInputs {
    /// The index of the transaction being proven within its block.
    pub txn_number_before: U256,
    /// The cumulative gas used through the execution of all transactions prior the current one.
    pub gas_used_before: U256,
    /// The cumulative gas used after the execution of the current transaction. The exact gas used
    /// by the current transaction is `gas_used_after` - `gas_used_before`.
    pub gas_used_after: U256,

    /// A None would yield an empty proof, otherwise this contains the encoding of a transaction.
    pub signed_txn: Option<Vec<u8>>,
    /// Withdrawal pairs `(addr, amount)`. At the end of the txs, `amount` is added to `addr`'s balance. See EIP-4895.
    pub withdrawals: Vec<(Address, U256)>,
    pub tries: TrieInputs,
    /// Expected trie roots after the transactions are executed.
    pub trie_roots_after: TrieRoots,

    /// State trie root of the checkpoint block.
    /// This could always be the genesis block of the chain, but it allows a prover to continue proving blocks
    /// from certain checkpoint heights without requiring proofs for blocks past this checkpoint.
    pub checkpoint_state_trie_root: H256,

    /// Mapping between smart contract code hashes and the contract byte code.
    /// All account smart contracts that are invoked will have an entry present.
    pub contract_code: HashMap<H256, Vec<u8>>,

    /// Information contained in the block header.
    pub block_metadata: BlockMetadata,

    /// The hash of the current block, and a list of the 256 previous block hashes.
    pub block_hashes: BlockHashes,
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
    ];

    let channel = MemoryChannel::GeneralPurpose(0);
    let mut ops = fields
        .map(|(field, val)| {
            mem_write_log(
                channel,
                // These fields are already scaled by their segment, and are in context 0 (kernel).
                MemoryAddress::new_bundle(U256::from(field as usize)).unwrap(),
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
) -> anyhow::Result<([Vec<PolynomialValues<F>>; NUM_TABLES], PublicValues)> {
    let mut state = GenerationState::<F>::new(inputs.clone(), &KERNEL.code)
        .map_err(|err| anyhow!("Failed to parse all the initial prover inputs: {:?}", err))?;

    apply_metadata_and_tries_memops(&mut state, &inputs);

    let cpu_res = timed!(timing, "simulate CPU", simulate_cpu(&mut state));
    if cpu_res.is_err() {
        // Retrieve previous PC (before jumping to KernelPanic), to see if we reached `hash_final_tries`.
        // We will output debugging information on the final tries only if we got a root mismatch.
        let previous_pc = state
            .traces
            .cpu
            .last()
            .expect("We should have CPU rows")
            .program_counter
            .to_canonical_u64() as usize;

        if KERNEL.offset_name(previous_pc).contains("hash_final_tries") {
            let state_trie_ptr = u256_to_usize(
                state
                    .memory
                    .read_global_metadata(GlobalMetadata::StateTrieRoot),
            )
            .map_err(|_| anyhow!("State trie pointer is too large to fit in a usize."))?;
            log::debug!(
                "Computed state trie: {:?}",
                get_state_trie::<HashedPartialTrie>(&state.memory, state_trie_ptr)
            );

            let txn_trie_ptr = u256_to_usize(
                state
                    .memory
                    .read_global_metadata(GlobalMetadata::TransactionTrieRoot),
            )
            .map_err(|_| anyhow!("Transactions trie pointer is too large to fit in a usize."))?;
            log::debug!(
                "Computed transactions trie: {:?}",
                get_txn_trie::<HashedPartialTrie>(&state.memory, txn_trie_ptr)
            );

            let receipt_trie_ptr = u256_to_usize(
                state
                    .memory
                    .read_global_metadata(GlobalMetadata::ReceiptTrieRoot),
            )
            .map_err(|_| anyhow!("Receipts trie pointer is too large to fit in a usize."))?;
            log::debug!(
                "Computed receipts trie: {:?}",
                get_receipt_trie::<HashedPartialTrie>(&state.memory, receipt_trie_ptr)
            );
        }

        cpu_res?;
    }

    log::info!(
        "Trace lengths (before padding): {:?}",
        state.traces.get_lengths()
    );

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
        checkpoint_state_trie_root: inputs.checkpoint_state_trie_root,
        txn_number_before: inputs.txn_number_before,
        txn_number_after,
        gas_used_before: inputs.gas_used_before,
        gas_used_after,
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
    Ok((tables, public_values))
}

fn simulate_cpu<F: Field>(state: &mut GenerationState<F>) -> anyhow::Result<()> {
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

fn simulate_cpu_between_labels_and_get_user_jumps<F: Field>(
    initial_label: &str,
    final_label: &str,
    state: &mut GenerationState<F>,
) -> Option<HashMap<usize, BTreeSet<usize>>> {
    if state.jumpdest_proofs.is_some() {
        None
    } else {
        const JUMP_OPCODE: u8 = 0x56;
        const JUMPI_OPCODE: u8 = 0x57;

        let halt_pc = KERNEL.global_labels[final_label];
        let mut jumpdest_addresses: HashMap<_, BTreeSet<usize>> = HashMap::new();

        state.registers.program_counter = KERNEL.global_labels[initial_label];
        let initial_clock = state.traces.clock();
        let initial_context = state.registers.context;

        log::debug!("Simulating CPU for jumpdest analysis.");

        loop {
            // skip jumpdest table validations in simulations
            if state.registers.is_kernel
                && state.registers.program_counter == KERNEL.global_labels["jumpdest_analysis"]
            {
                state.registers.program_counter = KERNEL.global_labels["jumpdest_analysis_end"]
            }
            let pc = state.registers.program_counter;
            let context = state.registers.context;
            let mut halt = state.registers.is_kernel
                && pc == halt_pc
                && state.registers.context == initial_context;
            let Ok(opcode) = u256_to_u8(state.memory.get(MemoryAddress::new(
                context,
                Segment::Code,
                state.registers.program_counter,
            ))) else {
                log::debug!(
                    "Simulated CPU for jumpdest analysis halted after {} cycles",
                    state.traces.clock() - initial_clock
                );
                return Some(jumpdest_addresses);
            };
            let cond = if let Ok(cond) = stack_peek(state, 1) {
                cond != U256::zero()
            } else {
                false
            };
            if !state.registers.is_kernel
                && (opcode == JUMP_OPCODE || (opcode == JUMPI_OPCODE && cond))
            {
                // Avoid deeper calls to abort
                let Ok(jumpdest) = u256_to_usize(state.registers.stack_top) else {
                    log::debug!(
                        "Simulated CPU for jumpdest analysis halted after {} cycles",
                        state.traces.clock() - initial_clock
                    );
                    return Some(jumpdest_addresses);
                };
                state.memory.set(
                    MemoryAddress::new(context, Segment::JumpdestBits, jumpdest),
                    U256::one(),
                );
                let jumpdest_opcode =
                    state
                        .memory
                        .get(MemoryAddress::new(context, Segment::Code, jumpdest));
                if let Some(ctx_addresses) = jumpdest_addresses.get_mut(&context) {
                    ctx_addresses.insert(jumpdest);
                } else {
                    jumpdest_addresses.insert(context, BTreeSet::from([jumpdest]));
                }
            }
            if halt || transition(state).is_err() {
                log::debug!(
                    "Simulated CPU for jumpdest analysis halted after {} cycles",
                    state.traces.clock() - initial_clock
                );
                return Some(jumpdest_addresses);
            }
        }
    }
}
