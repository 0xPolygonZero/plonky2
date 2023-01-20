use ethereum_types::U256;
use plonky2::field::types::Field;

use crate::generation::mpt::all_mpt_prover_inputs_reversed;
use crate::generation::rlp::all_rlp_prover_inputs_reversed;
use crate::generation::GenerationInputs;
use crate::witness::memory::MemoryState;
use crate::witness::state::RegistersState;
use crate::witness::traces::{TraceCheckpoint, Traces};

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

    /// Prover inputs containing the result of a MODMUL operation, in reverse order so that the next
    /// input can be obtained via `pop()`.
    pub(crate) bignum_modmul_prover_inputs: Vec<U256>,
}

impl<F: Field> GenerationState<F> {
    pub(crate) fn new(inputs: GenerationInputs, kernel_code: &[u8]) -> Self {
        let mpt_prover_inputs = all_mpt_prover_inputs_reversed(&inputs.tries);
        let rlp_prover_inputs = all_rlp_prover_inputs_reversed(&inputs.signed_txns);
        let bignum_modmul_prover_inputs = Vec::new();

        Self {
            inputs,
            registers: Default::default(),
            memory: MemoryState::new(kernel_code),
            traces: Traces::default(),
            next_txn_index: 0,
            mpt_prover_inputs,
            rlp_prover_inputs,
            bignum_modmul_prover_inputs,
        }
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
}
