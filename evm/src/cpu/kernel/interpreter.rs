//! An EVM interpreter for testing and debugging purposes.

use core::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::ops::Range;

use anyhow::bail;
use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::{BigEndianHash, H160, H256, U256, U512};
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;

use super::assembler::BYTES_PER_OFFSET;
use super::utils::u256_from_bool;
use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::txn_fields::NormalizedTxnField;
use crate::cpu::stack::MAX_USER_STACK_SIZE;
use crate::extension_tower::BN_BASE;
use crate::generation::mpt::load_all_mpts;
use crate::generation::prover_input::ProverInputFn;
use crate::generation::rlp::all_rlp_prover_inputs_reversed;
use crate::generation::state::{all_withdrawals_prover_inputs_reversed, GenerationState};
use crate::generation::GenerationInputs;
use crate::memory::segments::{Segment, SEGMENT_SCALING_FACTOR};
use crate::util::{h2u, u256_to_usize};
use crate::witness::errors::{ProgramError, ProverInputError};
use crate::witness::gas::gas_to_charge;
use crate::witness::memory::{MemoryAddress, MemoryContextState, MemorySegmentState, MemoryState};
use crate::witness::operation::{Operation, CONTEXT_SCALING_FACTOR};
use crate::witness::state::RegistersState;
use crate::witness::transition::decode;
use crate::witness::util::stack_peek;

type F = GoldilocksField;

/// Halt interpreter execution whenever a jump to this offset is done.
const DEFAULT_HALT_OFFSET: usize = 0xdeadbeef;

impl MemoryState {
    pub(crate) fn mload_general(&self, context: usize, segment: Segment, offset: usize) -> U256 {
        self.get(MemoryAddress::new(context, segment, offset))
    }

    fn mstore_general(
        &mut self,
        context: usize,
        segment: Segment,
        offset: usize,
        value: U256,
    ) -> InterpreterMemOpKind {
        let old_value = self.mload_general(context, segment, offset);
        self.set(MemoryAddress::new(context, segment, offset), value);
        InterpreterMemOpKind::Write(old_value, context, segment as usize, offset)
    }
}

pub(crate) struct Interpreter<'a> {
    jumpdests: Vec<usize>,
    pub(crate) generation_state: GenerationState<F>,
    prover_inputs_map: &'a HashMap<usize, ProverInputFn>,
    pub(crate) halt_offsets: Vec<usize>,
    pub(crate) debug_offsets: Vec<usize>,
    running: bool,
    opcode_count: [usize; 0x100],
    memops: Vec<InterpreterMemOpKind>,
}

/// Structure storing the state of the interpreter's registers.
struct InterpreterRegistersState {
    kernel_mode: bool,
    context: usize,
    registers: RegistersState,
}

/// Interpreter state at the last checkpoint: we only need to store
/// the state of the registers and the length of the vector of memory operations.
/// This data is enough to revert in case of an exception.
struct InterpreterCheckpoint {
    registers: InterpreterRegistersState,
    mem_len: usize,
}

pub(crate) fn run_interpreter(
    initial_offset: usize,
    initial_stack: Vec<U256>,
) -> anyhow::Result<Interpreter<'static>> {
    run(
        &KERNEL.code,
        initial_offset,
        initial_stack,
        &KERNEL.prover_inputs,
    )
}

#[derive(Clone)]
pub(crate) struct InterpreterMemoryInitialization {
    pub label: String,
    pub stack: Vec<U256>,
    pub segment: Segment,
    pub memory: Vec<(usize, Vec<U256>)>,
}

pub(crate) fn run_interpreter_with_memory(
    memory_init: InterpreterMemoryInitialization,
) -> anyhow::Result<Interpreter<'static>> {
    let label = KERNEL.global_labels[&memory_init.label];
    let mut stack = memory_init.stack;
    stack.reverse();
    let mut interpreter = Interpreter::new_with_kernel(label, stack);
    for (pointer, data) in memory_init.memory {
        for (i, term) in data.iter().enumerate() {
            interpreter.generation_state.memory.set(
                MemoryAddress::new(0, memory_init.segment, pointer + i),
                *term,
            )
        }
    }
    interpreter.run()?;
    Ok(interpreter)
}

pub(crate) fn run<'a>(
    code: &'a [u8],
    initial_offset: usize,
    initial_stack: Vec<U256>,
    prover_inputs: &'a HashMap<usize, ProverInputFn>,
) -> anyhow::Result<Interpreter<'a>> {
    let mut interpreter = Interpreter::new(code, initial_offset, initial_stack, prover_inputs);
    interpreter.run()?;
    Ok(interpreter)
}

/// Different types of Memory operations in the interpreter, and the data required to revert them.
enum InterpreterMemOpKind {
    /// We need to provide the context.
    Push(usize),
    /// If we pop a certain value, we need to push it back to the correct context when reverting.
    Pop(U256, usize),
    /// If we write a value at a certain address, we need to write the old value back when reverting.
    Write(U256, usize, usize, usize),
}

impl<'a> Interpreter<'a> {
    pub(crate) fn new_with_kernel(initial_offset: usize, initial_stack: Vec<U256>) -> Self {
        let mut result = Self::new(
            &KERNEL.code,
            initial_offset,
            initial_stack,
            &KERNEL.prover_inputs,
        );
        result.initialize_rlp_segment();
        result
    }

    /// Returns an instance of `Interpreter` given `GenerationInputs`, and assuming we are
    /// initializing with the `KERNEL` code.
    pub(crate) fn new_with_generation_inputs_and_kernel(
        initial_offset: usize,
        initial_stack: Vec<U256>,
        inputs: GenerationInputs,
    ) -> Self {
        let mut result = Self::new_with_kernel(initial_offset, initial_stack);
        result.initialize_interpreter_state_with_kernel(inputs);
        result
    }

    pub(crate) fn new(
        code: &'a [u8],
        initial_offset: usize,
        initial_stack: Vec<U256>,
        prover_inputs: &'a HashMap<usize, ProverInputFn>,
    ) -> Self {
        let mut result = Self {
            jumpdests: find_jumpdests(code),
            generation_state: GenerationState::new(GenerationInputs::default(), code)
                .expect("Default inputs are known-good"),
            prover_inputs_map: prover_inputs,
            // `DEFAULT_HALT_OFFSET` is used as a halting point for the interpreter,
            // while the label `halt` is the halting label in the kernel.
            halt_offsets: vec![DEFAULT_HALT_OFFSET, KERNEL.global_labels["halt"]],
            debug_offsets: vec![],
            running: false,
            opcode_count: [0; 256],
            memops: vec![],
        };
        result.generation_state.registers.program_counter = initial_offset;
        let initial_stack_len = initial_stack.len();
        result.generation_state.registers.stack_len = initial_stack_len;
        if !initial_stack.is_empty() {
            result.generation_state.registers.stack_top = initial_stack[initial_stack_len - 1];
            *result.stack_segment_mut() = initial_stack;
            result.stack_segment_mut().truncate(initial_stack_len - 1);
        }

        result
    }

    /// Initializes the interpreter state given `GenerationInputs`, using the KERNEL code.
    pub(crate) fn initialize_interpreter_state_with_kernel(&mut self, inputs: GenerationInputs) {
        self.initialize_interpreter_state(inputs, KERNEL.code_hash, KERNEL.code.len());
    }

