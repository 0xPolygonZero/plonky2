// TODO: remove when possible.
#![allow(dead_code)]

use std::ops::Range;

/// Filter. 1 if the row is part of bootstrapping the kernel code, 0 otherwise.
pub const IS_BOOTSTRAP_KERNEL: usize = 0;

/// Filter. 1 if the row is part of bootstrapping a contract's code, 0 otherwise.
pub const IS_BOOTSTRAP_CONTRACT: usize = IS_BOOTSTRAP_KERNEL + 1;

/// Filter. 1 if the row corresponds to a cycle of execution and 0 otherwise.
/// Lets us re-use decode columns in non-cycle rows.
pub const IS_CPU_CYCLE: usize = IS_BOOTSTRAP_CONTRACT + 1;

/// If CPU cycle: The opcode being decoded, in {0, ..., 255}.
pub const OPCODE: usize = IS_CPU_CYCLE + 1;

// If CPU cycle: flags for EVM instructions. PUSHn, DUPn, and SWAPn only get one flag each. Invalid
// opcodes are split between a number of flags for practical reasons. Exactly one of these flags
// must be 1.
pub const IS_STOP: usize = OPCODE + 1;
pub const IS_ADD: usize = IS_STOP + 1;
pub const IS_MUL: usize = IS_ADD + 1;
pub const IS_SUB: usize = IS_MUL + 1;
pub const IS_DIV: usize = IS_SUB + 1;
pub const IS_SDIV: usize = IS_DIV + 1;
pub const IS_MOD: usize = IS_SDIV + 1;
pub const IS_SMOD: usize = IS_MOD + 1;
pub const IS_ADDMOD: usize = IS_SMOD + 1;
pub const IS_MULMOD: usize = IS_ADDMOD + 1;
pub const IS_EXP: usize = IS_MULMOD + 1;
pub const IS_SIGNEXTEND: usize = IS_EXP + 1;
pub const IS_LT: usize = IS_SIGNEXTEND + 1;
pub const IS_GT: usize = IS_LT + 1;
pub const IS_SLT: usize = IS_GT + 1;
pub const IS_SGT: usize = IS_SLT + 1;
pub const IS_EQ: usize = IS_SGT + 1; // Note: This column must be 0 when is_cpu_cycle = 0.
pub const IS_ISZERO: usize = IS_EQ + 1; // Note: This column must be 0 when is_cpu_cycle = 0.
pub const IS_AND: usize = IS_ISZERO + 1;
pub const IS_OR: usize = IS_AND + 1;
pub const IS_XOR: usize = IS_OR + 1;
pub const IS_NOT: usize = IS_XOR + 1;
pub const IS_BYTE: usize = IS_NOT + 1;
pub const IS_SHL: usize = IS_BYTE + 1;
pub const IS_SHR: usize = IS_SHL + 1;
pub const IS_SAR: usize = IS_SHR + 1;
pub const IS_SHA3: usize = IS_SAR + 1;
pub const IS_ADDRESS: usize = IS_SHA3 + 1;
pub const IS_BALANCE: usize = IS_ADDRESS + 1;
pub const IS_ORIGIN: usize = IS_BALANCE + 1;
pub const IS_CALLER: usize = IS_ORIGIN + 1;
pub const IS_CALLVALUE: usize = IS_CALLER + 1;
pub const IS_CALLDATALOAD: usize = IS_CALLVALUE + 1;
pub const IS_CALLDATASIZE: usize = IS_CALLDATALOAD + 1;
pub const IS_CALLDATACOPY: usize = IS_CALLDATASIZE + 1;
pub const IS_CODESIZE: usize = IS_CALLDATACOPY + 1;
pub const IS_CODECOPY: usize = IS_CODESIZE + 1;
pub const IS_GASPRICE: usize = IS_CODECOPY + 1;
pub const IS_EXTCODESIZE: usize = IS_GASPRICE + 1;
pub const IS_EXTCODECOPY: usize = IS_EXTCODESIZE + 1;
pub const IS_RETURNDATASIZE: usize = IS_EXTCODECOPY + 1;
pub const IS_RETURNDATACOPY: usize = IS_RETURNDATASIZE + 1;
pub const IS_EXTCODEHASH: usize = IS_RETURNDATACOPY + 1;
pub const IS_BLOCKHASH: usize = IS_EXTCODEHASH + 1;
pub const IS_COINBASE: usize = IS_BLOCKHASH + 1;
pub const IS_TIMESTAMP: usize = IS_COINBASE + 1;
pub const IS_NUMBER: usize = IS_TIMESTAMP + 1;
pub const IS_DIFFICULTY: usize = IS_NUMBER + 1;
pub const IS_GASLIMIT: usize = IS_DIFFICULTY + 1;
pub const IS_CHAINID: usize = IS_GASLIMIT + 1;
pub const IS_SELFBALANCE: usize = IS_CHAINID + 1;
pub const IS_BASEFEE: usize = IS_SELFBALANCE + 1;
pub const IS_POP: usize = IS_BASEFEE + 1;
pub const IS_MLOAD: usize = IS_POP + 1;
pub const IS_MSTORE: usize = IS_MLOAD + 1;
pub const IS_MSTORE8: usize = IS_MSTORE + 1;
pub const IS_SLOAD: usize = IS_MSTORE8 + 1;
pub const IS_SSTORE: usize = IS_SLOAD + 1;
pub const IS_JUMP: usize = IS_SSTORE + 1;
pub const IS_JUMPI: usize = IS_JUMP + 1;
pub const IS_PC: usize = IS_JUMPI + 1;
pub const IS_MSIZE: usize = IS_PC + 1;
pub const IS_GAS: usize = IS_MSIZE + 1;
pub const IS_JUMPDEST: usize = IS_GAS + 1;
// Find the number of bytes to push by reading the bottom 5 bits of the opcode.
pub const IS_PUSH: usize = IS_JUMPDEST + 1;
// Find the stack offset to duplicate by reading the bottom 4 bits of the opcode.
pub const IS_DUP: usize = IS_PUSH + 1;
// Find the stack offset to swap with by reading the bottom 4 bits of the opcode.
pub const IS_SWAP: usize = IS_DUP + 1;
pub const IS_LOG0: usize = IS_SWAP + 1;
pub const IS_LOG1: usize = IS_LOG0 + 1;
pub const IS_LOG2: usize = IS_LOG1 + 1;
pub const IS_LOG3: usize = IS_LOG2 + 1;
pub const IS_LOG4: usize = IS_LOG3 + 1;
pub const IS_CREATE: usize = IS_LOG4 + 1;
pub const IS_CALL: usize = IS_CREATE + 1;
pub const IS_CALLCODE: usize = IS_CALL + 1;
pub const IS_RETURN: usize = IS_CALLCODE + 1;
pub const IS_DELEGATECALL: usize = IS_RETURN + 1;
pub const IS_CREATE2: usize = IS_DELEGATECALL + 1;
pub const IS_STATICCALL: usize = IS_CREATE2 + 1;
pub const IS_REVERT: usize = IS_STATICCALL + 1;
pub const IS_SELFDESTRUCT: usize = IS_REVERT + 1;

