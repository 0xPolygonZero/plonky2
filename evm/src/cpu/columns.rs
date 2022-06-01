pub const OPCODE: usize = 0;

pub const IS_CPU_CYCLE: usize = OPCODE + 1;

pub const IS_STOP: usize = IS_CPU_CYCLE + 1;
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
pub const IS_PUSH: usize = IS_JUMPDEST + 1;
pub const IS_DUP: usize = IS_PUSH + 1;
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

pub const START_INSTRUCTION_FLAGS: usize = IS_STOP;
pub const END_INSTRUCTION_FLAGS: usize = IS_INVALID_20 + 1;

pub const OPCODE_BITS: [usize; 8] = [
    END_INSTRUCTION_FLAGS + 1,
    END_INSTRUCTION_FLAGS + 2,
    END_INSTRUCTION_FLAGS + 3,
    END_INSTRUCTION_FLAGS + 4,
    END_INSTRUCTION_FLAGS + 5,
    END_INSTRUCTION_FLAGS + 6,
    END_INSTRUCTION_FLAGS + 7,
    END_INSTRUCTION_FLAGS + 8,
];

pub const NUM_CPU_COLUMNS: usize = OPCODE_BITS[OPCODE_BITS.len() - 1] + 1;