    /// Initializes the interpreter state given `GenerationInputs`.
    pub(crate) fn initialize_interpreter_state(
        &mut self,
        inputs: GenerationInputs,
        kernel_hash: H256,
        kernel_code_len: usize,
    ) {
        let tries = &inputs.tries;

        // Set state's inputs.
        self.generation_state.inputs = inputs.clone();

        // Initialize the MPT's pointers.
        let (trie_root_ptrs, trie_data) =
            load_all_mpts(tries).expect("Invalid MPT data for preinitialization");
        let trie_roots_after = &inputs.trie_roots_after;
        self.generation_state.trie_root_ptrs = trie_root_ptrs;

        // Initialize the `TrieData` segment.
        for (i, data) in trie_data.iter().enumerate() {
            let trie_addr = MemoryAddress::new(0, Segment::TrieData, i);
            self.generation_state.memory.set(trie_addr, data.into());
        }

        // Update the RLP and withdrawal prover inputs.
        let rlp_prover_inputs =
            all_rlp_prover_inputs_reversed(inputs.clone().signed_txn.as_ref().unwrap_or(&vec![]));
        let withdrawal_prover_inputs = all_withdrawals_prover_inputs_reversed(&inputs.withdrawals);
        self.generation_state.rlp_prover_inputs = rlp_prover_inputs;
        self.generation_state.withdrawal_prover_inputs = withdrawal_prover_inputs;

        // Set `GlobalMetadata` values.
        let metadata = &inputs.block_metadata;
        let global_metadata_to_set = [
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
            (GlobalMetadata::KernelHash, h2u(kernel_hash)),
            (GlobalMetadata::KernelLen, kernel_code_len.into()),
        ];

        self.set_global_metadata_multi_fields(&global_metadata_to_set);

        // Set final block bloom values.
        let final_block_bloom_fields = (0..8)
            .map(|i| {
                (
                    MemoryAddress::new_u256s(
                        U256::zero(),
                        (Segment::GlobalBlockBloom.unscale()).into(),
                        i.into(),
                    )
                    .unwrap(),
                    metadata.block_bloom[i],
                )
            })
            .collect::<Vec<_>>();

        self.set_memory_multi_addresses(&final_block_bloom_fields);

        // Set previous block hash.
        let block_hashes_fields = (0..256)
            .map(|i| {
                (
                    MemoryAddress::new_u256s(
                        U256::zero(),
                        (Segment::BlockHashes.unscale()).into(),
                        i.into(),
                    )
                    .unwrap(),
                    h2u(inputs.block_hashes.prev_hashes[i]),
                )
            })
            .collect::<Vec<_>>();

        self.set_memory_multi_addresses(&block_hashes_fields);
    }

    fn checkpoint(&self) -> InterpreterCheckpoint {
        let registers = InterpreterRegistersState {
            kernel_mode: self.is_kernel(),
            context: self.context(),
            registers: self.generation_state.registers,
        };
        InterpreterCheckpoint {
            registers,
            mem_len: self.memops.len(),
        }
    }

    fn roll_memory_back(&mut self, len: usize) {
        // We roll the memory back until `memops` reaches length `len`.
        debug_assert!(self.memops.len() >= len);
        while self.memops.len() > len {
            if let Some(op) = self.memops.pop() {
                match op {
                    InterpreterMemOpKind::Push(context) => {
                        self.generation_state.memory.contexts[context].segments
                            [Segment::Stack.unscale()]
                        .content
                        .pop();
                    }
                    InterpreterMemOpKind::Pop(value, context) => {
                        self.generation_state.memory.contexts[context].segments
                            [Segment::Stack.unscale()]
                        .content
                        .push(value)
                    }
                    InterpreterMemOpKind::Write(value, context, segment, offset) => {
                        self.generation_state.memory.contexts[context].segments
                            [segment >> SEGMENT_SCALING_FACTOR] // we need to unscale the segment value
                            .content[offset] = value
                    }
                }
            }
        }
    }

    fn rollback(&mut self, checkpoint: InterpreterCheckpoint) {
        let InterpreterRegistersState {
            kernel_mode,
            context,
            registers,
        } = checkpoint.registers;
        self.set_is_kernel(kernel_mode);
        self.set_context(context);
        self.generation_state.registers = registers;
        self.roll_memory_back(checkpoint.mem_len);
    }

    fn handle_error(&mut self, err: ProgramError) -> anyhow::Result<()> {
        let exc_code: u8 = match err {
            ProgramError::OutOfGas => 0,
            ProgramError::InvalidOpcode => 1,
            ProgramError::StackUnderflow => 2,
            ProgramError::InvalidJumpDestination => 3,
            ProgramError::InvalidJumpiDestination => 4,
            ProgramError::StackOverflow => 5,
            _ => bail!("TODO: figure out what to do with this..."),
        };

        self.run_exception(exc_code)
            .map_err(|_| anyhow::Error::msg("error handling errored..."))
    }

    pub(crate) fn run(&mut self) -> anyhow::Result<()> {
        self.running = true;
        while self.running {
            let pc = self.generation_state.registers.program_counter;
            if self.is_kernel() && self.halt_offsets.contains(&pc) {
                return Ok(());
            };

            let checkpoint = self.checkpoint();
            let result = self.run_opcode();
            match result {
                Ok(()) => Ok(()),
                Err(e) => {
                    if self.is_kernel() {
                        let offset_name =
                            KERNEL.offset_name(self.generation_state.registers.program_counter);
                        bail!(
                            "{:?} in kernel at pc={}, stack={:?}, memory={:?}",
                            e,
                            offset_name,
                            self.stack(),
                            self.generation_state.memory.contexts[0].segments
                                [Segment::KernelGeneral.unscale()]
                            .content,
                        );
                    }
                    self.rollback(checkpoint);
                    self.handle_error(e)
                }
            }?;
        }
        println!("Opcode count:");
        for i in 0..0x100 {
            if self.opcode_count[i] > 0 {
                println!("{}: {}", get_mnemonic(i as u8), self.opcode_count[i])
            }
        }
        println!("Total: {}", self.opcode_count.into_iter().sum::<usize>());
        Ok(())
    }

    fn code(&self) -> &MemorySegmentState {
        // The context is 0 if we are in kernel mode.
        &self.generation_state.memory.contexts[(1 - self.is_kernel() as usize) * self.context()]
            .segments[Segment::Code.unscale()]
    }

    fn code_slice(&self, n: usize) -> Vec<u8> {
        let pc = self.generation_state.registers.program_counter;
        self.code().content[pc..pc + n]
            .iter()
            .map(|u256| u256.byte(0))
            .collect::<Vec<_>>()
    }

    pub(crate) fn get_txn_field(&self, field: NormalizedTxnField) -> U256 {
        // These fields are already scaled by their respective segment.
        self.generation_state.memory.contexts[0].segments[Segment::TxnFields.unscale()]
            .get(field.unscale())
    }

    pub(crate) fn set_txn_field(&mut self, field: NormalizedTxnField, value: U256) {
        // These fields are already scaled by their respective segment.
        self.generation_state.memory.contexts[0].segments[Segment::TxnFields.unscale()]
            .set(field.unscale(), value);
    }

    pub(crate) fn get_txn_data(&self) -> &[U256] {
        &self.generation_state.memory.contexts[0].segments[Segment::TxnData.unscale()].content
    }

    pub(crate) fn get_context_metadata_field(&self, ctx: usize, field: ContextMetadata) -> U256 {
        // These fields are already scaled by their respective segment.
        self.generation_state.memory.contexts[ctx].segments[Segment::ContextMetadata.unscale()]
            .get(field.unscale())
    }

