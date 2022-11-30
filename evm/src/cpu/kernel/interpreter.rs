//! An EVM interpreter for testing and debugging purposes.

use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure};
use ethereum_types::{U256, U512};
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::txn_fields::NormalizedTxnField;
use crate::generation::memory::{MemoryContextState, MemorySegmentState};
use crate::generation::prover_input::ProverInputFn;
use crate::generation::state::GenerationState;
use crate::generation::GenerationInputs;
use crate::memory::segments::Segment;

type F = GoldilocksField;

/// Halt interpreter execution whenever a jump to this offset is done.
const DEFAULT_HALT_OFFSET: usize = 0xdeadbeef;

#[derive(Clone, Debug)]
pub(crate) struct InterpreterMemory {
    pub(crate) context_memory: Vec<MemoryContextState>,
}

impl Default for InterpreterMemory {
    fn default() -> Self {
        Self {
            context_memory: vec![MemoryContextState::default()],
        }
    }
}

impl InterpreterMemory {
    fn with_code_and_stack(code: &[u8], stack: Vec<U256>) -> Self {
        let mut mem = Self::default();
        for (i, b) in code.iter().copied().enumerate() {
            mem.context_memory[0].segments[Segment::Code as usize].set(i, b.into());
        }
        mem.context_memory[0].segments[Segment::Stack as usize].content = stack;

        mem
    }

    fn mload_general(&self, context: usize, segment: Segment, offset: usize) -> U256 {
        let value = self.context_memory[context].segments[segment as usize].get(offset);
        assert!(
            value.bits() <= segment.bit_range(),
            "Value read from memory exceeds expected range of {:?} segment",
            segment
        );
        value
    }

    fn mstore_general(&mut self, context: usize, segment: Segment, offset: usize, value: U256) {
        assert!(
            value.bits() <= segment.bit_range(),
            "Value written to memory exceeds expected range of {:?} segment",
            segment
        );
        self.context_memory[context].segments[segment as usize].set(offset, value)
    }
}

pub struct Interpreter<'a> {
    kernel_mode: bool,
    jumpdests: Vec<usize>,
    pub(crate) offset: usize,
    pub(crate) context: usize,
    pub(crate) memory: InterpreterMemory,
    pub(crate) generation_state: GenerationState<F>,
    prover_inputs_map: &'a HashMap<usize, ProverInputFn>,
    pub(crate) halt_offsets: Vec<usize>,
    pub(crate) debug_offsets: Vec<usize>,
    running: bool,
    opcode_count: [usize; 0x100],
}

pub fn run_interpreter(
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

pub fn run<'a>(
    code: &'a [u8],
    initial_offset: usize,
    initial_stack: Vec<U256>,
    prover_inputs: &'a HashMap<usize, ProverInputFn>,
) -> anyhow::Result<Interpreter<'a>> {
    let mut interpreter = Interpreter::new(code, initial_offset, initial_stack, prover_inputs);
    interpreter.run()?;
    Ok(interpreter)
}

impl<'a> Interpreter<'a> {
    pub(crate) fn new_with_kernel(initial_offset: usize, initial_stack: Vec<U256>) -> Self {
        Self::new(
            &KERNEL.code,
            initial_offset,
            initial_stack,
            &KERNEL.prover_inputs,
        )
    }

    pub(crate) fn new(
        code: &'a [u8],
        initial_offset: usize,
        initial_stack: Vec<U256>,
        prover_inputs: &'a HashMap<usize, ProverInputFn>,
    ) -> Self {
        Self {
            kernel_mode: true,
            jumpdests: find_jumpdests(code),
            offset: initial_offset,
            memory: InterpreterMemory::with_code_and_stack(code, initial_stack),
            generation_state: GenerationState::new(GenerationInputs::default()),
            prover_inputs_map: prover_inputs,
            context: 0,
            halt_offsets: vec![DEFAULT_HALT_OFFSET],
            debug_offsets: vec![],
            running: false,
            opcode_count: [0; 0x100],
        }
    }

    pub(crate) fn run(&mut self) -> anyhow::Result<()> {
        self.running = true;
        while self.running {
            self.run_opcode()?;
        }
        println!("Opcode count:");
        for i in 0..0x100 {
            if self.opcode_count[i] > 0 {
                println!("{}: {}", get_mnemonic(i as u8), self.opcode_count[i])
            }
        }
        Ok(())
    }