pub const IS_INVALID_0: usize = IS_SELFDESTRUCT + 1;
pub const IS_INVALID_1: usize = IS_INVALID_0 + 1;
pub const IS_INVALID_2: usize = IS_INVALID_1 + 1;
pub const IS_INVALID_3: usize = IS_INVALID_2 + 1;
pub const IS_INVALID_4: usize = IS_INVALID_3 + 1;
pub const IS_INVALID_5: usize = IS_INVALID_4 + 1;
pub const IS_INVALID_6: usize = IS_INVALID_5 + 1;
pub const IS_INVALID_7: usize = IS_INVALID_6 + 1;
pub const IS_INVALID_8: usize = IS_INVALID_7 + 1;
pub const IS_INVALID_9: usize = IS_INVALID_8 + 1;
pub const IS_INVALID_10: usize = IS_INVALID_9 + 1;
pub const IS_INVALID_11: usize = IS_INVALID_10 + 1;
pub const IS_INVALID_12: usize = IS_INVALID_11 + 1;
pub const IS_INVALID_13: usize = IS_INVALID_12 + 1;
pub const IS_INVALID_14: usize = IS_INVALID_13 + 1;
pub const IS_INVALID_15: usize = IS_INVALID_14 + 1;
pub const IS_INVALID_16: usize = IS_INVALID_15 + 1;
pub const IS_INVALID_17: usize = IS_INVALID_16 + 1;
pub const IS_INVALID_18: usize = IS_INVALID_17 + 1;
pub const IS_INVALID_19: usize = IS_INVALID_18 + 1;
pub const IS_INVALID_20: usize = IS_INVALID_19 + 1;
// An instruction is invalid if _any_ of the above flags is 1.

pub const START_INSTRUCTION_FLAGS: usize = IS_STOP;
pub const END_INSTRUCTION_FLAGS: usize = IS_INVALID_20 + 1;