    pub(crate) fn set_context_metadata_field(
        &mut self,
        ctx: usize,
        field: ContextMetadata,
        value: U256,
    ) {
        // These fields are already scaled by their respective segment.
        self.generation_state.memory.contexts[ctx].segments[Segment::ContextMetadata.unscale()]
            .set(field.unscale(), value)
    }

    pub(crate) fn get_global_metadata_field(&self, field: GlobalMetadata) -> U256 {
        // These fields are already scaled by their respective segment.
        let field = field.unscale();
        self.generation_state.memory.contexts[0].segments[Segment::GlobalMetadata.unscale()]
            .get(field)
    }

    pub(crate) fn set_global_metadata_field(&mut self, field: GlobalMetadata, value: U256) {
        // These fields are already scaled by their respective segment.
        let field = field.unscale();
        self.generation_state.memory.contexts[0].segments[Segment::GlobalMetadata.unscale()]
            .set(field, value)
    }

    pub(crate) fn set_global_metadata_multi_fields(&mut self, metadata: &[(GlobalMetadata, U256)]) {
        for &(field, value) in metadata {
            let field = field.unscale();
            self.generation_state.memory.contexts[0].segments[Segment::GlobalMetadata.unscale()]
                .set(field, value);
        }
    }

    pub(crate) fn get_trie_data(&self) -> &[U256] {
        &self.generation_state.memory.contexts[0].segments[Segment::TrieData.unscale()].content
    }

    pub(crate) fn get_trie_data_mut(&mut self) -> &mut Vec<U256> {
        &mut self.generation_state.memory.contexts[0].segments[Segment::TrieData.unscale()].content
    }

    pub(crate) fn get_memory_segment(&self, segment: Segment) -> Vec<U256> {
        self.generation_state.memory.contexts[0].segments[segment.unscale()]
            .content
            .clone()
    }

    pub(crate) fn get_memory_segment_bytes(&self, segment: Segment) -> Vec<u8> {
        self.generation_state.memory.contexts[0].segments[segment.unscale()]
            .content
            .iter()
            .map(|x| x.low_u32() as u8)
            .collect()
    }

    pub(crate) fn get_current_general_memory(&self) -> Vec<U256> {
        self.generation_state.memory.contexts[self.context()].segments
            [Segment::KernelGeneral.unscale()]
        .content
        .clone()
    }

    pub(crate) fn get_kernel_general_memory(&self) -> Vec<U256> {
        self.get_memory_segment(Segment::KernelGeneral)
    }

    pub(crate) fn get_rlp_memory(&self) -> Vec<u8> {
        self.get_memory_segment_bytes(Segment::RlpRaw)
    }

    pub(crate) fn set_current_general_memory(&mut self, memory: Vec<U256>) {
        let context = self.context();
        self.generation_state.memory.contexts[context].segments[Segment::KernelGeneral.unscale()]
            .content = memory;
    }

    pub(crate) fn set_memory_segment(&mut self, segment: Segment, memory: Vec<U256>) {
        self.generation_state.memory.contexts[0].segments[segment.unscale()].content = memory;
    }

    pub(crate) fn set_memory_segment_bytes(&mut self, segment: Segment, memory: Vec<u8>) {
        self.generation_state.memory.contexts[0].segments[segment.unscale()].content =
            memory.into_iter().map(U256::from).collect();
    }

    pub(crate) fn set_rlp_memory(&mut self, rlp: Vec<u8>) {
        self.set_memory_segment_bytes(Segment::RlpRaw, rlp)
    }

    pub(crate) fn set_code(&mut self, context: usize, code: Vec<u8>) {
        assert_ne!(context, 0, "Can't modify kernel code.");
        while self.generation_state.memory.contexts.len() <= context {
            self.generation_state
                .memory
                .contexts
                .push(MemoryContextState::default());
        }
        self.generation_state.memory.contexts[context].segments[Segment::Code.unscale()].content =
            code.into_iter().map(U256::from).collect();
    }

    pub(crate) fn set_memory_multi_addresses(&mut self, addrs: &[(MemoryAddress, U256)]) {
        for &(addr, val) in addrs {
            self.generation_state.memory.set(addr, val);
        }
    }

    pub(crate) fn get_jumpdest_bits(&self, context: usize) -> Vec<bool> {
        self.generation_state.memory.contexts[context].segments[Segment::JumpdestBits.unscale()]
            .content
            .iter()
            .map(|x| x.bit(0))
            .collect()
    }

    pub(crate) fn set_jumpdest_bits(&mut self, context: usize, jumpdest_bits: Vec<bool>) {
        self.generation_state.memory.contexts[context].segments[Segment::JumpdestBits.unscale()]
            .content = jumpdest_bits.iter().map(|&x| u256_from_bool(x)).collect();
        self.generation_state
            .set_proofs_and_jumpdests(HashMap::from([(
                context,
                BTreeSet::from_iter(
                    jumpdest_bits
                        .into_iter()
                        .enumerate()
                        .filter(|&(_, x)| x)
                        .map(|(i, _)| i),
                ),
            )]));
    }

    pub(crate) fn incr(&mut self, n: usize) {
        self.generation_state.registers.program_counter += n;
    }

    pub(crate) fn stack(&self) -> Vec<U256> {
        match self.stack_len().cmp(&1) {
            Ordering::Greater => {
                let mut stack = self.generation_state.memory.contexts[self.context()].segments
                    [Segment::Stack.unscale()]
                .content
                .clone();
                stack.truncate(self.stack_len() - 1);
                stack.push(
                    self.stack_top()
                        .expect("The stack is checked to be nonempty"),
                );
                stack
            }
            Ordering::Equal => {
                vec![self
                    .stack_top()
                    .expect("The stack is checked to be nonempty")]
            }
            Ordering::Less => {
                vec![]
            }
        }
    }
    fn stack_segment_mut(&mut self) -> &mut Vec<U256> {
        let context = self.context();
        &mut self.generation_state.memory.contexts[context].segments[Segment::Stack.unscale()]
            .content
    }

    pub(crate) fn extract_kernel_memory(self, segment: Segment, range: Range<usize>) -> Vec<U256> {
        let mut output: Vec<U256> = vec![];
        for i in range {
            let term = self
                .generation_state
                .memory
                .get(MemoryAddress::new(0, segment, i));
            output.push(term);
        }
        output
    }

    pub(crate) fn push(&mut self, x: U256) -> Result<(), ProgramError> {
        if !self.is_kernel() && self.stack_len() >= MAX_USER_STACK_SIZE {
            return Err(ProgramError::StackOverflow);
        }
        if self.stack_len() > 0 {
            let top = self
                .stack_top()
                .expect("The stack is checked to be nonempty");
            let cur_len = self.stack_len();
            let stack_addr = MemoryAddress::new(self.context(), Segment::Stack, cur_len - 1);
            self.generation_state.memory.set(stack_addr, top);
        }
        self.generation_state.registers.stack_top = x;
        self.generation_state.registers.stack_len += 1;
        self.memops.push(InterpreterMemOpKind::Push(self.context()));

        Ok(())
    }

    fn push_bool(&mut self, x: bool) -> Result<(), ProgramError> {
        self.push(if x { U256::one() } else { U256::zero() })
    }

    pub(crate) fn pop(&mut self) -> Result<U256, ProgramError> {
        let result = stack_peek(&self.generation_state, 0);

        if let Ok(val) = result {
            self.memops
                .push(InterpreterMemOpKind::Pop(val, self.context()));
        }
        if self.stack_len() > 1 {
            let top = stack_peek(&self.generation_state, 1).unwrap();
            self.generation_state.registers.stack_top = top;
        }
        self.generation_state.registers.stack_len -= 1;

        result
    }