    fn code(&self) -> &MemorySegmentState {
        &self.memory.context_memory[self.context].segments[Segment::Code as usize]
    }

    fn code_slice(&self, n: usize) -> Vec<u8> {
        self.code().content[self.offset..self.offset + n]
            .iter()
            .map(|u256| u256.byte(0))
            .collect::<Vec<_>>()
    }

    pub(crate) fn get_txn_field(&self, field: NormalizedTxnField) -> U256 {
        self.memory.context_memory[0].segments[Segment::TxnFields as usize].get(field as usize)
    }

    pub(crate) fn set_txn_field(&mut self, field: NormalizedTxnField, value: U256) {
        self.memory.context_memory[0].segments[Segment::TxnFields as usize]
            .set(field as usize, value);
    }

    pub(crate) fn get_txn_data(&self) -> &[U256] {
        &self.memory.context_memory[0].segments[Segment::TxnData as usize].content
    }

    pub(crate) fn get_global_metadata_field(&self, field: GlobalMetadata) -> U256 {
        self.memory.context_memory[0].segments[Segment::GlobalMetadata as usize].get(field as usize)
    }

    pub(crate) fn set_global_metadata_field(&mut self, field: GlobalMetadata, value: U256) {
        self.memory.context_memory[0].segments[Segment::GlobalMetadata as usize]
            .set(field as usize, value)
    }

    pub(crate) fn get_trie_data(&self) -> &[U256] {
        &self.memory.context_memory[0].segments[Segment::TrieData as usize].content
    }

    pub(crate) fn get_trie_data_mut(&mut self) -> &mut Vec<U256> {
        &mut self.memory.context_memory[0].segments[Segment::TrieData as usize].content
    }

    pub(crate) fn get_rlp_memory(&self) -> Vec<u8> {
        self.memory.context_memory[0].segments[Segment::RlpRaw as usize]
            .content
            .iter()
            .map(|x| x.as_u32() as u8)
            .collect()
    }

    pub(crate) fn set_rlp_memory(&mut self, rlp: Vec<u8>) {
        self.memory.context_memory[0].segments[Segment::RlpRaw as usize].content =
            rlp.into_iter().map(U256::from).collect();
    }

    pub(crate) fn set_code(&mut self, context: usize, code: Vec<u8>) {
        assert_ne!(context, 0, "Can't modify kernel code.");
        while self.memory.context_memory.len() <= context {
            self.memory
                .context_memory
                .push(MemoryContextState::default());
        }
        self.memory.context_memory[context].segments[Segment::Code as usize].content =
            code.into_iter().map(U256::from).collect();
    }

    pub(crate) fn get_jumpdest_bits(&self, context: usize) -> Vec<bool> {
        self.memory.context_memory[context].segments[Segment::JumpdestBits as usize]
            .content
            .iter()
            .map(|x| x.bit(0))
            .collect()
    }

    fn incr(&mut self, n: usize) {
        self.offset += n;
    }

    pub(crate) fn stack(&self) -> &[U256] {
        &self.memory.context_memory[self.context].segments[Segment::Stack as usize].content
    }

    fn stack_mut(&mut self) -> &mut Vec<U256> {
        &mut self.memory.context_memory[self.context].segments[Segment::Stack as usize].content
    }

    pub(crate) fn push(&mut self, x: U256) {
        self.stack_mut().push(x);
    }

    fn push_bool(&mut self, x: bool) {
        self.push(if x { U256::one() } else { U256::zero() });
    }

    pub(crate) fn pop(&mut self) -> U256 {
        self.stack_mut().pop().expect("Pop on empty stack.")
    }

