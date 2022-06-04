use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns;

// List of opcode blocks
// Each block corresponds to exactly one flag, and each flag corresponds to exactly one block.
// Each block of opcodes:
// - is contiguous
// - has a length that is a power of 2
// - its start index is a multiple of its length (it is aligned)
// These properties permit us to check if an opcode belongs to a block of length 2^n by checking its
// top 8-n bits.
const OPCODES: [(u64, usize, usize); 102] = [
    // (start index of block, number of top bits to check (log2), flag column)
    (0x00, 0, columns::IS_STOP),
    (0x01, 0, columns::IS_ADD),
    (0x02, 0, columns::IS_MUL),
    (0x03, 0, columns::IS_SUB),
    (0x04, 0, columns::IS_DIV),
    (0x05, 0, columns::IS_SDIV),
    (0x06, 0, columns::IS_MOD),
    (0x07, 0, columns::IS_SMOD),
    (0x08, 0, columns::IS_ADDMOD),
    (0x09, 0, columns::IS_MULMOD),
    (0x0a, 0, columns::IS_EXP),
    (0x0b, 0, columns::IS_SIGNEXTEND),
    (0x0c, 2, columns::IS_INVALID_0), // 0x0c-0x0f
    (0x10, 0, columns::IS_LT),
    (0x11, 0, columns::IS_GT),
    (0x12, 0, columns::IS_SLT),
    (0x13, 0, columns::IS_SGT),
    (0x14, 0, columns::IS_EQ),
    (0x15, 0, columns::IS_ISZERO),
    (0x16, 0, columns::IS_AND),
    (0x17, 0, columns::IS_OR),
    (0x18, 0, columns::IS_XOR),
    (0x19, 0, columns::IS_NOT),
    (0x1a, 0, columns::IS_BYTE),
    (0x1b, 0, columns::IS_SHL),
    (0x1c, 0, columns::IS_SHR),
    (0x1d, 0, columns::IS_SAR),
    (0x1e, 1, columns::IS_INVALID_1), // 0x1e-0x1f
    (0x20, 0, columns::IS_SHA3),
    (0x21, 0, columns::IS_INVALID_2),
    (0x22, 1, columns::IS_INVALID_3), // 0x22-0x23
    (0x24, 2, columns::IS_INVALID_4), // 0x24-0x27
    (0x28, 3, columns::IS_INVALID_5), // 0x28-0x2f
    (0x30, 0, columns::IS_ADDRESS),
    (0x31, 0, columns::IS_BALANCE),
    (0x32, 0, columns::IS_ORIGIN),
    (0x33, 0, columns::IS_CALLER),
    (0x34, 0, columns::IS_CALLVALUE),
    (0x35, 0, columns::IS_CALLDATALOAD),
    (0x36, 0, columns::IS_CALLDATASIZE),
    (0x37, 0, columns::IS_CALLDATACOPY),
    (0x38, 0, columns::IS_CODESIZE),
    (0x39, 0, columns::IS_CODECOPY),
    (0x3a, 0, columns::IS_GASPRICE),
    (0x3b, 0, columns::IS_EXTCODESIZE),
    (0x3c, 0, columns::IS_EXTCODECOPY),
    (0x3d, 0, columns::IS_RETURNDATASIZE),
    (0x3e, 0, columns::IS_RETURNDATACOPY),
    (0x3f, 0, columns::IS_EXTCODEHASH),
    (0x40, 0, columns::IS_BLOCKHASH),
    (0x41, 0, columns::IS_COINBASE),
    (0x42, 0, columns::IS_TIMESTAMP),
    (0x43, 0, columns::IS_NUMBER),
    (0x44, 0, columns::IS_DIFFICULTY),
    (0x45, 0, columns::IS_GASLIMIT),
    (0x46, 0, columns::IS_CHAINID),
    (0x47, 0, columns::IS_SELFBALANCE),
    (0x48, 0, columns::IS_BASEFEE),
    (0x49, 0, columns::IS_INVALID_6),
    (0x4a, 1, columns::IS_INVALID_7), // 0x4a-0x4b
    (0x4c, 2, columns::IS_INVALID_8), // 0x4c-0x4f
    (0x50, 0, columns::IS_POP),
    (0x51, 0, columns::IS_MLOAD),
    (0x52, 0, columns::IS_MSTORE),
    (0x53, 0, columns::IS_MSTORE8),
    (0x54, 0, columns::IS_SLOAD),
    (0x55, 0, columns::IS_SSTORE),
    (0x56, 0, columns::IS_JUMP),
    (0x57, 0, columns::IS_JUMPI),
    (0x58, 0, columns::IS_PC),
    (0x59, 0, columns::IS_MSIZE),
    (0x5a, 0, columns::IS_GAS),
    (0x5b, 0, columns::IS_JUMPDEST),
    (0x5c, 2, columns::IS_INVALID_9), // 0x5c-0x5f
    (0x60, 5, columns::IS_PUSH),      // 0x60-0x7f
    (0x80, 4, columns::IS_DUP),       // 0x80-0x8f
    (0x90, 4, columns::IS_SWAP),      // 0x90-0x9f
    (0xa0, 0, columns::IS_LOG0),
    (0xa1, 0, columns::IS_LOG1),
    (0xa2, 0, columns::IS_LOG2),
    (0xa3, 0, columns::IS_LOG3),
    (0xa4, 0, columns::IS_LOG4),
    (0xa5, 0, columns::IS_INVALID_10),
    (0xa6, 1, columns::IS_INVALID_11), // 0xa6-0xa7
    (0xa8, 3, columns::IS_INVALID_12), // 0xa8-0xaf
    (0xb0, 4, columns::IS_INVALID_13), // 0xb0-0xbf
    (0xc0, 5, columns::IS_INVALID_14), // 0xc0-0xdf
    (0xe0, 4, columns::IS_INVALID_15), // 0xe0-0xef
    (0xf0, 0, columns::IS_CREATE),
    (0xf1, 0, columns::IS_CALL),
    (0xf2, 0, columns::IS_CALLCODE),
    (0xf3, 0, columns::IS_RETURN),
    (0xf4, 0, columns::IS_DELEGATECALL),
    (0xf5, 0, columns::IS_CREATE2),
    (0xf6, 1, columns::IS_INVALID_16), // 0xf6-0xf7
    (0xf8, 1, columns::IS_INVALID_17), // 0xf8-0xf9
    (0xfa, 0, columns::IS_STATICCALL),
    (0xfb, 0, columns::IS_INVALID_18),
    (0xfc, 0, columns::IS_INVALID_19),
    (0xfd, 0, columns::IS_REVERT),
    (0xfe, 0, columns::IS_INVALID_20),
    (0xff, 0, columns::IS_SELFDESTRUCT),
];

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_CPU_COLUMNS]) {
    let cycle_filter = lv[columns::IS_CPU_CYCLE];
    if cycle_filter == F::ZERO {
        return;
    }
    // This assert is not _strictly_ necessary, but I include it as a sanity check.
    assert_eq!(cycle_filter, F::ONE, "cycle_filter should be 0 or 1");

    let opcode = lv[columns::OPCODE].to_canonical_u64();
    assert!(opcode < 256, "opcode should be in {{0, ..., 255}}");

    for (i, &col) in columns::OPCODE_BITS.iter().enumerate() {
        let bit = (opcode >> (7 - i)) & 1;
        lv[col] = F::from_canonical_u64(bit);
    }

    let top_bits: [u64; 9] = [
        0,
        opcode & 0x80,
        opcode & 0xc0,
        opcode & 0xe0,
        opcode & 0xf0,
        opcode & 0xf8,
        opcode & 0xfc,
        opcode & 0xfe,
        opcode,
    ];

    for (oc, block_length, col) in OPCODES {
        lv[col] = F::from_bool(top_bits[8 - block_length] == oc);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_CPU_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let cycle_filter = lv[columns::IS_CPU_CYCLE];

    // Ensure that the opcode bits are valid: each has to be either 0 or 1, and they must match
    // the opcode. Note that this also validates that this implicitly range-checks the opcode.
    let bits = columns::OPCODE_BITS.map(|i| lv[i]);
    // First check that the bits are either 0 or 1.
    for bit in bits {
        yield_constr.constraint(cycle_filter * bit * (bit - P::ONES));
    }

    // top_bits[i] is the opcode with all but the top i bits cleared.
    let top_bits = {
        let mut top_bits = [P::ZEROS; 9];
        for i in 0..8 {
            top_bits[i + 1] = top_bits[i] + bits[i] * P::Scalar::from_canonical_u64(1 << (7 - i));
        }
        top_bits
    };

    // Now check that they match the opcode.
    let opcode = lv[columns::OPCODE];
    yield_constr.constraint(cycle_filter * (opcode - top_bits[8]));

    // Check that the instruction flags are valid.
    // First, check that they are all either 0 or 1.
    for &flag in &lv[columns::START_INSTRUCTION_FLAGS..columns::END_INSTRUCTION_FLAGS] {
        yield_constr.constraint(cycle_filter * flag * (flag - P::ONES));
    }
    // Now check that exactly one is 1.
    let flag_sum: P = (columns::START_INSTRUCTION_FLAGS..columns::END_INSTRUCTION_FLAGS)
        .into_iter()
        .map(|i| lv[i])
        .sum();
    yield_constr.constraint(cycle_filter * (P::ONES - flag_sum));

    // Finally, classify all opcodes into blocks
    for (oc, block_length, col) in OPCODES {
        let constr = lv[col] * (top_bits[8 - block_length] - P::Scalar::from_canonical_u64(oc));
        yield_constr.constraint(cycle_filter * constr);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_CPU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let cycle_filter = lv[columns::IS_CPU_CYCLE];

    // Ensure that the opcode bits are valid: each has to be either 0 or 1, and they must match
    // the opcode. Note that this also validates that this implicitly range-checks the opcode.
    let bits = columns::OPCODE_BITS.map(|i| lv[i]);
    // First check that the bits are either 0 or 1.
    for bit in bits {
        let constr = builder.mul_sub_extension(bit, bit, bit);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    let top_bits = {
        let mut top_bits = [builder.zero_extension(); 9];
        for i in 0..8 {
            top_bits[i + 1] = builder.mul_const_add_extension(
                F::from_canonical_u64(1 << (7 - i)),
                bits[i],
                top_bits[i],
            );
        }
        top_bits
    };

    // Now check that the bits match the opcode.
    {
        let constr = builder.sub_extension(lv[columns::OPCODE], top_bits[8]);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    };

    // Check that the instruction flags are valid.
    // First, check that they are all either 0 or 1.
    for &flag in &lv[columns::START_INSTRUCTION_FLAGS..columns::END_INSTRUCTION_FLAGS] {
        let constr = builder.mul_sub_extension(flag, flag, flag);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }
    // Now check that they sum to 1.
    {
        let mut constr = builder.one_extension();
        for &flag in &lv[columns::START_INSTRUCTION_FLAGS..columns::END_INSTRUCTION_FLAGS] {
            constr = builder.sub_extension(constr, flag);
        }
        constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    for (oc, block_length, col) in OPCODES {
        let flag = lv[col];
        let constr = builder.constant_extension(F::from_canonical_u64(oc).into());
        let constr = builder.sub_extension(top_bits[8 - block_length], constr);
        let constr = builder.mul_extension(flag, constr);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }
}