/// If CPU cycle: the opcode, broken up into bits.
/// **Big-endian** order.
pub const OPCODE_BITS: [usize; 8] = [
    END_INSTRUCTION_FLAGS,
    END_INSTRUCTION_FLAGS + 1,
    END_INSTRUCTION_FLAGS + 2,
    END_INSTRUCTION_FLAGS + 3,
    END_INSTRUCTION_FLAGS + 4,
    END_INSTRUCTION_FLAGS + 5,
    END_INSTRUCTION_FLAGS + 6,
    END_INSTRUCTION_FLAGS + 7,
];

/// Filter. 1 iff a Keccak permutation is computed on this row.
pub const IS_KECCAK: usize = OPCODE_BITS[OPCODE_BITS.len() - 1] + 1;

pub const START_KECCAK_INPUT: usize = IS_KECCAK + 1;
pub const KECCAK_INPUT_LIMBS: Range<usize> = START_KECCAK_INPUT..START_KECCAK_INPUT + 50;

pub const START_KECCAK_OUTPUT: usize = KECCAK_INPUT_LIMBS.end;
pub const KECCAK_OUTPUT_LIMBS: Range<usize> = START_KECCAK_OUTPUT..START_KECCAK_OUTPUT + 50;

// Assuming a limb size of 16 bits. This can be changed, but it must be <= 28 bits.
// TODO: These input/output columns can be shared between the logic operations and others.
pub const LOGIC_INPUT0: Range<usize> = KECCAK_OUTPUT_LIMBS.end..KECCAK_OUTPUT_LIMBS.end + 16;
pub const LOGIC_INPUT1: Range<usize> = LOGIC_INPUT0.end..LOGIC_INPUT0.end + 16;
pub const LOGIC_OUTPUT: Range<usize> = LOGIC_INPUT1.end..LOGIC_INPUT1.end + 16;

pub const SIMPLE_LOGIC_DIFF: usize = LOGIC_OUTPUT.end;
pub const SIMPLE_LOGIC_DIFF_INV: usize = SIMPLE_LOGIC_DIFF + 1;

pub(crate) const NUM_MEMORY_OPS: usize = 4;
pub(crate) const NUM_MEMORY_VALUE_LIMBS: usize = 8;

pub(crate) const CLOCK: usize = SIMPLE_LOGIC_DIFF_INV + 1;

// Uses_memop(i) is `F::ONE` iff this row includes a memory operation in its `i`th spot.
const USES_MEMOP_START: usize = CLOCK + 1;
pub const fn uses_memop(op: usize) -> usize {
    debug_assert!(op < NUM_MEMORY_OPS);
    USES_MEMOP_START + op
}

const MEMOP_ISREAD_START: usize = USES_MEMOP_START + NUM_MEMORY_OPS;
pub const fn memop_is_read(op: usize) -> usize {
    debug_assert!(op < NUM_MEMORY_OPS);
    MEMOP_ISREAD_START + op
}

const MEMOP_ADDR_CONTEXT_START: usize = MEMOP_ISREAD_START + NUM_MEMORY_OPS;
pub const fn memop_addr_context(op: usize) -> usize {
    debug_assert!(op < NUM_MEMORY_OPS);
    MEMOP_ADDR_CONTEXT_START + op
}

const MEMOP_ADDR_SEGMENT_START: usize = MEMOP_ADDR_CONTEXT_START + NUM_MEMORY_OPS;
pub const fn memop_addr_segment(op: usize) -> usize {
    debug_assert!(op < NUM_MEMORY_OPS);
    MEMOP_ADDR_SEGMENT_START + op
}

const MEMOP_ADDR_VIRTUAL_START: usize = MEMOP_ADDR_SEGMENT_START + NUM_MEMORY_OPS;
pub const fn memop_addr_virtual(op: usize) -> usize {
    debug_assert!(op < NUM_MEMORY_OPS);
    MEMOP_ADDR_VIRTUAL_START + op
}

const MEMOP_ADDR_VALUE_START: usize = MEMOP_ADDR_VIRTUAL_START + NUM_MEMORY_OPS;
pub const fn memop_value(op: usize, limb: usize) -> usize {
    debug_assert!(op < NUM_MEMORY_OPS);
    debug_assert!(limb < NUM_MEMORY_VALUE_LIMBS);
    MEMOP_ADDR_VALUE_START + op * NUM_MEMORY_VALUE_LIMBS + limb
}

pub const NUM_CPU_COLUMNS: usize = MEMOP_ADDR_VALUE_START + NUM_MEMORY_OPS * NUM_MEMORY_VALUE_LIMBS;
