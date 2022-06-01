use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns;

const EASY_OPCODES: [(usize, u64); 78] = [
    (columns::IS_STOP, 0x00),
    (columns::IS_ADD, 0x01),
    (columns::IS_MUL, 0x02),
    (columns::IS_SUB, 0x03),
    (columns::IS_DIV, 0x04),
    (columns::IS_SDIV, 0x05),
    (columns::IS_MOD, 0x06),
    (columns::IS_SMOD, 0x07),
    (columns::IS_ADDMOD, 0x08),
    (columns::IS_MULMOD, 0x09),
    (columns::IS_EXP, 0x0a),
    (columns::IS_SIGNEXTEND, 0x0b),
    (columns::IS_LT, 0x10),
    (columns::IS_GT, 0x11),
    (columns::IS_SLT, 0x12),
    (columns::IS_SGT, 0x13),
    (columns::IS_EQ, 0x14),
    (columns::IS_ISZERO, 0x15),
    (columns::IS_AND, 0x16),
    (columns::IS_OR, 0x17),
    (columns::IS_XOR, 0x18),
    (columns::IS_NOT, 0x19),
    (columns::IS_BYTE, 0x1a),
    (columns::IS_SHL, 0x1b),
    (columns::IS_SHR, 0x1c),
    (columns::IS_SAR, 0x1d),
    (columns::IS_SHA3, 0x20),
    (columns::IS_ADDRESS, 0x30),
    (columns::IS_BALANCE, 0x31),
    (columns::IS_ORIGIN, 0x32),
    (columns::IS_CALLER, 0x33),
    (columns::IS_CALLVALUE, 0x34),
    (columns::IS_CALLDATALOAD, 0x35),
    (columns::IS_CALLDATASIZE, 0x36),
    (columns::IS_CALLDATACOPY, 0x37),
    (columns::IS_CODESIZE, 0x38),
    (columns::IS_CODECOPY, 0x39),
    (columns::IS_GASPRICE, 0x3a),
    (columns::IS_EXTCODESIZE, 0x3b),
    (columns::IS_EXTCODECOPY, 0x3c),
    (columns::IS_RETURNDATASIZE, 0x3d),
    (columns::IS_RETURNDATACOPY, 0x3e),
    (columns::IS_EXTCODEHASH, 0x3f),
    (columns::IS_BLOCKHASH, 0x40),
    (columns::IS_COINBASE, 0x41),
    (columns::IS_TIMESTAMP, 0x42),
    (columns::IS_NUMBER, 0x43),
    (columns::IS_DIFFICULTY, 0x44),
    (columns::IS_GASLIMIT, 0x45),
    (columns::IS_CHAINID, 0x46),
    (columns::IS_SELFBALANCE, 0x47),
    (columns::IS_BASEFEE, 0x48),
    (columns::IS_POP, 0x50),
    (columns::IS_MLOAD, 0x51),
    (columns::IS_MSTORE, 0x52),
    (columns::IS_MSTORE8, 0x53),
    (columns::IS_SLOAD, 0x54),
    (columns::IS_SSTORE, 0x55),
    (columns::IS_JUMP, 0x56),
    (columns::IS_JUMPI, 0x57),
    (columns::IS_PC, 0x58),
    (columns::IS_MSIZE, 0x59),
    (columns::IS_GAS, 0x5a),
    (columns::IS_JUMPDEST, 0x5b),
    (columns::IS_LOG0, 0xa0),
    (columns::IS_LOG1, 0xa1),
    (columns::IS_LOG2, 0xa2),
    (columns::IS_LOG3, 0xa3),
    (columns::IS_LOG4, 0xa4),
    (columns::IS_CREATE, 0xf0),
    (columns::IS_CALL, 0xf1),
    (columns::IS_CALLCODE, 0xf2),
    (columns::IS_RETURN, 0xf3),
    (columns::IS_DELEGATECALL, 0xf4),
    (columns::IS_CREATE2, 0xf5),
    (columns::IS_STATICCALL, 0xfa),
    (columns::IS_REVERT, 0xfd),
    (columns::IS_SELFDESTRUCT, 0xff),
];

const OPCODE_BLOCKS: [(usize, usize, u64); 24] = [
    (columns::IS_PUSH, 3, 0x60),
    (columns::IS_DUP, 4, 0x80),
    (columns::IS_SWAP, 4, 0x90),
    (columns::IS_INVALID_0, 6, 0x0c),  // 0x0c-0x0f, 000011xx
    (columns::IS_INVALID_1, 7, 0x1e),  // 0x1e-0x1f, 0001111x
    (columns::IS_INVALID_2, 8, 0x21),  // 0x21, 00100001
    (columns::IS_INVALID_3, 7, 0x22),  // 0x22-0x23, 0010001x
    (columns::IS_INVALID_4, 6, 0x24),  // 0x24-0x27, 001001xx
    (columns::IS_INVALID_5, 5, 0x28),  // 0x28-0x2f, 00101xxx
    (columns::IS_INVALID_6, 8, 0x49),  // 0x49, 01001001
    (columns::IS_INVALID_7, 7, 0x4a),  // 0x4a-0x4b, 0100101x
    (columns::IS_INVALID_8, 6, 0x4c),  // 0x4c-0x4f, 010011xx
    (columns::IS_INVALID_9, 6, 0x5c),  // 0x5c-0x5f, 010111xx
    (columns::IS_INVALID_10, 8, 0xa5), // 0xa5, 10100101
    (columns::IS_INVALID_11, 7, 0xa6), // 0xa6-0xa7, 1010011x
    (columns::IS_INVALID_12, 5, 0xa8), // 0xa8-0xaf, 10101xxx
    (columns::IS_INVALID_13, 4, 0xb0), // 0xb0-0xbf, 1011xxxx
    (columns::IS_INVALID_14, 3, 0xc0), // 0xc0-0xdf, 110xxxxx
    (columns::IS_INVALID_15, 4, 0xe0), // 0xe0-0xef, 1110xxxx
    (columns::IS_INVALID_16, 7, 0xf6), // 0xf6-0xf7, 1111011x
    (columns::IS_INVALID_17, 7, 0xf8), // 0xf8-0xf9, 1111100x
    (columns::IS_INVALID_18, 8, 0xfb), // 0xfb, 11111011
    (columns::IS_INVALID_19, 8, 0xfc), // 0xfc, 11111100
    (columns::IS_INVALID_20, 8, 0xfe), // 0xfe, 11111110
];

