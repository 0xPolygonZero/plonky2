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
}

impl<F: Field> GenerationState<F> {
    pub(crate) fn new(inputs: GenerationInputs, kernel_code: &[u8]) -> Self {
        let mpt_prover_inputs = all_mpt_prover_inputs_reversed(&inputs.tries);
        let rlp_prover_inputs = all_rlp_prover_inputs_reversed(&inputs.signed_txns);

        Self {
            inputs,
            registers: Default::default(),
            memory: MemoryState::new(kernel_code),
            traces: Traces::default(),
            next_txn_index: 0,
            mpt_prover_inputs,
            rlp_prover_inputs,
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

    // /// Evaluate the Keccak-f permutation in-place on some data in memory, and record the operations
    // /// for the purpose of witness generation.
    // #[allow(unused)] // TODO: Should be used soon.
    // pub(crate) fn keccak_memory(
    //     &mut self,
    //     context: usize,
    //     segment: Segment,
    //     virt: usize,
    // ) -> [u64; keccak::keccak_stark::NUM_INPUTS] {
    //     let read_timestamp = self.cpu_rows.len() * NUM_CHANNELS;
    //     let _write_timestamp = read_timestamp + 1;
    //     let input = (0..25)
    //         .map(|i| {
    //             let bytes = [0, 1, 2, 3, 4, 5, 6, 7].map(|j| {
    //                 let virt = virt + i * 8 + j;
    //                 let byte = self.get_mem(context, segment, virt, read_timestamp);
    //                 debug_assert!(byte.bits() <= 8);
    //                 byte.as_u32() as u8
    //             });
    //             u64::from_le_bytes(bytes)
    //         })
    //         .collect::<Vec<_>>()
    //         .try_into()
    //         .unwrap();
    //     let output = self.keccak(input);
    //     self.keccak_memory_inputs.push(KeccakMemoryOp {
    //         context,
    //         segment,
    //         virt,
    //         read_timestamp,
    //         input,
    //         output,
    //     });
    //     // TODO: Write output to memory.
    //     output
    // }

    // /// Evaluate the Keccak-f permutation, and record the operation for the purpose of witness
    // /// generation.
    // pub(crate) fn keccak(
    //     &mut self,
    //     mut input: [u64; keccak::keccak_stark::NUM_INPUTS],
    // ) -> [u64; keccak::keccak_stark::NUM_INPUTS] {
    //     self.keccak_inputs.push(input);
    //     keccakf(&mut input);
    //     input
    // }
}