    fn run_opcode(&mut self) -> Result<(), ProgramError> {
        let opcode = self
            .code()
            .get(self.generation_state.registers.program_counter)
            .byte(0);
        self.opcode_count[opcode as usize] += 1;
        self.incr(1);
        match opcode {
            0x00 => self.run_syscall(opcode, 0, false), // "STOP",
            0x01 => self.run_add(),                     // "ADD",
            0x02 => self.run_mul(),                     // "MUL",
            0x03 => self.run_sub(),                     // "SUB",
            0x04 => self.run_div(),                     // "DIV",
            0x05 => self.run_syscall(opcode, 2, false), // "SDIV",
            0x06 => self.run_mod(),                     // "MOD",
            0x07 => self.run_syscall(opcode, 2, false), // "SMOD",
            0x08 => self.run_addmod(),                  // "ADDMOD",
            0x09 => self.run_mulmod(),                  // "MULMOD",
            0x0a => self.run_syscall(opcode, 2, false), // "EXP",
            0x0b => self.run_syscall(opcode, 2, false), // "SIGNEXTEND",
            0x0c => self.run_addfp254(),                // "ADDFP254",
            0x0d => self.run_mulfp254(),                // "MULFP254",
            0x0e => self.run_subfp254(),                // "SUBFP254",
            0x0f => self.run_submod(),                  // "SUBMOD",
            0x10 => self.run_lt(),                      // "LT",
            0x11 => self.run_gt(),                      // "GT",
            0x12 => self.run_syscall(opcode, 2, false), // "SLT",
            0x13 => self.run_syscall(opcode, 2, false), // "SGT",
            0x14 => self.run_eq(),                      // "EQ",
            0x15 => self.run_iszero(),                  // "ISZERO",
            0x16 => self.run_and(),                     // "AND",
            0x17 => self.run_or(),                      // "OR",
            0x18 => self.run_xor(),                     // "XOR",
            0x19 => self.run_not(),                     // "NOT",
            0x1a => self.run_byte(),                    // "BYTE",
            0x1b => self.run_shl(),                     // "SHL",
            0x1c => self.run_shr(),                     // "SHR",
            0x1d => self.run_syscall(opcode, 2, false), // "SAR",
            0x20 => self.run_syscall(opcode, 2, false), // "KECCAK256",
            0x21 => self.run_keccak_general(),          // "KECCAK_GENERAL",
            0x30 => self.run_syscall(opcode, 0, true),  // "ADDRESS",
            0x31 => self.run_syscall(opcode, 1, false), // "BALANCE",
            0x32 => self.run_syscall(opcode, 0, true),  // "ORIGIN",
            0x33 => self.run_syscall(opcode, 0, true),  // "CALLER",
            0x34 => self.run_syscall(opcode, 0, true),  // "CALLVALUE",
            0x35 => self.run_syscall(opcode, 1, false), // "CALLDATALOAD",
            0x36 => self.run_syscall(opcode, 0, true),  // "CALLDATASIZE",
            0x37 => self.run_syscall(opcode, 3, false), // "CALLDATACOPY",
            0x38 => self.run_syscall(opcode, 0, true),  // "CODESIZE",
            0x39 => self.run_syscall(opcode, 3, false), // "CODECOPY",
            0x3a => self.run_syscall(opcode, 0, true),  // "GASPRICE",
            0x3b => self.run_syscall(opcode, 1, false), // "EXTCODESIZE",
            0x3c => self.run_syscall(opcode, 4, false), // "EXTCODECOPY",
            0x3d => self.run_syscall(opcode, 0, true),  // "RETURNDATASIZE",
            0x3e => self.run_syscall(opcode, 3, false), // "RETURNDATACOPY",
            0x3f => self.run_syscall(opcode, 1, false), // "EXTCODEHASH",
            0x40 => self.run_syscall(opcode, 1, false), // "BLOCKHASH",
            0x41 => self.run_syscall(opcode, 0, true),  // "COINBASE",
            0x42 => self.run_syscall(opcode, 0, true),  // "TIMESTAMP",
            0x43 => self.run_syscall(opcode, 0, true),  // "NUMBER",
            0x44 => self.run_syscall(opcode, 0, true),  // "DIFFICULTY",
            0x45 => self.run_syscall(opcode, 0, true),  // "GASLIMIT",
            0x46 => self.run_syscall(opcode, 0, true),  // "CHAINID",
            0x47 => self.run_syscall(opcode, 0, true),  // SELFABALANCE,
            0x48 => self.run_syscall(opcode, 0, true),  // "BASEFEE",
            0x49 => self.run_prover_input(),            // "PROVER_INPUT",
            0x50 => self.run_pop(),                     // "POP",
            0x51 => self.run_syscall(opcode, 1, false), // "MLOAD",
            0x52 => self.run_syscall(opcode, 2, false), // "MSTORE",
            0x53 => self.run_syscall(opcode, 2, false), // "MSTORE8",
            0x54 => self.run_syscall(opcode, 1, false), // "SLOAD",
            0x55 => self.run_syscall(opcode, 2, false), // "SSTORE",
            0x56 => self.run_jump(),                    // "JUMP",
            0x57 => self.run_jumpi(),                   // "JUMPI",
            0x58 => self.run_pc(),                      // "PC",
            0x59 => self.run_syscall(opcode, 0, true),  // "MSIZE",
            0x5a => self.run_syscall(opcode, 0, true),  // "GAS",
            0x5b => self.run_jumpdest(),                // "JUMPDEST",
            x if (0x5f..0x80).contains(&x) => self.run_push(x - 0x5f), // "PUSH"
            x if (0x80..0x90).contains(&x) => self.run_dup(x - 0x7f), // "DUP"
            x if (0x90..0xa0).contains(&x) => self.run_swap(x - 0x8f), // "SWAP"
            0xa0 => self.run_syscall(opcode, 2, false), // "LOG0",
            0xa1 => self.run_syscall(opcode, 3, false), // "LOG1",
            0xa2 => self.run_syscall(opcode, 4, false), // "LOG2",
            0xa3 => self.run_syscall(opcode, 5, false), // "LOG3",
            0xa4 => self.run_syscall(opcode, 6, false), // "LOG4",
            0xa5 => {
                log::warn!(
                    "Kernel panic at {}, stack = {:?}, memory = {:?}",
                    KERNEL.offset_name(self.generation_state.registers.program_counter),
                    self.stack(),
                    self.get_kernel_general_memory()
                );
                Err(ProgramError::KernelPanic)
            } // "PANIC",
            x if (0xc0..0xe0).contains(&x) => self.run_mstore_32bytes(x - 0xc0 + 1), // "MSTORE_32BYTES",
            0xf0 => self.run_syscall(opcode, 3, false),                              // "CREATE",
            0xf1 => self.run_syscall(opcode, 7, false),                              // "CALL",
            0xf2 => self.run_syscall(opcode, 7, false),                              // "CALLCODE",
            0xf3 => self.run_syscall(opcode, 2, false),                              // "RETURN",
            0xf4 => self.run_syscall(opcode, 6, false), // "DELEGATECALL",
            0xf5 => self.run_syscall(opcode, 4, false), // "CREATE2",
            0xf6 => self.run_get_context(),             // "GET_CONTEXT",
            0xf7 => self.run_set_context(),             // "SET_CONTEXT",
            0xf8 => self.run_mload_32bytes(),           // "MLOAD_32BYTES",
            0xf9 => self.run_exit_kernel(),             // "EXIT_KERNEL",
            0xfa => self.run_syscall(opcode, 6, false), // "STATICCALL",
            0xfb => self.run_mload_general(),           // "MLOAD_GENERAL",
            0xfc => self.run_mstore_general(),          // "MSTORE_GENERAL",
            0xfd => self.run_syscall(opcode, 2, false), // "REVERT",
            0xfe => {
                log::warn!(
                    "Invalid opcode at {}",
                    KERNEL.offset_name(self.generation_state.registers.program_counter),
                );
                Err(ProgramError::InvalidOpcode)
            } // "INVALID",
            0xff => self.run_syscall(opcode, 1, false), // "SELFDESTRUCT",
            _ => {
                log::warn!(
                    "Unrecognized opcode at {}",
                    KERNEL.offset_name(self.generation_state.registers.program_counter),
                );
                Err(ProgramError::InvalidOpcode)
            }
        }?;

        if self
            .debug_offsets
            .contains(&self.generation_state.registers.program_counter)
        {
            println!("At {}, stack={:?}", self.offset_name(), self.stack());
        } else if let Some(label) = self.offset_label() {
            println!("At {label}");
        }

        let op = decode(self.generation_state.registers, opcode)
            // We default to prover inputs, as those are kernel-only instructions that charge nothing.
            .unwrap_or(Operation::ProverInput);
        self.generation_state.registers.gas_used += gas_to_charge(op);

        if !self.is_kernel() {
            let gas_limit_address = MemoryAddress {
                context: self.context(),
                segment: Segment::ContextMetadata.unscale(),
                virt: ContextMetadata::GasLimit.unscale(),
            };
            let gas_limit =
                u256_to_usize(self.generation_state.memory.get(gas_limit_address))? as u64;
            if self.generation_state.registers.gas_used > gas_limit {
                return Err(ProgramError::OutOfGas);
            }
        }

        Ok(())
    }

