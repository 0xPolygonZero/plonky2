use std::collections::{BTreeSet, HashMap};

use ethereum_types::{Address, BigEndianHash, H160, H256, U256};
use keccak_hash::keccak;
use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;

use super::mpt::{load_all_mpts, TrieRootPtrs};
use super::TrieInputs;
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::generation::rlp::all_rlp_prover_inputs_reversed;
use crate::generation::GenerationInputs;
use crate::memory::segments::Segment;
use crate::util::u256_to_usize;
use crate::witness::errors::ProgramError;
use crate::witness::memory::{MemoryAddress, MemoryState};
use crate::witness::state::RegistersState;
use crate::witness::traces::{TraceCheckpoint, Traces};
use crate::witness::util::stack_peek;

pub(crate) struct GenerationStateCheckpoint {
    pub(crate) registers: RegistersState,
    pub(crate) traces: TraceCheckpoint,
}

#[derive(Debug)]
pub(crate) struct GenerationState<F: Field> {
    pub(crate) inputs: GenerationInputs,
    pub(crate) registers: RegistersState,
    pub(crate) memory: MemoryState,
    pub(crate) traces: Traces<F>,

    /// Prover inputs containing RLP data, in reverse order so that the next input can be obtained
    /// via `pop()`.
    pub(crate) rlp_prover_inputs: Vec<U256>,

    pub(crate) withdrawal_prover_inputs: Vec<U256>,

    /// The state trie only stores state keys, which are hashes of addresses, but sometimes it is
    /// useful to see the actual addresses for debugging. Here we store the mapping for all known
    /// addresses.
    pub(crate) state_key_to_address: HashMap<H256, Address>,

    /// Prover inputs containing the result of a MODMUL operation, in little-endian order (so that
    /// inputs are obtained in big-endian order via `pop()`). Contains both the remainder and the
    /// quotient, in that order.
    pub(crate) bignum_modmul_result_limbs: Vec<U256>,

    /// Pointers, within the `TrieData` segment, of the three MPTs.
    pub(crate) trie_root_ptrs: TrieRootPtrs,

    /// A hash map where the key is a context in the user's code and the value is the set of
    /// jump destinations with its corresponding "proof". A "proof" for a jump destination is
    /// either 0 or an address i > 32 in the code (not necessarily pointing to an opcode) such that
    /// for every j in [i, i+32] it holds that code[j] < 0x7f - j + i.
    pub(crate) jumpdest_proofs: Option<HashMap<usize, Vec<usize>>>,
}

impl<F: Field> GenerationState<F> {
    fn preinitialize_mpts(&mut self, trie_inputs: &TrieInputs) -> TrieRootPtrs {
        let (trie_roots_ptrs, trie_data) =
            load_all_mpts(trie_inputs).expect("Invalid MPT data for preinitialization");

        self.memory.contexts[0].segments[Segment::TrieData.unscale()].content = trie_data;

        trie_roots_ptrs
    }
    pub(crate) fn new(inputs: GenerationInputs, kernel_code: &[u8]) -> Result<Self, ProgramError> {
        log::debug!("Input signed_txn: {:?}", &inputs.signed_txn);
        log::debug!("Input state_trie: {:?}", &inputs.tries.state_trie);
        log::debug!(
            "Input transactions_trie: {:?}",
            &inputs.tries.transactions_trie
        );
        log::debug!("Input receipts_trie: {:?}", &inputs.tries.receipts_trie);
        log::debug!("Input storage_tries: {:?}", &inputs.tries.storage_tries);
        log::debug!("Input contract_code: {:?}", &inputs.contract_code);

        let rlp_prover_inputs =
            all_rlp_prover_inputs_reversed(inputs.clone().signed_txn.as_ref().unwrap_or(&vec![]));
        let withdrawal_prover_inputs = all_withdrawals_prover_inputs_reversed(&inputs.withdrawals);
        let bignum_modmul_result_limbs = Vec::new();

        let mut state = Self {
            inputs: inputs.clone(),
            registers: Default::default(),
            memory: MemoryState::new(kernel_code),
            traces: Traces::default(),
            rlp_prover_inputs,
            withdrawal_prover_inputs,
            state_key_to_address: HashMap::new(),
            bignum_modmul_result_limbs,
            trie_root_ptrs: TrieRootPtrs {
                state_root_ptr: 0,
                txn_root_ptr: 0,
                receipt_root_ptr: 0,
            },
            jumpdest_proofs: None,
        };
        let trie_root_ptrs = state.preinitialize_mpts(&inputs.tries);

        state.trie_root_ptrs = trie_root_ptrs;
        Ok(state)
    }