    fn run_opcode(&mut self) -> anyhow::Result<()> {
        let opcode = self.code().get(self.offset).byte(0);
        self.opcode_count[opcode as usize] += 1;
        self.incr(1);
        match opcode {
            0x00 => self.run_stop(),                                    // "STOP",
            0x01 => self.run_add(),                                     // "ADD",
            0x02 => self.run_mul(),                                     // "MUL",
            0x03 => self.run_sub(),                                     // "SUB",
            0x04 => self.run_div(),                                     // "DIV",
            0x05 => todo!(),                                            // "SDIV",
            0x06 => self.run_mod(),                                     // "MOD",
            0x07 => todo!(),                                            // "SMOD",
            0x08 => self.run_addmod(),                                  // "ADDMOD",
            0x09 => self.run_mulmod(),                                  // "MULMOD",
            0x0a => self.run_exp(),                                     // "EXP",
            0x0b => todo!(),                                            // "SIGNEXTEND",
            0x0c => self.run_addfp254(),                                // "ADDFP254",
            0x0d => self.run_mulfp254(),                                // "MULFP254",
            0x0e => self.run_subfp254(),                                // "SUBFP254",
            0x10 => self.run_lt(),                                      // "LT",
            0x11 => self.run_gt(),                                      // "GT",
            0x12 => todo!(),                                            // "SLT",
            0x13 => todo!(),                                            // "SGT",
            0x14 => self.run_eq(),                                      // "EQ",
            0x15 => self.run_iszero(),                                  // "ISZERO",
            0x16 => self.run_and(),                                     // "AND",
            0x17 => self.run_or(),                                      // "OR",
            0x18 => self.run_xor(),                                     // "XOR",
            0x19 => self.run_not(),                                     // "NOT",
            0x1a => self.run_byte(),                                    // "BYTE",
            0x1b => self.run_shl(),                                     // "SHL",
            0x1c => self.run_shr(),                                     // "SHR",
            0x1d => todo!(),                                            // "SAR",
            0x20 => self.run_keccak256(),                               // "KECCAK256",
            0x21 => self.run_keccak_general(),                          // "KECCAK_GENERAL",
            0x30 => todo!(),                                            // "ADDRESS",
            0x31 => todo!(),                                            // "BALANCE",
            0x32 => todo!(),                                            // "ORIGIN",
            0x33 => todo!(),                                            // "CALLER",
            0x34 => self.run_callvalue(),                               // "CALLVALUE",
            0x35 => self.run_calldataload(),                            // "CALLDATALOAD",
            0x36 => self.run_calldatasize(),                            // "CALLDATASIZE",
            0x37 => self.run_calldatacopy(),                            // "CALLDATACOPY",
            0x38 => todo!(),                                            // "CODESIZE",
            0x39 => todo!(),                                            // "CODECOPY",
            0x3a => todo!(),                                            // "GASPRICE",
            0x3b => todo!(),                                            // "EXTCODESIZE",
            0x3c => todo!(),                                            // "EXTCODECOPY",
            0x3d => todo!(),                                            // "RETURNDATASIZE",
            0x3e => todo!(),                                            // "RETURNDATACOPY",
            0x3f => todo!(),                                            // "EXTCODEHASH",
            0x40 => todo!(),                                            // "BLOCKHASH",
            0x41 => todo!(),                                            // "COINBASE",
            0x42 => todo!(),                                            // "TIMESTAMP",
            0x43 => todo!(),                                            // "NUMBER",
            0x44 => todo!(),                                            // "DIFFICULTY",
            0x45 => todo!(),                                            // "GASLIMIT",
            0x46 => todo!(),                                            // "CHAINID",
            0x48 => todo!(),                                            // "BASEFEE",
            0x49 => self.run_prover_input()?,                           // "PROVER_INPUT",
            0x50 => self.run_pop(),                                     // "POP",
            0x51 => self.run_mload(),                                   // "MLOAD",
            0x52 => self.run_mstore(),                                  // "MSTORE",
            0x53 => self.run_mstore8(),                                 // "MSTORE8",
            0x54 => todo!(),                                            // "SLOAD",
            0x55 => todo!(),                                            // "SSTORE",
            0x56 => self.run_jump(),                                    // "JUMP",
            0x57 => self.run_jumpi(),                                   // "JUMPI",
            0x58 => self.run_pc(),                                      // "PC",
            0x59 => self.run_msize(),                                   // "MSIZE",
            0x5a => todo!(),                                            // "GAS",
            0x5b => self.run_jumpdest(),                                // "JUMPDEST",
            0x5c => todo!(),                                            // "GET_STATE_ROOT",
            0x5d => todo!(),                                            // "SET_STATE_ROOT",
            0x5e => todo!(),                                            // "GET_RECEIPT_ROOT",
            0x5f => todo!(),                                            // "SET_RECEIPT_ROOT",
            x if (0x60..0x80).contains(&x) => self.run_push(x - 0x5f),  // "PUSH"
            x if (0x80..0x90).contains(&x) => self.run_dup(x - 0x7f),   // "DUP"
            x if (0x90..0xa0).contains(&x) => self.run_swap(x - 0x8f)?, // "SWAP"
            0xa0 => todo!(),                                            // "LOG0",
            0xa1 => todo!(),                                            // "LOG1",
            0xa2 => todo!(),                                            // "LOG2",
            0xa3 => todo!(),                                            // "LOG3",
            0xa4 => todo!(),                                            // "LOG4",
            0xa5 => bail!("Executed PANIC"),                            // "PANIC",
            0xf0 => todo!(),                                            // "CREATE",
            0xf1 => todo!(),                                            // "CALL",
            0xf2 => todo!(),                                            // "CALLCODE",
            0xf3 => todo!(),                                            // "RETURN",
            0xf4 => todo!(),                                            // "DELEGATECALL",
            0xf5 => todo!(),                                            // "CREATE2",
            0xf6 => self.run_get_context(),                             // "GET_CONTEXT",
            0xf7 => self.run_set_context(),                             // "SET_CONTEXT",
            0xf8 => todo!(),                                            // "CONSUME_GAS",
            0xf9 => todo!(),                                            // "EXIT_KERNEL",
            0xfa => todo!(),                                            // "STATICCALL",
            0xfb => self.run_mload_general(),                           // "MLOAD_GENERAL",
            0xfc => self.run_mstore_general(),                          // "MSTORE_GENERAL",
            0xfd => todo!(),                                            // "REVERT",
            0xfe => bail!("Executed INVALID"),                          // "INVALID",
            0xff => todo!(),                                            // "SELFDESTRUCT",
            _ => bail!("Unrecognized opcode {}.", opcode),
        };

        if self.debug_offsets.contains(&self.offset) {
            println!("At {}, stack={:?}", self.offset_name(), self.stack());
        } else if let Some(label) = self.offset_label() {
            println!("At {label}");
        }

        Ok(())
    }

