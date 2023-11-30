use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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
use crate::generation::state::GenerationState;
use crate::generation::trie_extractor::{get_receipt_trie, get_state_trie, get_txn_trie};
use crate::memory::segments::Segment;
use crate::proof::{BlockHashes, BlockMetadata, ExtraBlockData, PublicValues, TrieRoots};
use crate::prover::check_abort_signal;
use crate::util::{h2u, u256_to_usize};
use crate::witness::memory::{MemoryAddress, MemoryChannel};
use crate::witness::transition::transition;

pub mod mpt;
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

    /// State trie root of the checkpoint block.
    /// This could always be the genesis block of the chain, but it allows a prover to continue proving blocks
    /// from certain checkpoint heights without requiring proofs for blocks past this checkpoint.
    pub checkpoint_state_trie_root: H256,

    /// Mapping between smart contract code hashes and the contract byte code.
    /// All account smart contracts that are invoked will have an entry present.
    pub contract_code: HashMap<H256, Vec<u8>>,

    pub block_metadata: BlockMetadata,

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

fn _simulate_cpu<F: RichField + Extendable<D>, const D: usize>(
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

fn __simulate_cpu<F: RichField + Extendable<D>, const D: usize>(
    state: &mut GenerationState<F>,
) -> anyhow::Result<()> {
    let mut profiling_map = HashMap::<String, usize>::new();

    let halt_pc = KERNEL.global_labels["halt"];

    loop {
        // If we've reached the kernel's halt routine, and our trace length is a power of 2, stop.
        let pc = state.registers.program_counter;
        if let Ok(idx) = KERNEL
            .ordered_labels
            .binary_search_by_key(&pc, |label| KERNEL.global_labels[label])
        {
            profiling_map
                .entry(KERNEL.ordered_labels[idx].clone())
                .and_modify(|counter| *counter += 1)
                .or_insert(1);
        }
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

            let mut sorted_labels: Vec<_> = profiling_map.iter().collect();
            sorted_labels.sort_unstable_by_key(|item| item.1);
            sorted_labels.reverse();
            log::info!("Offsets: {:?}", sorted_labels);

            return Ok(());
        }

        transition(state)?;
    }
}