    fn offset_name(&self) -> String {
        KERNEL.offset_name(self.generation_state.registers.program_counter)
    }

    fn offset_label(&self) -> Option<String> {
        KERNEL.offset_label(self.generation_state.registers.program_counter)
    }

    fn run_add(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push(x.overflowing_add(y).0)
    }

    fn run_mul(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push(x.overflowing_mul(y).0)
    }

    fn run_sub(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push(x.overflowing_sub(y).0)
    }

    fn run_addfp254(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()? % BN_BASE;
        let y = self.pop()? % BN_BASE;
        // BN_BASE is 254-bit so addition can't overflow
        self.push((x + y) % BN_BASE)
    }

    fn run_mulfp254(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push(
            U256::try_from(x.full_mul(y) % BN_BASE)
                .expect("BN_BASE is 254 bit so the U512 fits in a U256"),
        )
    }

    fn run_subfp254(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()? % BN_BASE;
        let y = self.pop()? % BN_BASE;
        // BN_BASE is 254-bit so addition can't overflow
        self.push((x + (BN_BASE - y)) % BN_BASE)
    }

    fn run_div(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push(if y.is_zero() { U256::zero() } else { x / y })
    }

    fn run_mod(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push(if y.is_zero() { U256::zero() } else { x % y })
    }

    fn run_addmod(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        let z = self.pop()?;
        self.push(if z.is_zero() {
            z
        } else {
            let (x, y, z) = (U512::from(x), U512::from(y), U512::from(z));
            U256::try_from((x + y) % z)
                .expect("Inputs are U256 and their sum mod a U256 fits in a U256.")
        })
    }

    fn run_submod(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        let z = self.pop()?;
        self.push(if z.is_zero() {
            z
        } else {
            let (x, y, z) = (U512::from(x), U512::from(y), U512::from(z));
            U256::try_from((z + x - y) % z)
                .expect("Inputs are U256 and their difference mod a U256 fits in a U256.")
        })
    }

    fn run_mulmod(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        let z = self.pop()?;
        self.push(if z.is_zero() {
            z
        } else {
            U256::try_from(x.full_mul(y) % z)
                .expect("Inputs are U256 and their product mod a U256 fits in a U256.")
        })
    }

    fn run_lt(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push_bool(x < y)
    }

    fn run_gt(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push_bool(x > y)
    }

    fn run_eq(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push_bool(x == y)
    }

    fn run_iszero(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        self.push_bool(x.is_zero())
    }

    fn run_and(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push(x & y)
    }

    fn run_or(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push(x | y)
    }

    fn run_xor(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let y = self.pop()?;
        self.push(x ^ y)
    }

    fn run_not(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        self.push(!x)
    }

    fn run_byte(&mut self) -> anyhow::Result<(), ProgramError> {
        let i = self.pop()?;
        let x = self.pop()?;
        let result = if i < 32.into() {
            x.byte(31 - i.as_usize())
        } else {
            0
        };
        self.push(result.into())
    }

    fn run_shl(&mut self) -> anyhow::Result<(), ProgramError> {
        let shift = self.pop()?;
        let value = self.pop()?;
        self.push(if shift < U256::from(256usize) {
            value << shift
        } else {
            U256::zero()
        })
    }

    fn run_shr(&mut self) -> anyhow::Result<(), ProgramError> {
        let shift = self.pop()?;
        let value = self.pop()?;
        self.push(value >> shift)
    }

    fn run_keccak_general(&mut self) -> anyhow::Result<(), ProgramError> {
        let addr = self.pop()?;
        let (context, segment, offset) = unpack_address!(addr);

        // Not strictly needed but here to avoid surprises with MSIZE.
        assert_ne!(segment, Segment::MainMemory, "Call KECCAK256 instead.");
        let size = self.pop()?.as_usize();
        let bytes = (offset..offset + size)
            .map(|i| {
                self.generation_state
                    .memory
                    .mload_general(context, segment, i)
                    .byte(0)
            })
            .collect::<Vec<_>>();
        println!("Hashing {:?}", &bytes);
        let hash = keccak(bytes);
        self.push(U256::from_big_endian(hash.as_bytes()))
    }

    fn run_prover_input(&mut self) -> Result<(), ProgramError> {
        let prover_input_fn = self
            .prover_inputs_map
            .get(&(self.generation_state.registers.program_counter - 1))
            .ok_or(ProgramError::ProverInputError(
                ProverInputError::InvalidMptInput,
            ))?;
        let output = self.generation_state.prover_input(prover_input_fn)?;
        self.push(output)
    }

    fn run_pop(&mut self) -> anyhow::Result<(), ProgramError> {
        self.pop().map(|_| ())
    }

    fn run_syscall(
        &mut self,
        opcode: u8,
        stack_values_read: usize,
        stack_len_increased: bool,
    ) -> Result<(), ProgramError> {
        TryInto::<u64>::try_into(self.generation_state.registers.gas_used)
            .map_err(|_| ProgramError::GasLimitError)?;
        if self.generation_state.registers.stack_len < stack_values_read {
            return Err(ProgramError::StackUnderflow);
        }

        if stack_len_increased
            && !self.is_kernel()
            && self.generation_state.registers.stack_len >= MAX_USER_STACK_SIZE
        {
            return Err(ProgramError::StackOverflow);
        };

        let handler_jumptable_addr = KERNEL.global_labels["syscall_jumptable"];
        let handler_addr = {
            let offset = handler_jumptable_addr + (opcode as usize) * (BYTES_PER_OFFSET as usize);
            self.get_memory_segment(Segment::Code)[offset..offset + 3]
                .iter()
                .fold(U256::from(0), |acc, &elt| acc * (1 << 8) + elt)
        };

        let new_program_counter =
            u256_to_usize(handler_addr).map_err(|_| ProgramError::IntegerTooLarge)?;

        let syscall_info = U256::from(self.generation_state.registers.program_counter)
            + U256::from((self.is_kernel() as usize) << 32)
            + (U256::from(self.generation_state.registers.gas_used) << 192);
        self.generation_state.registers.program_counter = new_program_counter;

        self.set_is_kernel(true);
        self.generation_state.registers.gas_used = 0;
        self.push(syscall_info)
    }

