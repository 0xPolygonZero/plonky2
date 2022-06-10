// Filter. 1 if the row corresponds to a cycle of execution and 0 otherwise.
// Lets us re-use decode columns in non-cycle rows.
pub const IS_CPU_CYCLE: usize = 0;

// If CPU cycle: The opcode being decoded, in {0, ..., 255}.
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
pub const IS_EQ: usize = IS_SGT + 1;
pub const IS_ISZERO: usize = IS_EQ + 1;
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

// If CPU cycle: the opcode, broken up into bits.
// **big-endian** order
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
#[allow(dead_code)] // TODO: Remove when used
pub const KECCAK_INPUT_LIMBS: [usize; 50] = [
    START_KECCAK_INPUT,
    START_KECCAK_INPUT + 1,
    START_KECCAK_INPUT + 2,
    START_KECCAK_INPUT + 3,
    START_KECCAK_INPUT + 4,
    START_KECCAK_INPUT + 5,
    START_KECCAK_INPUT + 6,
    START_KECCAK_INPUT + 7,
    START_KECCAK_INPUT + 8,
    START_KECCAK_INPUT + 9,
    START_KECCAK_INPUT + 10,
    START_KECCAK_INPUT + 11,
    START_KECCAK_INPUT + 12,
    START_KECCAK_INPUT + 13,
    START_KECCAK_INPUT + 14,
    START_KECCAK_INPUT + 15,
    START_KECCAK_INPUT + 16,
    START_KECCAK_INPUT + 17,
    START_KECCAK_INPUT + 18,
    START_KECCAK_INPUT + 19,
    START_KECCAK_INPUT + 20,
    START_KECCAK_INPUT + 21,
    START_KECCAK_INPUT + 22,
    START_KECCAK_INPUT + 23,
    START_KECCAK_INPUT + 24,
    START_KECCAK_INPUT + 25,
    START_KECCAK_INPUT + 26,
    START_KECCAK_INPUT + 27,
    START_KECCAK_INPUT + 28,
    START_KECCAK_INPUT + 29,
    START_KECCAK_INPUT + 30,
    START_KECCAK_INPUT + 31,
    START_KECCAK_INPUT + 32,
    START_KECCAK_INPUT + 33,
    START_KECCAK_INPUT + 34,
    START_KECCAK_INPUT + 35,
    START_KECCAK_INPUT + 36,
    START_KECCAK_INPUT + 37,
    START_KECCAK_INPUT + 38,
    START_KECCAK_INPUT + 39,
    START_KECCAK_INPUT + 40,
    START_KECCAK_INPUT + 41,
    START_KECCAK_INPUT + 42,
    START_KECCAK_INPUT + 43,
    START_KECCAK_INPUT + 44,
    START_KECCAK_INPUT + 45,
    START_KECCAK_INPUT + 46,
    START_KECCAK_INPUT + 47,
    START_KECCAK_INPUT + 48,
    START_KECCAK_INPUT + 49,
];

pub const START_KECCAK_OUTPUT: usize = START_KECCAK_INPUT + 50;
pub const KECCAK_OUTPUT_LIMBS: [usize; 50] = [
    START_KECCAK_OUTPUT,
    START_KECCAK_OUTPUT + 1,
    START_KECCAK_OUTPUT + 2,
    START_KECCAK_OUTPUT + 3,
    START_KECCAK_OUTPUT + 4,
    START_KECCAK_OUTPUT + 5,
    START_KECCAK_OUTPUT + 6,
    START_KECCAK_OUTPUT + 7,
    START_KECCAK_OUTPUT + 8,
    START_KECCAK_OUTPUT + 9,
    START_KECCAK_OUTPUT + 10,
    START_KECCAK_OUTPUT + 11,
    START_KECCAK_OUTPUT + 12,
    START_KECCAK_OUTPUT + 13,
    START_KECCAK_OUTPUT + 14,
    START_KECCAK_OUTPUT + 15,
    START_KECCAK_OUTPUT + 16,
    START_KECCAK_OUTPUT + 17,
    START_KECCAK_OUTPUT + 18,
    START_KECCAK_OUTPUT + 19,
    START_KECCAK_OUTPUT + 20,
    START_KECCAK_OUTPUT + 21,
    START_KECCAK_OUTPUT + 22,
    START_KECCAK_OUTPUT + 23,
    START_KECCAK_OUTPUT + 24,
    START_KECCAK_OUTPUT + 25,
    START_KECCAK_OUTPUT + 26,
    START_KECCAK_OUTPUT + 27,
    START_KECCAK_OUTPUT + 28,
    START_KECCAK_OUTPUT + 29,
    START_KECCAK_OUTPUT + 30,
    START_KECCAK_OUTPUT + 31,
    START_KECCAK_OUTPUT + 32,
    START_KECCAK_OUTPUT + 33,
    START_KECCAK_OUTPUT + 34,
    START_KECCAK_OUTPUT + 35,
    START_KECCAK_OUTPUT + 36,
    START_KECCAK_OUTPUT + 37,
    START_KECCAK_OUTPUT + 38,
    START_KECCAK_OUTPUT + 39,
    START_KECCAK_OUTPUT + 40,
    START_KECCAK_OUTPUT + 41,
    START_KECCAK_OUTPUT + 42,
    START_KECCAK_OUTPUT + 43,
    START_KECCAK_OUTPUT + 44,
    START_KECCAK_OUTPUT + 45,
    START_KECCAK_OUTPUT + 46,
    START_KECCAK_OUTPUT + 47,
    START_KECCAK_OUTPUT + 48,
    START_KECCAK_OUTPUT + 49,
];

pub const NUM_CPU_COLUMNS: usize = KECCAK_OUTPUT_LIMBS[KECCAK_OUTPUT_LIMBS.len() - 1] + 1;
