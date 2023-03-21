use std::collections::HashMap;

use ethereum_types::{Address, H160, H256, U256};
use keccak_hash::keccak;
use plonky2::field::types::Field;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::generation::mpt::all_mpt_prover_inputs_reversed;
use crate::generation::rlp::all_rlp_prover_inputs_reversed;
use crate::generation::GenerationInputs;
use crate::witness::memory::MemoryState;
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

    pub(crate) next_txn_index: usize,

    /// Prover inputs containing MPT data, in reverse order so that the next input can be obtained
    /// via `pop()`.
    pub(crate) mpt_prover_inputs: Vec<U256>,

    /// Prover inputs containing RLP data, in reverse order so that the next input can be obtained
    /// via `pop()`.
    pub(crate) rlp_prover_inputs: Vec<U256>,

    /// The state trie only stores state keys, which are hashes of addresses, but sometimes it is
    /// useful to see the actual addresses for debugging. Here we store the mapping for all known
    /// addresses.
    pub(crate) state_key_to_address: HashMap<H256, Address>,

    /// Prover inputs containing the result of a MODMUL operation, in little-endian order (so that
    /// inputs are obtained in big-endian order via `pop()`). Contains both the remainder and the
    /// quotient, in that order.
    pub(crate) bignum_modmul_result_limbs: Vec<U256>,
}

impl<F: Field> GenerationState<F> {
    pub(crate) fn new(inputs: GenerationInputs, kernel_code: &[u8]) -> Self {
        log::debug!("Input signed_txns: {:?}", &inputs.signed_txns);
        log::debug!("Input state_trie: {:?}", &inputs.tries.state_trie);
        log::debug!(
            "Input transactions_trie: {:?}",
            &inputs.tries.transactions_trie
        );
        log::debug!("Input receipts_trie: {:?}", &inputs.tries.receipts_trie);
        log::debug!("Input storage_tries: {:?}", &inputs.tries.storage_tries);
        log::debug!("Input contract_code: {:?}", &inputs.contract_code);
        let mpt_prover_inputs = all_mpt_prover_inputs_reversed(&inputs.tries);
        let rlp_prover_inputs = all_rlp_prover_inputs_reversed(&inputs.signed_txns);
        let bignum_modmul_result_limbs = Vec::new();

        Self {
            inputs,
            registers: Default::default(),
            memory: MemoryState::new(kernel_code),
            traces: Traces::default(),
            next_txn_index: 0,
            mpt_prover_inputs,
            rlp_prover_inputs,
            state_key_to_address: HashMap::new(),
            bignum_modmul_result_limbs,
        }
    }

    /// Updates `program_counter`, and potentially adds some extra handling if we're jumping to a
    /// special location.
    pub fn jump_to(&mut self, dst: usize) {
        self.registers.program_counter = dst;
        if dst == KERNEL.global_labels["observe_new_address"] {
            let address = stack_peek(self, 0).expect("Empty stack");
            let mut address_bytes = [0; 20];
            address.to_big_endian(&mut address_bytes);
            self.observe_address(H160(address_bytes));
        }
    }

    /// Observe the given address, so that we will be able to recognize the associated state key.
    /// This is just for debugging purposes.
    pub fn observe_address(&mut self, address: Address) {
        let state_key = keccak(address.0);
        self.state_key_to_address.insert(state_key, address);
    }

    pub fn checkpoint(&self) -> GenerationStateCheckpoint {
        GenerationStateCheckpoint {
            registers: self.registers,
            traces: self.traces.checkpoint(),
        }
    }

    pub fn rollback(&mut self, checkpoint: GenerationStateCheckpoint) {
        self.registers = checkpoint.registers;
        self.traces.rollback(checkpoint.traces);
    }

    pub(crate) fn stack(&self) -> Vec<U256> {
        const MAX_TO_SHOW: usize = 10;
        (0..self.registers.stack_len.min(MAX_TO_SHOW))
            .map(|i| stack_peek(self, i).unwrap())
            .collect()
    }
}