    fn run_jump(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        // Check that the destination is valid.
        let x: u32 = x
            .try_into()
            .map_err(|_| ProgramError::InvalidJumpDestination)?;
        self.jump_to(x as usize)
    }

    fn run_jumpi(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let b = self.pop()?;
        if !b.is_zero() {
            let x: u32 = x
                .try_into()
                .map_err(|_| ProgramError::InvalidJumpiDestination)?;
            self.jump_to(x as usize)?;
        }

        Ok(())
    }

    fn run_pc(&mut self) -> anyhow::Result<(), ProgramError> {
        self.push(
            (self
                .generation_state
                .registers
                .program_counter
                .saturating_sub(1))
            .into(),
        )
    }

    fn run_jumpdest(&mut self) -> anyhow::Result<(), ProgramError> {
        assert!(!self.is_kernel(), "JUMPDEST is not needed in kernel code");
        Ok(())
    }

    fn jump_to(&mut self, offset: usize) -> anyhow::Result<(), ProgramError> {
        // The JUMPDEST rule is not enforced in kernel mode.
        if !self.is_kernel() && self.jumpdests.binary_search(&offset).is_err() {
            return Err(ProgramError::InvalidJumpDestination);
        }

        self.generation_state.registers.program_counter = offset;

        if self.halt_offsets.contains(&offset) {
            self.running = false;
        }
        Ok(())
    }

    fn run_push(&mut self, num_bytes: u8) -> anyhow::Result<(), ProgramError> {
        let x = U256::from_big_endian(&self.code_slice(num_bytes as usize));
        self.incr(num_bytes as usize);
        self.push(x)
    }

    fn run_dup(&mut self, n: u8) -> anyhow::Result<(), ProgramError> {
        let len = self.stack_len();
        if !self.is_kernel() && len >= MAX_USER_STACK_SIZE {
            return Err(ProgramError::StackOverflow);
        }
        if n as usize > self.stack_len() {
            return Err(ProgramError::StackUnderflow);
        }
        self.push(stack_peek(&self.generation_state, n as usize - 1)?)
    }

    fn run_swap(&mut self, n: u8) -> anyhow::Result<(), ProgramError> {
        let len = self.stack_len();
        if n as usize >= len {
            return Err(ProgramError::StackUnderflow);
        }
        let to_swap = stack_peek(&self.generation_state, n as usize)?;
        let old_value = self.stack_segment_mut()[len - n as usize - 1];

        self.stack_segment_mut()[len - n as usize - 1] = self.stack_top()?;
        let mem_write_op = InterpreterMemOpKind::Write(
            old_value,
            self.context(),
            Segment::Stack.unscale(),
            len - n as usize - 1,
        );
        self.memops.push(mem_write_op);
        self.generation_state.registers.stack_top = to_swap;
        Ok(())
    }

    fn run_get_context(&mut self) -> anyhow::Result<(), ProgramError> {
        self.push(U256::from(self.context()) << CONTEXT_SCALING_FACTOR)
    }

    fn run_set_context(&mut self) -> anyhow::Result<(), ProgramError> {
        let x = self.pop()?;
        let new_ctx = (x >> CONTEXT_SCALING_FACTOR).as_usize();
        let sp_to_save = self.stack_len().into();

        let old_ctx = self.context();

        let sp_field = ContextMetadata::StackSize.unscale();

        let old_sp_addr = MemoryAddress::new(old_ctx, Segment::ContextMetadata, sp_field);
        let new_sp_addr = MemoryAddress::new(new_ctx, Segment::ContextMetadata, sp_field);
        self.generation_state.memory.set(old_sp_addr, sp_to_save);

        let new_sp = self.generation_state.memory.get(new_sp_addr).as_usize();

        if new_sp > 0 {
            let new_stack_top = self.generation_state.memory.contexts[new_ctx].segments
                [Segment::Stack.unscale()]
            .content[new_sp - 1];
            self.generation_state.registers.stack_top = new_stack_top;
        }
        self.set_context(new_ctx);
        self.generation_state.registers.stack_len = new_sp;
        Ok(())
    }

    fn run_mload_general(&mut self) -> anyhow::Result<(), ProgramError> {
        let addr = self.pop()?;
        let (context, segment, offset) = unpack_address!(addr);
        let value = self
            .generation_state
            .memory
            .mload_general(context, segment, offset);
        assert!(value.bits() <= segment.bit_range());
        self.push(value)
    }

    fn run_mload_32bytes(&mut self) -> anyhow::Result<(), ProgramError> {
        let addr = self.pop()?;
        let (context, segment, offset) = unpack_address!(addr);
        let len = self.pop()?.as_usize();
        if len > 32 {
            return Err(ProgramError::IntegerTooLarge);
        }
        let bytes: Vec<u8> = (0..len)
            .map(|i| {
                self.generation_state
                    .memory
                    .mload_general(context, segment, offset + i)
                    .low_u32() as u8
            })
            .collect();
        let value = U256::from_big_endian(&bytes);
        self.push(value)
    }

    fn run_mstore_general(&mut self) -> anyhow::Result<(), ProgramError> {
        let value = self.pop()?;
        let addr = self.pop()?;
        let (context, segment, offset) = unpack_address!(addr);
        let memop = self
            .generation_state
            .memory
            .mstore_general(context, segment, offset, value);
        self.memops.push(memop);
        Ok(())
    }

    fn run_mstore_32bytes(&mut self, n: u8) -> anyhow::Result<(), ProgramError> {
        let addr = self.pop()?;
        let (context, segment, offset) = unpack_address!(addr);
        let value = self.pop()?;

        let mut bytes = vec![0; 32];
        value.to_little_endian(&mut bytes);
        bytes.resize(n as usize, 0);
        bytes.reverse();

        for (i, &byte) in bytes.iter().enumerate() {
            let memop = self.generation_state.memory.mstore_general(
                context,
                segment,
                offset + i,
                byte.into(),
            );
            self.memops.push(memop);
        }

        self.push(addr + U256::from(n))
    }

    fn run_exit_kernel(&mut self) -> anyhow::Result<(), ProgramError> {
        let kexit_info = self.pop()?;

        let kexit_info_u64 = kexit_info.0[0];
        let program_counter = kexit_info_u64 as u32 as usize;
        let is_kernel_mode_val = (kexit_info_u64 >> 32) as u32;
        assert!(is_kernel_mode_val == 0 || is_kernel_mode_val == 1);
        let is_kernel_mode = is_kernel_mode_val != 0;
        let gas_used_val = kexit_info.0[3];
        TryInto::<u64>::try_into(gas_used_val).map_err(|_| ProgramError::GasLimitError)?;

        self.generation_state.registers.program_counter = program_counter;
        self.set_is_kernel(is_kernel_mode);
        self.generation_state.registers.gas_used = gas_used_val;

        Ok(())
    }