#[allow(dead_code)]
pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_CPU_COLUMNS]) {
    let cycle_filter = lv[columns::IS_CPU_CYCLE];
    if cycle_filter == F::ZERO {
        return;
    }
    // This assert is not _strictly_ necessary, but I include it as a sanity check.
    assert_eq!(cycle_filter, F::ONE, "cycle_filter should be 0 or 1");

    let opcode = lv[columns::OPCODE].to_canonical_u64();
    assert!(opcode < 256, "opcode should be in {{0, ..., 255}}");
    let bits = [
        (opcode >> 7) & 1,
        (opcode >> 6) & 1,
        (opcode >> 5) & 1,
        (opcode >> 4) & 1,
        (opcode >> 3) & 1,
        (opcode >> 2) & 1,
        (opcode >> 1) & 1,
        opcode & 1,
    ];

    for (&b, col) in bits.iter().zip(columns::OPCODE_BITS) {
        lv[col] = F::from_canonical_u64(b);
    }

    let top_bits = {
        let mut top_bits = [0u64; 9];
        for i in 0..8 {
            top_bits[i + 1] = top_bits[i] + (bits[i] << (7 - i));
        }
        top_bits
    };

    for (col, oc) in EASY_OPCODES {
        lv[col] = F::from_bool(oc == opcode);
    }

    for (col, bits_to_check, oc) in OPCODE_BLOCKS {
        lv[col] = F::from_bool(top_bits[bits_to_check] == oc);
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

    let top_bits = {
        let mut top_bits = [P::ZEROS; 9];
        for i in 0..8 {
            top_bits[i + 1] = top_bits[i] + bits[i] * P::Scalar::from_canonical_u64(1 << (7 - i));
        }
        top_bits
    };

    // Now check that they match the opcode.
    let opcode = lv[columns::OPCODE];
    let opcode_from_bits = top_bits[8];
    yield_constr.constraint(cycle_filter * (opcode - opcode_from_bits));

    // Check that the instruction flags are valid.
    // First, check that they are all either 0 or 1.
    for &flag in lv
        .iter()
        .take(columns::END_INSTRUCTION_FLAGS)
        .skip(columns::START_INSTRUCTION_FLAGS)
    {
        yield_constr.constraint(cycle_filter * flag * (flag - P::ONES));
    }
    // Now check that exactly one is 1.
    let flag_sum: P = (columns::START_INSTRUCTION_FLAGS..columns::END_INSTRUCTION_FLAGS)
        .into_iter()
        .map(|i| lv[i])
        .sum();
    yield_constr.constraint(cycle_filter * (flag_sum - P::ONES));

    // Deal with all the "easy opcodes" first: ones where the opcode is valid and is the only one
    // that corresponds to a particular instruction.
    for (col, oc) in EASY_OPCODES {
        let constr = lv[col] * (opcode - P::Scalar::from_canonical_u64(oc));
        yield_constr.constraint(cycle_filter * constr);
    }

    for (col, bits_to_check, oc) in OPCODE_BLOCKS {
        let constr = lv[col] * (top_bits[bits_to_check] - P::Scalar::from_canonical_u64(oc));
        yield_constr.constraint(cycle_filter * constr);
    }

    // To check if the opcode is invalid, sum all the IS_INVALID_N columns.
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

    // Now check that they match the opcode.
    let opcode = lv[columns::OPCODE];
    {
        let opcode_from_bits = top_bits[8];
        let constr = builder.sub_extension(opcode, opcode_from_bits);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    };

    // Check that the instruction flags are valid.
    // First, check that they are all either 0 or 1.
    for &flag in lv
        .iter()
        .take(columns::END_INSTRUCTION_FLAGS)
        .skip(columns::START_INSTRUCTION_FLAGS)
    {
        let constr = builder.mul_sub_extension(flag, flag, flag);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }
    {
        let mut constr = builder.one_extension();
        for &flag in lv
            .iter()
            .take(columns::END_INSTRUCTION_FLAGS)
            .skip(columns::START_INSTRUCTION_FLAGS)
        {
            constr = builder.sub_extension(constr, flag);
        }
        constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // Deal with all the "easy opcodes" first: ones where the opcode is valid and is the only one
    // that corresponds to a particular instruction.
    for (col, oc) in EASY_OPCODES {
        let flag = lv[col];
        let constr =
            builder.arithmetic_extension(F::ONE, -F::from_canonical_u64(oc), flag, opcode, flag);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    for (col, bits_to_check, oc) in OPCODE_BLOCKS {
        let flag = lv[col];
        let constr = builder.arithmetic_extension(
            F::ONE,
            -F::from_canonical_u64(oc),
            flag,
            top_bits[bits_to_check],
            flag,
        );
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }
}