    /// Get a string representation of the current offset for debugging purposes.
    fn offset_name(&self) -> String {
        self.offset_label()
            .unwrap_or_else(|| self.offset.to_string())
    }

    fn offset_label(&self) -> Option<String> {
        // TODO: Not sure we should use KERNEL? Interpreter is more general in other places.
        KERNEL
            .global_labels
            .iter()
            .find_map(|(k, v)| (*v == self.offset).then(|| k.clone()))
    }

    fn run_stop(&mut self) {
        self.running = false;
    }

    fn run_add(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x.overflowing_add(y).0);
    }

    fn run_mul(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x.overflowing_mul(y).0);
    }

    fn run_sub(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x.overflowing_sub(y).0);
    }

    // TODO: 107 is hardcoded as a dummy prime for testing
    // should be changed to the proper implementation prime

    fn run_addfp254(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push((x + y) % 107);
    }

    fn run_mulfp254(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(U256::try_from(x.full_mul(y) % 107).unwrap());
    }

    fn run_subfp254(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push((U256::from(107) + x - y) % 107);
    }

    fn run_div(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(if y.is_zero() { U256::zero() } else { x / y });
    }

    fn run_mod(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(if y.is_zero() { U256::zero() } else { x % y });
    }

    fn run_addmod(&mut self) {
        let x = U512::from(self.pop());
        let y = U512::from(self.pop());
        let z = U512::from(self.pop());
        self.push(if z.is_zero() {
            U256::zero()
        } else {
            U256::try_from((x + y) % z).unwrap()
        });
    }

    fn run_mulmod(&mut self) {
        let x = self.pop();
        let y = self.pop();
        let z = U512::from(self.pop());
        self.push(if z.is_zero() {
            U256::zero()
        } else {
            U256::try_from(x.full_mul(y) % z).unwrap()
        });
    }

    fn run_exp(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x.overflowing_pow(y).0);
    }

    fn run_lt(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push_bool(x < y);
    }

    fn run_gt(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push_bool(x > y);
    }

    fn run_eq(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push_bool(x == y);
    }

    fn run_iszero(&mut self) {
        let x = self.pop();
        self.push_bool(x.is_zero());
    }

    fn run_and(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x & y);
    }

    fn run_or(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x | y);
    }

    fn run_xor(&mut self) {
        let x = self.pop();
        let y = self.pop();
        self.push(x ^ y);
    }

    fn run_not(&mut self) {
        let x = self.pop();
        self.push(!x);
    }

    fn run_byte(&mut self) {
        let i = self.pop();
        let x = self.pop();
        let result = if i > 32.into() {
            0
        } else {
            let mut bytes = [0; 32];
            x.to_big_endian(&mut bytes);
            bytes[i.as_usize()]
        };
        self.push(result.into());
    }

    fn run_shl(&mut self) {
        let shift = self.pop();
        let value = self.pop();
        self.push(value << shift);
    }

    fn run_shr(&mut self) {
        let shift = self.pop();
        let value = self.pop();
        self.push(value >> shift);
    }

    fn run_keccak256(&mut self) {
        let offset = self.pop().as_usize();
        let size = self.pop().as_usize();
        let bytes = (offset..offset + size)
            .map(|i| {
                self.memory
                    .mload_general(self.context, Segment::MainMemory, i)
                    .byte(0)
            })
            .collect::<Vec<_>>();
        let hash = keccak(bytes);
        self.push(U256::from_big_endian(hash.as_bytes()));
    }

    fn run_keccak_general(&mut self) {
        let context = self.pop().as_usize();
        let segment = Segment::all()[self.pop().as_usize()];
        // Not strictly needed but here to avoid surprises with MSIZE.
        assert_ne!(segment, Segment::MainMemory, "Call KECCAK256 instead.");
        let offset = self.pop().as_usize();
        let size = self.pop().as_usize();
        let bytes = (offset..offset + size)
            .map(|i| self.memory.mload_general(context, segment, i).byte(0))
            .collect::<Vec<_>>();
        println!("Hashing {:?}", &bytes);
        let hash = keccak(bytes);
        self.push(U256::from_big_endian(hash.as_bytes()));
    }

    fn run_callvalue(&mut self) {
        self.push(
            self.memory.context_memory[self.context].segments[Segment::ContextMetadata as usize]
                .get(ContextMetadata::CallValue as usize),
        )
    }

    fn run_calldataload(&mut self) {
        let offset = self.pop().as_usize();
        let value = U256::from_big_endian(
            &(0..32)
                .map(|i| {
                    self.memory
                        .mload_general(self.context, Segment::Calldata, offset + i)
                        .byte(0)
                })
                .collect::<Vec<_>>(),
        );
        self.push(value);
    }

    fn run_calldatasize(&mut self) {
        self.push(
            self.memory.context_memory[self.context].segments[Segment::ContextMetadata as usize]
                .get(ContextMetadata::CalldataSize as usize),
        )
    }

    fn run_calldatacopy(&mut self) {
        let dest_offset = self.pop().as_usize();
        let offset = self.pop().as_usize();
        let size = self.pop().as_usize();
        for i in 0..size {
            let calldata_byte =
                self.memory
                    .mload_general(self.context, Segment::Calldata, offset + i);
            self.memory.mstore_general(
                self.context,
                Segment::MainMemory,
                dest_offset + i,
                calldata_byte,
            );
        }
    }

    fn run_prover_input(&mut self) -> anyhow::Result<()> {
        let prover_input_fn = self
            .prover_inputs_map
            .get(&(self.offset - 1))
            .ok_or_else(|| anyhow!("Offset not in prover inputs."))?;
        let stack = self.stack().to_vec();
        let output = self.generation_state.prover_input(&stack, prover_input_fn);
        self.push(output);
        Ok(())
    }

    fn run_pop(&mut self) {
        self.pop();
    }

    fn run_mload(&mut self) {
        let offset = self.pop().as_usize();
        let value = U256::from_big_endian(
            &(0..32)
                .map(|i| {
                    self.memory
                        .mload_general(self.context, Segment::MainMemory, offset + i)
                        .byte(0)
                })
                .collect::<Vec<_>>(),
        );
        self.push(value);
    }

    fn run_mstore(&mut self) {
        let offset = self.pop().as_usize();
        let value = self.pop();
        let mut bytes = [0; 32];
        value.to_big_endian(&mut bytes);
        for (i, byte) in (0..32).zip(bytes) {
            self.memory
                .mstore_general(self.context, Segment::MainMemory, offset + i, byte.into());
        }
    }

    fn run_mstore8(&mut self) {
        let offset = self.pop().as_usize();
        let value = self.pop();
        self.memory.mstore_general(
            self.context,
            Segment::MainMemory,
            offset,
            value.byte(0).into(),
        );
    }

    fn run_jump(&mut self) {
        let x = self.pop().as_usize();
        self.jump_to(x);
    }

    fn run_jumpi(&mut self) {
        let x = self.pop().as_usize();
        let b = self.pop();
        if !b.is_zero() {
            self.jump_to(x);
        }
    }

    fn run_pc(&mut self) {
        self.push((self.offset - 1).into());
    }

    fn run_msize(&mut self) {
        self.push(
            self.memory.context_memory[self.context].segments[Segment::ContextMetadata as usize]
                .get(ContextMetadata::MSize as usize),
        )
    }

    fn run_jumpdest(&mut self) {
        assert!(!self.kernel_mode, "JUMPDEST is not needed in kernel code");
    }

    fn jump_to(&mut self, offset: usize) {
        // The JUMPDEST rule is not enforced in kernel mode.
        if !self.kernel_mode && self.jumpdests.binary_search(&offset).is_err() {
            panic!("Destination is not a JUMPDEST.");
        }

        self.offset = offset;

        if self.halt_offsets.contains(&offset) {
            self.running = false;
        }
    }

    fn run_push(&mut self, num_bytes: u8) {
        let x = U256::from_big_endian(&self.code_slice(num_bytes as usize));
        self.incr(num_bytes as usize);
        self.push(x);
    }

    fn run_dup(&mut self, n: u8) {
        self.push(self.stack()[self.stack().len() - n as usize]);
    }

    fn run_swap(&mut self, n: u8) -> anyhow::Result<()> {
        let len = self.stack().len();
        ensure!(len > n as usize);
        self.stack_mut().swap(len - 1, len - n as usize - 1);
        Ok(())
    }

    fn run_get_context(&mut self) {
        self.push(self.context.into());
    }

    fn run_set_context(&mut self) {
        let x = self.pop();
        self.context = x.as_usize();
    }

    fn run_mload_general(&mut self) {
        let context = self.pop().as_usize();
        let segment = Segment::all()[self.pop().as_usize()];
        let offset = self.pop().as_usize();
        let value = self.memory.mload_general(context, segment, offset);
        assert!(value.bits() <= segment.bit_range());
        self.push(value);
    }

    fn run_mstore_general(&mut self) {
        let context = self.pop().as_usize();
        let segment = Segment::all()[self.pop().as_usize()];
        let offset = self.pop().as_usize();
        let value = self.pop();
        assert!(
            value.bits() <= segment.bit_range(),
            "Value {} exceeds {:?} range of {} bits",
            value,
            segment,
            segment.bit_range()
        );
        self.memory.mstore_general(context, segment, offset, value);
    }
}

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
        0x5c => "GET_STATE_ROOT",
        0x5d => "SET_STATE_ROOT",
        0x5e => "GET_RECEIPT_ROOT",
        0x5f => "SET_RECEIPT_ROOT",
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
        0xf0 => "CREATE",
        0xf1 => "CALL",
        0xf2 => "CALLCODE",
        0xf3 => "RETURN",
        0xf4 => "DELEGATECALL",
        0xf5 => "CREATE2",
        0xf6 => "GET_CONTEXT",
        0xf7 => "SET_CONTEXT",
        0xf8 => "CONSUME_GAS",
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::cpu::kernel::interpreter::run;
    use crate::memory::segments::Segment;

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
        let code = vec![
            0x60, 0xff, 0x60, 0x0, 0x52, 0x60, 0, 0x51, 0x60, 0x1, 0x51, 0x60, 0x42, 0x60, 0x27,
            0x53,
        ];
        let pis = HashMap::new();
        let run = run(&code, 0, vec![], &pis)?;
        assert_eq!(run.stack(), &[0xff.into(), 0xff00.into()]);
        assert_eq!(
            run.memory.context_memory[0].segments[Segment::MainMemory as usize].get(0x27),
            0x42.into()
        );
        assert_eq!(
            run.memory.context_memory[0].segments[Segment::MainMemory as usize].get(0x1f),
            0xff.into()
        );
        Ok(())
    }
}