    fn run_exception(&mut self, exc_code: u8) -> Result<(), ProgramError> {
        let disallowed_len = MAX_USER_STACK_SIZE + 1;

        if self.stack_len() == disallowed_len {
            // This is a stack overflow that should have been caught earlier.
            return Err(ProgramError::StackOverflow);
        };

        let handler_jumptable_addr = KERNEL.global_labels["exception_jumptable"];
        let handler_addr = {
            let offset = handler_jumptable_addr + (exc_code as usize) * (BYTES_PER_OFFSET as usize);
            assert_eq!(BYTES_PER_OFFSET, 3, "Code below assumes 3 bytes per offset");
            self.get_memory_segment(Segment::Code)[offset..offset + 3]
                .iter()
                .fold(U256::from(0), |acc, &elt| acc * 256 + elt)
        };

        let new_program_counter = u256_to_usize(handler_addr)?;

        let exc_info = U256::from(self.generation_state.registers.program_counter)
            + (U256::from(self.generation_state.registers.gas_used) << 192);

        self.push(exc_info)?;

        // Set registers before pushing to the stack; in particular, we need to set kernel mode so we
        // can't incorrectly trigger a stack overflow. However, note that we have to do it _after_ we
        // make `exc_info`, which should contain the old values.
        self.generation_state.registers.program_counter = new_program_counter;
        self.set_is_kernel(true);
        self.generation_state.registers.gas_used = 0;

        Ok(())
    }

    pub(crate) const fn stack_len(&self) -> usize {
        self.generation_state.registers.stack_len
    }

    pub(crate) fn stack_top(&self) -> anyhow::Result<U256, ProgramError> {
        if self.stack_len() > 0 {
            Ok(self.generation_state.registers.stack_top)
        } else {
            Err(ProgramError::StackUnderflow)
        }
    }

    pub(crate) const fn is_kernel(&self) -> bool {
        self.generation_state.registers.is_kernel
    }

    pub(crate) fn set_is_kernel(&mut self, is_kernel: bool) {
        self.generation_state.registers.is_kernel = is_kernel
    }

    pub(crate) const fn context(&self) -> usize {
        self.generation_state.registers.context
    }

    pub(crate) fn set_context(&mut self, context: usize) {
        if context == 0 {
            assert!(self.is_kernel());
        }
        self.generation_state.registers.context = context;
    }

    /// Writes the encoding of 0 to position @ENCODED_EMPTY_NODE_POS.
    pub(crate) fn initialize_rlp_segment(&mut self) {
        self.generation_state.memory.set(
            MemoryAddress::new(0, Segment::RlpRaw, 0xFFFFFFFF),
            128.into(),
        )
    }
}

// Computes the two's complement of the given integer.
fn two_complement(x: U256) -> U256 {
    let flipped_bits = x ^ MINUS_ONE;
    flipped_bits.overflowing_add(U256::one()).0
}

fn signed_cmp(x: U256, y: U256) -> Ordering {
    let x_is_zero = x.is_zero();
    let y_is_zero = y.is_zero();

    if x_is_zero && y_is_zero {
        return Ordering::Equal;
    }

    let x_is_pos = x.eq(&(x & SIGN_MASK));
    let y_is_pos = y.eq(&(y & SIGN_MASK));

    if x_is_zero {
        if y_is_pos {
            return Ordering::Less;
        } else {
            return Ordering::Greater;
        }
    };

    if y_is_zero {
        if x_is_pos {
            return Ordering::Greater;
        } else {
            return Ordering::Less;
        }
    };

    match (x_is_pos, y_is_pos) {
        (true, true) => x.cmp(&y),
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => x.cmp(&y).reverse(),
    }
}

/// -1 in two's complement representation consists in all bits set to 1.
const MINUS_ONE: U256 = U256([
    0xffffffffffffffff,
    0xffffffffffffffff,
    0xffffffffffffffff,
    0xffffffffffffffff,
]);

/// -2^255 in two's complement representation consists in the MSB set to 1.
const MIN_VALUE: U256 = U256([
    0x0000000000000000,
    0x0000000000000000,
    0x0000000000000000,
    0x8000000000000000,
]);

const SIGN_MASK: U256 = U256([
    0xffffffffffffffff,
    0xffffffffffffffff,
    0xffffffffffffffff,
    0x7fffffffffffffff,
]);

/// Return the (ordered) JUMPDEST offsets in the code.
fn find_jumpdests(code: &[u8]) -> Vec<usize> {
    let mut offset = 0;
    let mut res = Vec::new();
    while offset < code.len() {
        let opcode = code[offset];
        match opcode {
            0x5b => res.push(offset),
            x if (0x60..0x80).contains(&x) => offset += x as usize - 0x5f, // PUSH instruction, disregard data.
            _ => (),
        }
        offset += 1;
    }
    res
}