    /// Updates `program_counter`, and potentially adds some extra handling if we're jumping to a
    /// special location.
    pub(crate) fn jump_to(&mut self, dst: usize) -> Result<(), ProgramError> {
        self.registers.program_counter = dst;
        if dst == KERNEL.global_labels["observe_new_address"] {
            let tip_u256 = stack_peek(self, 0)?;
            let tip_h256 = H256::from_uint(&tip_u256);
            let tip_h160 = H160::from(tip_h256);
            self.observe_address(tip_h160);
        } else if dst == KERNEL.global_labels["observe_new_contract"] {
            let tip_u256 = stack_peek(self, 0)?;
            let tip_h256 = H256::from_uint(&tip_u256);
            self.observe_contract(tip_h256)?;
        }

        Ok(())
    }

    /// Observe the given address, so that we will be able to recognize the associated state key.
    /// This is just for debugging purposes.
    pub(crate) fn observe_address(&mut self, address: Address) {
        let state_key = keccak(address.0);
        self.state_key_to_address.insert(state_key, address);
    }

    /// Observe the given code hash and store the associated code.
    /// When called, the code corresponding to `codehash` should be stored in the return data.
    pub(crate) fn observe_contract(&mut self, codehash: H256) -> Result<(), ProgramError> {
        if self.inputs.contract_code.contains_key(&codehash) {
            return Ok(()); // Return early if the code hash has already been observed.
        }

        let ctx = self.registers.context;
        let returndata_offset = ContextMetadata::ReturndataSize.unscale();
        let returndata_size_addr =
            MemoryAddress::new(ctx, Segment::ContextMetadata, returndata_offset);
        let returndata_size = u256_to_usize(self.memory.get(returndata_size_addr))?;
        let code = self.memory.contexts[ctx].segments[Segment::Returndata.unscale()].content
            [..returndata_size]
            .iter()
            .map(|x| x.low_u32() as u8)
            .collect::<Vec<_>>();
        debug_assert_eq!(keccak(&code), codehash);

        self.inputs.contract_code.insert(codehash, code);

        Ok(())
    }

    pub(crate) fn checkpoint(&self) -> GenerationStateCheckpoint {
        GenerationStateCheckpoint {
            registers: self.registers,
            traces: self.traces.checkpoint(),
        }
    }

    pub(crate) fn rollback(&mut self, checkpoint: GenerationStateCheckpoint) {
        self.registers = checkpoint.registers;
        self.traces.rollback(checkpoint.traces);
    }

    pub(crate) fn stack(&self) -> Vec<U256> {
        const MAX_TO_SHOW: usize = 10;
        (0..self.registers.stack_len.min(MAX_TO_SHOW))
            .map(|i| stack_peek(self, i).unwrap())
            .collect()
    }

    /// Clones everything but the traces.
    pub(crate) fn soft_clone(&self) -> GenerationState<F> {
        Self {
            inputs: self.inputs.clone(),
            registers: self.registers,
            memory: self.memory.clone(),
            traces: Traces::default(),
            rlp_prover_inputs: self.rlp_prover_inputs.clone(),
            state_key_to_address: self.state_key_to_address.clone(),
            bignum_modmul_result_limbs: self.bignum_modmul_result_limbs.clone(),
            withdrawal_prover_inputs: self.withdrawal_prover_inputs.clone(),
            trie_root_ptrs: TrieRootPtrs {
                state_root_ptr: 0,
                txn_root_ptr: 0,
                receipt_root_ptr: 0,
            },
            jumpdest_proofs: None,
        }
    }
}

/// Withdrawals prover input array is of the form `[addr0, amount0, ..., addrN, amountN, U256::MAX, U256::MAX]`.
/// Returns the reversed array.
pub(crate) fn all_withdrawals_prover_inputs_reversed(withdrawals: &[(Address, U256)]) -> Vec<U256> {
    let mut withdrawal_prover_inputs = withdrawals
        .iter()
        .flat_map(|w| [U256::from((w.0).0.as_slice()), w.1])
        .collect::<Vec<_>>();
    withdrawal_prover_inputs.push(U256::MAX);
    withdrawal_prover_inputs.push(U256::MAX);
    withdrawal_prover_inputs.reverse();
    withdrawal_prover_inputs
}