fn simulate_cpu<F: RichField + Extendable<D>, const D: usize>(
    state: &mut GenerationState<F>,
) -> anyhow::Result<()> {
    let mut profiling_map = HashMap::<String, usize>::new();

    let halt_pc = KERNEL.global_labels["halt"];

    loop {
        // If we've reached the kernel's halt routine, and our trace length is a power of 2, stop.
        let pc = state.registers.program_counter;
        let idx = match KERNEL
            .ordered_labels
            .binary_search_by_key(&pc, |label| KERNEL.global_labels[label])
        {
            Ok(idx) => Some(idx),
            Err(0) => None,
            Err(idx) => Some(idx - 1),
        };
        if let Some(idx) = idx {
            profiling_map
                .entry(KERNEL.ordered_labels[idx].clone())
                .and_modify(|counter| *counter += 1)
                .or_insert(1);
        }
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

            let mut sorted_labels: Vec<_> = profiling_map.iter().collect();
            sorted_labels.sort_unstable_by_key(|item| item.1);
            sorted_labels.reverse();
            log::info!("Offsets: {:?}", sorted_labels);

            return Ok(());
        }

        transition(state)?;
        {
            let _ = [
                ("secp_add_valid_points_no_edge_case", 10980),
                ("ecrecover", 9009),
                ("num_bytes", 7306),
                ("hex_prefix_rlp", 5820),
                ("secp_double", 4076),
                ("encode_node_branch", 3408),
                ("mstore_unpacking", 2368),
                ("main", 2320),
                ("secp_add_valid_points", 2281),
                ("insert_accessed_addresses", 2238),
                ("decode_int_given_len", 1809),
                ("read_rlp_to_memory", 1626),
                ("load_mpt", 1355),
                ("encode_or_hash_node", 1160),
                ("mpt_read_branch", 1152),
                ("mpt_read", 1065),
                ("encode_node", 803),
                ("memcpy_bytes", 731),
                ("encode_account", 662),
                ("prepend_rlp_list_prefix", 602),
                ("pack_small_rlp", 590),
                ("secp_precompute_table", 477),
                ("encode_node_leaf", 459),
                ("mstore_unpacking_rlp", 448),
                ("maybe_hash_node", 438),
                ("encode_node_empty", 413),
                ("encode_rlp_fixed", 380),
                ("insert_touched_addresses", 368),
                ("mpt_read_extension_not_found", 340),
                ("mpt_read_state_trie", 323),
                ("sys_sstore", 313),
                ("debug_it_should_be_128", 295),
                ("process_receipt", 292),
                ("process_type_0_txn", 283),
                ("encode_rlp_scalar", 271),
                ("check_bloom_loop", 269),
                ("mpt_insert_hash_node", 252),
                ("initialize_block_bloom", 247),
                ("encode_rlp_list_prefix", 221),
                ("encode_rlp_multi_byte_string_prefix", 216),
                ("mpt_load_state_trie_value", 213),
                ("mload_packing", 204),
                ("mpt_hash", 198),
                ("decode_rlp_string_len", 197),
                ("jumpdest_analysis", 164),
                ("load_code", 155),
                ("process_normalized_txn", 154),
                ("secp_glv_decompose", 148),
                ("process_message_txn_code_loaded", 145),
                ("insert_accessed_storage_keys", 135),
                ("delete_all_touched_addresses", 128),
                ("mpt_read_leaf_not_found", 119),
                ("encode_receipt", 113),
                ("increment_nonce", 108),
                ("add_eth", 93),
                ("process_message_txn", 82),
                ("process_message_txn_after_call", 77),
                ("doubly_encode_rlp_scalar", 74),
                ("deduct_eth", 72),
                ("intrinsic_gas", 64),
                ("hash_final_tries", 58),
                ("update_txn_trie", 58),
                ("terminate_common", 53),
                ("extcodehash", 45),
                ("warm_precompiles", 45),
                ("sys_stop", 42),
                ("start_txn", 41),
                ("mpt_insert", 39),
                ("load_all_mpts", 38),
                ("sload_current", 36),
                ("encode_txn", 36),
                ("scalar_to_rlp", 35),
                ("encode_rlp_string", 34),
                ("hash_initial_tries", 33),
                ("encode_node_branch_prepend_prefix", 32),
                ("decode_rlp_scalar", 28),
                ("encode_rlp_string_small", 24),
                ("route_txn", 24),
                ("mpt_hash_storage_trie", 24),
                ("encode_rlp_string_large", 23),
                ("buy_gas", 20),
                ("mpt_insert_receipt_trie", 17),
                ("transfer_eth", 17),
                ("decode_rlp_list_len", 17),
                ("mpt_insert_txn_trie", 16),
                ("balance", 15),
                ("mpt_hash_txn_trie", 14),
                ("mpt_hash_receipt_trie", 14),
                ("mpt_hash_state_trie", 14),
                ("logs_bloom", 13),
                ("delete_all_selfdestructed_addresses", 13),
                ("encode_rlp_string_large_after_writing_len", 13),
                ("txn_after", 12),
                ("encode_rlp_256", 12),
                ("encode_storage_value", 12),
                ("increment_bounded_rlp", 11),
                ("withdrawals", 10),
                ("warm_coinbase", 9),
                ("nonce", 9),
                ("increment_sender_nonce", 9),
                ("process_based_on_type", 8),
                ("after_storage_read", 7),
                ("mpt_read_empty", 7),
                ("ec_double_retself", 6),
                ("warm_origin", 5),
                ("add_bignum", 5),
                ("check_bloom_loop_end", 4),
                ("execute_withdrawals", 3),
                ("encode_rlp_160", 3),
                ("charge_gas_hook", 2),
                ("halt", 1),
                ("jumped_to_0", 1),
            ];
            let _ = [
                ("secp_add_valid_points_no_edge_case", 10980),
                ("ecrecover", 9009),
                ("num_bytes", 7306),
                ("hex_prefix_rlp", 5820),
                ("secp_double", 4076),
                ("encode_node_branch", 3408),
                ("mstore_unpacking", 2368),
                ("main", 2306),
                ("secp_add_valid_points", 2281),
                ("insert_accessed_addresses", 2238),
                ("decode_int_given_len", 1809),
                ("encode_node_empty", 1652),
                ("read_rlp_to_memory", 1626),
                ("load_mpt", 1355),
                ("encode_or_hash_node", 1160),
                ("mpt_read_branch", 1152),
                ("mpt_read", 1065),
                ("encode_node", 803),
                ("memcpy_bytes", 731),
                ("encode_account", 662),
                ("prepend_rlp_list_prefix", 602),
                ("pack_small_rlp", 590),
                ("secp_precompute_table", 477),
                ("encode_node_leaf", 459),
                ("mstore_unpacking_rlp", 448),
                ("maybe_hash_node", 438),
                ("encode_rlp_fixed", 380),
                ("insert_touched_addresses", 368),
                ("mpt_read_extension_not_found", 340),
                ("mpt_read_state_trie", 323),
                ("sys_sstore", 313),
                ("hash_final_tries", 305),
                ("process_receipt", 292),
                ("process_type_0_txn", 283),
                ("encode_rlp_scalar", 271),
                ("check_bloom_loop", 269),
                ("mpt_insert_hash_node", 252),
                ("encode_rlp_list_prefix", 221),
                ("encode_rlp_multi_byte_string_prefix", 216),
                ("mpt_load_state_trie_value", 213),
                ("mload_packing", 204),
                ("mpt_hash", 198),
                ("decode_rlp_string_len", 197),
                ("jumpdest_analysis", 164),
                ("load_code", 155),
                ("process_normalized_txn", 154),
                ("secp_glv_decompose", 148),
                ("process_message_txn_code_loaded", 145),
                ("insert_accessed_storage_keys", 135),
                ("delete_all_touched_addresses", 128),
                ("mpt_read_leaf_not_found", 119),
                ("encode_receipt", 113),
                ("increment_nonce", 108),
                ("add_eth", 93),
                ("process_message_txn", 82),
                ("process_message_txn_after_call", 77),
                ("doubly_encode_rlp_scalar", 74),
                ("deduct_eth", 72),
                ("intrinsic_gas", 64),
                ("update_txn_trie", 58),
                ("terminate_common", 53),
                ("warm_precompiles", 45),
                ("extcodehash", 45),
                ("sys_stop", 42),
                ("start_txn", 41),
                ("mpt_insert", 39),
                ("load_all_mpts", 38),
                ("sload_current", 36),
                ("encode_txn", 36),
                ("scalar_to_rlp", 35),
                ("encode_rlp_string", 34),
                ("hash_initial_tries", 33),
                ("encode_node_branch_prepend_prefix", 32),
                ("decode_rlp_scalar", 28),
                ("route_txn", 24),
                ("mpt_hash_storage_trie", 24),
                ("encode_rlp_string_small", 24),
                ("encode_rlp_string_large", 23),
                ("buy_gas", 20),
                ("decode_rlp_list_len", 17),
                ("transfer_eth", 17),
                ("mpt_insert_receipt_trie", 17),
                ("mpt_insert_txn_trie", 16),
                ("balance", 15),
                ("mpt_hash_receipt_trie", 14),
                ("mpt_hash_txn_trie", 14),
                ("mpt_hash_state_trie", 14),
                ("delete_all_selfdestructed_addresses", 13),
                ("logs_bloom", 13),
                ("encode_rlp_string_large_after_writing_len", 13),
                ("encode_storage_value", 12),
                ("encode_rlp_256", 12),
                ("txn_after", 12),
                ("increment_bounded_rlp", 11),
                ("withdrawals", 10),
                ("nonce", 9),
                ("increment_sender_nonce", 9),
                ("warm_coinbase", 9),
                ("process_based_on_type", 8),
                ("after_storage_read", 7),
                ("mpt_read_empty", 7),
                ("ec_double_retself", 6),
                ("add_bignum", 5),
                ("warm_origin", 5),
                ("check_bloom_loop_end", 4),
                ("encode_rlp_160", 3),
                ("execute_withdrawals", 3),
                ("charge_gas_hook", 2),
                ("halt", 1),
                ("jumped_to_0", 1),
            ];

            let _ = [
                ("mstore_unpacking", 148),
                ("secp_add_valid_points", 139),
                ("secp_add_valid_points_no_edge_case", 132),
                ("secp_double", 129),
                ("mstore_unpacking_rlp", 112),
                ("encode_or_hash_node", 76),
                ("encode_node", 72),
                ("maybe_hash_node", 72),
                ("mload_packing", 68),
                ("pack_small_rlp", 59),
                ("encode_node_empty", 59),
                ("mpt_read", 50),
                ("num_bytes", 48),
                ("load_mpt", 38),
                ("mpt_read_branch", 32),
                ("memcpy_bytes", 27),
                ("encode_rlp_fixed", 20),
                ("encode_rlp_scalar", 19),
                ("mpt_read_state_trie", 17),
                ("prepend_rlp_list_prefix", 14),
                ("encode_rlp_256", 12),
                ("mpt_hash", 12),
                ("insert_accessed_addresses", 12),
                ("decode_rlp_string_len", 9),
                ("hex_prefix_rlp", 9),
                ("encode_node_leaf", 9),
                ("decode_int_given_len", 9),
                ("encode_rlp_multi_byte_string_prefix", 8),
                ("encode_rlp_list_prefix", 8),
                ("decode_rlp_scalar", 7),
                ("mpt_hash_storage_trie", 6),
                ("encode_account", 6),
                ("insert_touched_addresses", 5),
                ("encode_node_branch_prepend_prefix", 4),
                ("encode_node_branch", 4),
                ("mpt_load_state_trie_value", 3),
                ("extcodehash", 3),
                ("mpt_insert", 3),
                ("add_eth", 3),
                ("mpt_hash_state_trie", 2),
                ("encode_rlp_string", 2),
                ("deduct_eth", 2),
                ("mpt_hash_txn_trie", 2),
                ("ec_double_retself", 2),
                ("mpt_hash_receipt_trie", 2),
                ("secp_glv_decompose", 2),
                ("charge_gas_hook", 2),
                ("sys_sstore", 1),
                ("increment_sender_nonce", 1),
                ("buy_gas", 1),
                ("main", 1),
                ("process_type_0_txn", 1),
                ("jumpdest_analysis", 1),
                ("txn_after", 1),
                ("encode_rlp_160", 1),
                ("intrinsic_gas", 1),
                ("delete_all_selfdestructed_addresses", 1),
                ("encode_rlp_string_large_after_writing_len", 1),
                ("jumped_to_0", 1),
                ("decode_rlp_list_len", 1),
                ("mpt_read_empty", 1),
                ("hash_final_tries", 1),
                ("sload_current", 1),
                ("encode_txn", 1),
                ("start_txn", 1),
                ("encode_rlp_string_large", 1),
                ("load_code", 1),
                ("increment_bounded_rlp", 1),
                ("encode_receipt", 1),
                ("process_message_txn", 1),
                ("ecrecover", 1),
                ("warm_origin", 1),
                ("encode_rlp_string_small", 1),
                ("process_based_on_type", 1),
                ("secp_precompute_table", 1),
                ("halt", 1),
                ("update_txn_trie", 1),
                ("transfer_eth", 1),
                ("logs_bloom", 1),
                ("read_rlp_to_memory", 1),
                ("encode_storage_value", 1),
                ("process_receipt", 1),
                ("process_message_txn_code_loaded", 1),
                ("increment_nonce", 1),
                ("delete_all_touched_addresses", 1),
                ("terminate_common", 1),
                ("balance", 1),
                ("withdrawals", 1),
                ("sys_stop", 1),
                ("after_storage_read", 1),
                ("mpt_insert_receipt_trie", 1),
                ("hash_initial_tries", 1),
                ("doubly_encode_rlp_scalar", 1),
                ("route_txn", 1),
                ("mpt_insert_txn_trie", 1),
                ("warm_coinbase", 1),
                ("load_all_mpts", 1),
                ("warm_precompiles", 1),
                ("add_bignum", 1),
                ("insert_accessed_storage_keys", 1),
                ("process_normalized_txn", 1),
                ("scalar_to_rlp", 1),
                ("nonce", 1),
                ("process_message_txn_after_call", 1),
                ("execute_withdrawals", 1),
            ];
            let _ = [
                ("secp_add_valid_points_no_edge_case", 10980),
                ("ecrecover", 9009),
                ("num_bytes", 7306),
                ("hex_prefix_rlp", 5820),
                ("secp_double", 4076),
                ("encode_node_branch", 3440),
                ("encode_or_hash_node", 2991),
                ("mstore_unpacking", 2368),
                ("main", 2306),
                ("secp_add_valid_points", 2281),
                ("insert_accessed_addresses", 2238),
                ("decode_int_given_len", 1809),
                ("encode_node_empty", 1652),
                ("read_rlp_to_memory", 1626),
                ("load_mpt", 1355),
                ("mpt_read_branch", 1152),
                ("mpt_read", 1065),
                ("memcpy_bytes", 731),
                ("encode_account", 662),
                ("prepend_rlp_list_prefix", 602),
                ("hash_final_tries", 578),
                ("secp_precompute_table", 477),
                ("encode_node_leaf", 459),
                ("mstore_unpacking_rlp", 448),
                ("encode_rlp_fixed", 380),
                ("insert_touched_addresses", 368),
                ("mpt_read_extension_not_found", 340),
                ("mpt_read_state_trie", 323),
                ("sys_sstore", 313),
                ("process_receipt", 292),
                ("process_type_0_txn", 283),
                ("encode_rlp_scalar", 271),
                ("mpt_insert_hash_node", 252),
                ("encode_rlp_list_prefix", 221),
                ("encode_rlp_multi_byte_string_prefix", 216),
                ("mpt_load_state_trie_value", 213),
                ("mload_packing", 204),
                ("mpt_hash", 198),
                ("decode_rlp_string_len", 197),
                ("jumpdest_analysis", 164),
                ("load_code", 155),
                ("process_normalized_txn", 154),
                ("secp_glv_decompose", 148),
                ("process_message_txn_code_loaded", 145),
                ("insert_accessed_storage_keys", 135),
                ("delete_all_touched_addresses", 128),
                ("mpt_read_leaf_not_found", 119),
                ("encode_receipt", 113),
                ("increment_nonce", 108),
                ("add_eth", 93),
                ("process_message_txn", 82),
                ("process_message_txn_after_call", 77),
                ("doubly_encode_rlp_scalar", 74),
                ("deduct_eth", 72),
                ("intrinsic_gas", 64),
                ("update_txn_trie", 58),
                ("terminate_common", 53),
                ("warm_precompiles", 45),
                ("extcodehash", 45),
                ("sys_stop", 42),
                ("start_txn", 41),
                ("mpt_insert", 39),
                ("load_all_mpts", 38),
                ("encode_txn", 36),
                ("sload_current", 36),
                ("scalar_to_rlp", 35),
                ("encode_rlp_string", 34),
                ("hash_initial_tries", 33),
                ("decode_rlp_scalar", 28),
                ("route_txn", 24),
                ("mpt_hash_storage_trie", 24),
                ("encode_rlp_string_small", 24),
                ("encode_rlp_string_large", 23),
                ("buy_gas", 20),
                ("transfer_eth", 17),
                ("mpt_insert_receipt_trie", 17),
                ("decode_rlp_list_len", 17),
                ("mpt_insert_txn_trie", 16),
                ("balance", 15),
                ("mpt_hash_txn_trie", 14),
                ("mpt_hash_receipt_trie", 14),
                ("mpt_hash_state_trie", 14),
                ("delete_all_selfdestructed_addresses", 13),
                ("logs_bloom", 13),
                ("encode_rlp_string_large_after_writing_len", 13),
                ("encode_storage_value", 12),
                ("encode_rlp_256", 12),
                ("txn_after", 12),
                ("increment_bounded_rlp", 11),
                ("withdrawals", 10),
                ("increment_sender_nonce", 9),
                ("warm_coinbase", 9),
                ("nonce", 9),
                ("process_based_on_type", 8),
                ("after_storage_read", 7),
                ("mpt_read_empty", 7),
                ("ec_double_retself", 6),
                ("add_bignum", 5),
                ("warm_origin", 5),
                ("encode_rlp_160", 3),
                ("execute_withdrawals", 3),
                ("charge_gas_hook", 2),
                ("jumped_to_0", 1),
                ("halt", 1),
            ];

            let _ = [
                ("secp_add_valid_points_no_edge_case", 10980),
                ("ecrecover", 9009),
                ("num_bytes", 7306),
                ("hex_prefix_rlp", 5820),
                ("secp_double", 4076),
                ("encode_node_branch", 3440),
                ("mstore_unpacking", 2368),
                ("main", 2306),
                ("secp_add_valid_points", 2281),
                ("insert_accessed_addresses", 2238),
                ("decode_int_given_len", 1809),
                ("encode_node_empty", 1652),
                ("read_rlp_to_memory", 1626),
                ("load_mpt", 1355),
                ("encode_or_hash_node", 1160),
                ("mpt_read_branch", 1152),
                ("mpt_read", 1065),
                ("encode_node", 803),
                ("memcpy_bytes", 731),
                ("encode_account", 662),
                ("prepend_rlp_list_prefix", 602),
                ("pack_small_rlp", 590),
                ("hash_final_tries", 578),
                ("secp_precompute_table", 477),
                ("encode_node_leaf", 459),
                ("mstore_unpacking_rlp", 448),
                ("maybe_hash_node", 438),
                ("encode_rlp_fixed", 380),
                ("insert_touched_addresses", 368),
                ("mpt_read_extension_not_found", 340),
                ("mpt_read_state_trie", 323),
                ("sys_sstore", 313),
                ("process_receipt", 292),
                ("process_type_0_txn", 283),
                ("encode_rlp_scalar", 271),
                ("mpt_insert_hash_node", 252),
                ("encode_rlp_list_prefix", 221),
                ("encode_rlp_multi_byte_string_prefix", 216),
                ("mpt_load_state_trie_value", 213),
                ("mload_packing", 204),
                ("mpt_hash", 198),
                ("decode_rlp_string_len", 197),
                ("jumpdest_analysis", 164),
                ("load_code", 155),
                ("process_normalized_txn", 154),
                ("secp_glv_decompose", 148),
                ("process_message_txn_code_loaded", 145),
                ("insert_accessed_storage_keys", 135),
                ("delete_all_touched_addresses", 128),
                ("mpt_read_leaf_not_found", 119),
                ("encode_receipt", 113),
                ("increment_nonce", 108),
                ("add_eth", 93),
                ("process_message_txn", 82),
                ("process_message_txn_after_call", 77),
                ("doubly_encode_rlp_scalar", 74),
                ("deduct_eth", 72),
                ("intrinsic_gas", 64),
                ("update_txn_trie", 58),
                ("terminate_common", 53),
                ("extcodehash", 45),
                ("warm_precompiles", 45),
                ("sys_stop", 42),
                ("start_txn", 41),
                ("mpt_insert", 39),
                ("load_all_mpts", 38),
                ("encode_txn", 36),
                ("sload_current", 36),
                ("scalar_to_rlp", 35),
                ("encode_rlp_string", 34),
                ("hash_initial_tries", 33),
                ("decode_rlp_scalar", 28),
                ("route_txn", 24),
                ("mpt_hash_storage_trie", 24),
                ("encode_rlp_string_small", 24),
                ("encode_rlp_string_large", 23),
                ("buy_gas", 20),
                ("decode_rlp_list_len", 17),
                ("transfer_eth", 17),
                ("mpt_insert_receipt_trie", 17),
                ("mpt_insert_txn_trie", 16),
                ("balance", 15),
                ("mpt_hash_txn_trie", 14),
                ("mpt_hash_receipt_trie", 14),
                ("mpt_hash_state_trie", 14),
                ("encode_rlp_string_large_after_writing_len", 13),
                ("logs_bloom", 13),
                ("delete_all_selfdestructed_addresses", 13),
                ("txn_after", 12),
                ("encode_storage_value", 12),
                ("encode_rlp_256", 12),
                ("increment_bounded_rlp", 11),
                ("withdrawals", 10),
                ("nonce", 9),
                ("increment_sender_nonce", 9),
                ("warm_coinbase", 9),
                ("process_based_on_type", 8),
                ("mpt_read_empty", 7),
                ("after_storage_read", 7),
                ("ec_double_retself", 6),
                ("add_bignum", 5),
                ("warm_origin", 5),
                ("encode_rlp_160", 3),
                ("execute_withdrawals", 3),
                ("charge_gas_hook", 2),
                ("halt", 1),
                ("jumped_to_0", 1),
            ];

            let _ = [
                ("secp_add_valid_points_no_edge_case", 10980),
                ("ecrecover", 9009),
                ("num_bytes", 7306),
                ("hex_prefix_rlp", 5820),
                ("secp_double", 4076),
                ("encode_node_branch", 3408),
                ("mstore_unpacking", 2368),
                ("main", 2306),
                ("secp_add_valid_points", 2281),
                ("insert_accessed_addresses", 2238),
                ("decode_int_given_len", 1809),
                ("encode_node_empty", 1652),
                ("read_rlp_to_memory", 1626),
                ("load_mpt", 1355),
                ("encode_or_hash_node", 1160),
                ("mpt_read_branch", 1152),
                ("mpt_read", 1065),
                ("encode_node", 803),
                ("memcpy_bytes", 731),
                ("encode_account", 662),
                ("prepend_rlp_list_prefix", 602),
                ("pack_small_rlp", 590),
                ("hash_final_tries", 578),
                ("secp_precompute_table", 477),
                ("encode_node_leaf", 459),
                ("mstore_unpacking_rlp", 448),
                ("maybe_hash_node", 438),
                ("encode_rlp_fixed", 380),
                ("insert_touched_addresses", 368),
                ("mpt_read_extension_not_found", 340),
                ("mpt_read_state_trie", 323),
                ("sys_sstore", 313),
                ("process_receipt", 292),
                ("process_type_0_txn", 283),
                ("encode_rlp_scalar", 271),
                ("mpt_insert_hash_node", 252),
                ("encode_rlp_list_prefix", 221),
                ("encode_rlp_multi_byte_string_prefix", 216),
                ("mpt_load_state_trie_value", 213),
                ("mload_packing", 204),
                ("mpt_hash", 198),
                ("decode_rlp_string_len", 197),
                ("jumpdest_analysis", 164),
                ("load_code", 155),
                ("process_normalized_txn", 154),
                ("secp_glv_decompose", 148),
                ("process_message_txn_code_loaded", 145),
                ("insert_accessed_storage_keys", 135),
                ("delete_all_touched_addresses", 128),
                ("mpt_read_leaf_not_found", 119),
                ("encode_receipt", 113),
                ("increment_nonce", 108),
                ("add_eth", 93),
                ("process_message_txn", 82),
                ("process_message_txn_after_call", 77),
                ("doubly_encode_rlp_scalar", 74),
                ("deduct_eth", 72),
                ("intrinsic_gas", 64),
                ("update_txn_trie", 58),
                ("terminate_common", 53),
                ("extcodehash", 45),
                ("warm_precompiles", 45),
                ("sys_stop", 42),
                ("start_txn", 41),
                ("mpt_insert", 39),
                ("load_all_mpts", 38),
                ("encode_txn", 36),
                ("sload_current", 36),
                ("scalar_to_rlp", 35),
                ("encode_rlp_string", 34),
                ("hash_initial_tries", 33),
                ("encode_node_branch_prepend_prefix", 32),
                ("decode_rlp_scalar", 28),
                ("mpt_hash_storage_trie", 24),
                ("route_txn", 24),
                ("encode_rlp_string_small", 24),
                ("encode_rlp_string_large", 23),
                ("buy_gas", 20),
                ("transfer_eth", 17),
                ("mpt_insert_receipt_trie", 17),
                ("decode_rlp_list_len", 17),
                ("mpt_insert_txn_trie", 16),
                ("balance", 15),
                ("mpt_hash_txn_trie", 14),
                ("mpt_hash_state_trie", 14),
                ("mpt_hash_receipt_trie", 14),
                ("delete_all_selfdestructed_addresses", 13),
                ("logs_bloom", 13),
                ("encode_rlp_string_large_after_writing_len", 13),
                ("txn_after", 12),
                ("encode_rlp_256", 12),
                ("encode_storage_value", 12),
                ("increment_bounded_rlp", 11),
                ("withdrawals", 10),
                ("warm_coinbase", 9),
                ("increment_sender_nonce", 9),
                ("nonce", 9),
                ("process_based_on_type", 8),
                ("mpt_read_empty", 7),
                ("after_storage_read", 7),
                ("ec_double_retself", 6),
                ("add_bignum", 5),
                ("warm_origin", 5),
                ("encode_rlp_160", 3),
                ("execute_withdrawals", 3),
                ("charge_gas_hook", 2),
                ("halt", 1),
                ("jumped_to_0", 1),
            ];
        }
    }
}