fn get_mnemonic(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "STOP",
        0x01 => "ADD",
        0x02 => "MUL",
        0x03 => "SUB",
        0x04 => "DIV",
        0x05 => "SDIV",
        0x06 => "MOD",
        0x07 => "SMOD",
        0x08 => "ADDMOD",
        0x09 => "MULMOD",
        0x0a => "EXP",
        0x0b => "SIGNEXTEND",
        0x0c => "ADDFP254",
        0x0d => "MULFP254",
        0x0e => "SUBFP254",
        0x0f => "SUBMOD",
        0x10 => "LT",
        0x11 => "GT",
        0x12 => "SLT",
        0x13 => "SGT",
        0x14 => "EQ",
        0x15 => "ISZERO",
        0x16 => "AND",
        0x17 => "OR",
        0x18 => "XOR",
        0x19 => "NOT",
        0x1a => "BYTE",
        0x1b => "SHL",
        0x1c => "SHR",
        0x1d => "SAR",
        0x20 => "KECCAK256",
        0x21 => "KECCAK_GENERAL",
        0x30 => "ADDRESS",
        0x31 => "BALANCE",
        0x32 => "ORIGIN",
        0x33 => "CALLER",
        0x34 => "CALLVALUE",
        0x35 => "CALLDATALOAD",
        0x36 => "CALLDATASIZE",
        0x37 => "CALLDATACOPY",
        0x38 => "CODESIZE",
        0x39 => "CODECOPY",
        0x3a => "GASPRICE",
        0x3b => "EXTCODESIZE",
        0x3c => "EXTCODECOPY",
        0x3d => "RETURNDATASIZE",
        0x3e => "RETURNDATACOPY",
        0x3f => "EXTCODEHASH",
        0x40 => "BLOCKHASH",
        0x41 => "COINBASE",
        0x42 => "TIMESTAMP",
        0x43 => "NUMBER",
        0x44 => "DIFFICULTY",
        0x45 => "GASLIMIT",
        0x46 => "CHAINID",
        0x48 => "BASEFEE",
        0x49 => "PROVER_INPUT",
        0x50 => "POP",
        0x51 => "MLOAD",
        0x52 => "MSTORE",
        0x53 => "MSTORE8",
        0x54 => "SLOAD",
        0x55 => "SSTORE",
        0x56 => "JUMP",
        0x57 => "JUMPI",
        0x58 => "GETPC",
        0x59 => "MSIZE",
        0x5a => "GAS",
        0x5b => "JUMPDEST",
        0x5f => "PUSH0",
        0x60 => "PUSH1",
        0x61 => "PUSH2",
        0x62 => "PUSH3",
        0x63 => "PUSH4",
        0x64 => "PUSH5",
        0x65 => "PUSH6",
        0x66 => "PUSH7",
        0x67 => "PUSH8",
        0x68 => "PUSH9",
        0x69 => "PUSH10",
        0x6a => "PUSH11",
        0x6b => "PUSH12",
        0x6c => "PUSH13",
        0x6d => "PUSH14",
        0x6e => "PUSH15",
        0x6f => "PUSH16",
        0x70 => "PUSH17",
        0x71 => "PUSH18",
        0x72 => "PUSH19",
        0x73 => "PUSH20",
        0x74 => "PUSH21",
        0x75 => "PUSH22",
        0x76 => "PUSH23",
        0x77 => "PUSH24",
        0x78 => "PUSH25",
        0x79 => "PUSH26",
        0x7a => "PUSH27",
        0x7b => "PUSH28",
        0x7c => "PUSH29",
        0x7d => "PUSH30",
        0x7e => "PUSH31",
        0x7f => "PUSH32",
        0x80 => "DUP1",
        0x81 => "DUP2",
        0x82 => "DUP3",
        0x83 => "DUP4",
        0x84 => "DUP5",
        0x85 => "DUP6",
        0x86 => "DUP7",
        0x87 => "DUP8",
        0x88 => "DUP9",
        0x89 => "DUP10",
        0x8a => "DUP11",
        0x8b => "DUP12",
        0x8c => "DUP13",
        0x8d => "DUP14",
        0x8e => "DUP15",
        0x8f => "DUP16",
        0x90 => "SWAP1",
        0x91 => "SWAP2",
        0x92 => "SWAP3",
        0x93 => "SWAP4",
        0x94 => "SWAP5",
        0x95 => "SWAP6",
        0x96 => "SWAP7",
        0x97 => "SWAP8",
        0x98 => "SWAP9",
        0x99 => "SWAP10",
        0x9a => "SWAP11",
        0x9b => "SWAP12",
        0x9c => "SWAP13",
        0x9d => "SWAP14",
        0x9e => "SWAP15",
        0x9f => "SWAP16",
        0xa0 => "LOG0",
        0xa1 => "LOG1",
        0xa2 => "LOG2",
        0xa3 => "LOG3",
        0xa4 => "LOG4",
        0xa5 => "PANIC",
        0xc0 => "MSTORE_32BYTES_1",
        0xc1 => "MSTORE_32BYTES_2",
        0xc2 => "MSTORE_32BYTES_3",
        0xc3 => "MSTORE_32BYTES_4",
        0xc4 => "MSTORE_32BYTES_5",
        0xc5 => "MSTORE_32BYTES_6",
        0xc6 => "MSTORE_32BYTES_7",
        0xc7 => "MSTORE_32BYTES_8",
        0xc8 => "MSTORE_32BYTES_9",
        0xc9 => "MSTORE_32BYTES_10",
        0xca => "MSTORE_32BYTES_11",
        0xcb => "MSTORE_32BYTES_12",
        0xcc => "MSTORE_32BYTES_13",
        0xcd => "MSTORE_32BYTES_14",
        0xce => "MSTORE_32BYTES_15",
        0xcf => "MSTORE_32BYTES_16",
        0xd0 => "MSTORE_32BYTES_17",
        0xd1 => "MSTORE_32BYTES_18",
        0xd2 => "MSTORE_32BYTES_19",
        0xd3 => "MSTORE_32BYTES_20",
        0xd4 => "MSTORE_32BYTES_21",
        0xd5 => "MSTORE_32BYTES_22",
        0xd6 => "MSTORE_32BYTES_23",
        0xd7 => "MSTORE_32BYTES_24",
        0xd8 => "MSTORE_32BYTES_25",
        0xd9 => "MSTORE_32BYTES_26",
        0xda => "MSTORE_32BYTES_27",
        0xdb => "MSTORE_32BYTES_28",
        0xdc => "MSTORE_32BYTES_29",
        0xdd => "MSTORE_32BYTES_30",
        0xde => "MSTORE_32BYTES_31",
        0xdf => "MSTORE_32BYTES_32",
        0xf0 => "CREATE",
        0xf1 => "CALL",
        0xf2 => "CALLCODE",
        0xf3 => "RETURN",
        0xf4 => "DELEGATECALL",
        0xf5 => "CREATE2",
        0xf6 => "GET_CONTEXT",
        0xf7 => "SET_CONTEXT",
        0xf8 => "MLOAD_32BYTES",
        0xf9 => "EXIT_KERNEL",
        0xfa => "STATICCALL",
        0xfb => "MLOAD_GENERAL",
        0xfc => "MSTORE_GENERAL",
        0xfd => "REVERT",
        0xfe => "INVALID",
        0xff => "SELFDESTRUCT",
        _ => panic!("Unrecognized opcode {opcode}"),
    }
}

#[macro_use]
macro_rules! unpack_address {
    ($addr:ident) => {{
        let offset = $addr.low_u32() as usize;
        let segment = Segment::all()[($addr >> SEGMENT_SCALING_FACTOR).low_u32() as usize];
        let context = ($addr >> CONTEXT_SCALING_FACTOR).low_u32() as usize;
        (context, segment, offset)
    }};
}
pub(crate) use unpack_address;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ethereum_types::U256;

    use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
    use crate::cpu::kernel::interpreter::{run, Interpreter};
    use crate::memory::segments::Segment;
    use crate::witness::memory::MemoryAddress;
    use crate::witness::operation::CONTEXT_SCALING_FACTOR;

    #[test]
    fn test_run() -> anyhow::Result<()> {
        let code = vec![
            0x60, 0x1, 0x60, 0x2, 0x1, 0x63, 0xde, 0xad, 0xbe, 0xef, 0x56,
        ]; // PUSH1, 1, PUSH1, 2, ADD, PUSH4 deadbeef, JUMP
        assert_eq!(
            run(&code, 0, vec![], &HashMap::new())?.stack(),
            &[0x3.into()],
        );
        Ok(())
    }

    #[test]
    fn test_run_with_memory() -> anyhow::Result<()> {
        //         PUSH1 0xff
        //         PUSH1 0
        //         MSTORE

        //         PUSH1 0
        //         MLOAD

        //         PUSH1 1
        //         MLOAD

        //         PUSH1 0x42
        //         PUSH1 0x27
        //         MSTORE8
        let code = [
            0x60, 0xff, 0x60, 0x0, 0x52, 0x60, 0, 0x51, 0x60, 0x1, 0x51, 0x60, 0x42, 0x60, 0x27,
            0x53,
        ];
        let mut interpreter = Interpreter::new_with_kernel(0, vec![]);

        interpreter.set_code(1, code.to_vec());

        interpreter.generation_state.memory.contexts[1].segments
            [Segment::ContextMetadata.unscale()]
        .set(ContextMetadata::GasLimit.unscale(), 100_000.into());
        // Set context and kernel mode.
        interpreter.set_context(1);
        interpreter.set_is_kernel(false);
        // Set memory necessary to sys_stop.
        interpreter.generation_state.memory.set(
            MemoryAddress::new(
                1,
                Segment::ContextMetadata,
                ContextMetadata::ParentProgramCounter.unscale(),
            ),
            0xdeadbeefu32.into(),
        );
        interpreter.generation_state.memory.set(
            MemoryAddress::new(
                1,
                Segment::ContextMetadata,
                ContextMetadata::ParentContext.unscale(),
            ),
            U256::one() << CONTEXT_SCALING_FACTOR,
        );

        interpreter.run()?;

        // sys_stop returns `success` and `cum_gas_used`, that we need to pop.
        interpreter.pop();
        interpreter.pop();

        assert_eq!(interpreter.stack(), &[0xff.into(), 0xff00.into()]);
        assert_eq!(
            interpreter.generation_state.memory.contexts[1].segments[Segment::MainMemory.unscale()]
                .get(0x27),
            0x42.into()
        );
        assert_eq!(
            interpreter.generation_state.memory.contexts[1].segments[Segment::MainMemory.unscale()]
                .get(0x1f),
            0xff.into()
        );
        Ok(())
    }
}
