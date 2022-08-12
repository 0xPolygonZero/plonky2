use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP};

#[derive(PartialEq, Eq)]
enum Availability {
    All,
    User,
    Kernel,
}
use Availability::{All, Kernel, User};

/// List of opcode blocks
///  Each block corresponds to exactly one flag, and each flag corresponds to exactly one block.
///  Each block of opcodes:
/// - is contiguous,
/// - has a length that is a power of 2, and
/// - its start index is a multiple of its length (it is aligned).
///  These properties permit us to check if an opcode belongs to a block of length 2^n by checking
/// its top 8-n bits.
///  Additionally, each block can be made available only to the user, only to the kernel, or to
/// both. This is mainly useful for making some instructions kernel-only, while still decoding to
/// invalid for the user. We do this by making one kernel-only block and another user-only block.
/// The exception is the PANIC instruction which is user-only without a corresponding kernel block.
/// This makes the proof unverifiable when PANIC is executed in kernel mode, which is the intended
/// behavior.
const OPCODES: [(u8, usize, Availability, usize); 113] = [
    // (start index of block, number of top bits to check (log2), availability, flag column)
    (0x00, 0, All, COL_MAP.is_stop),
    (0x01, 0, All, COL_MAP.is_add),
    (0x02, 0, All, COL_MAP.is_mul),
    (0x03, 0, All, COL_MAP.is_sub),
    (0x04, 0, All, COL_MAP.is_div),
    (0x05, 0, All, COL_MAP.is_sdiv),
    (0x06, 0, All, COL_MAP.is_mod),
    (0x07, 0, All, COL_MAP.is_smod),
    (0x08, 0, All, COL_MAP.is_addmod),
    (0x09, 0, All, COL_MAP.is_mulmod),
    (0x0a, 0, All, COL_MAP.is_exp),
    (0x0b, 0, All, COL_MAP.is_signextend),
    (0x0c, 2, All, COL_MAP.is_invalid_0), // 0x0c-0x0f
    (0x10, 0, All, COL_MAP.is_lt),
    (0x11, 0, All, COL_MAP.is_gt),
    (0x12, 0, All, COL_MAP.is_slt),
    (0x13, 0, All, COL_MAP.is_sgt),
    (0x14, 0, All, COL_MAP.is_eq),
    (0x15, 0, All, COL_MAP.is_iszero),
    (0x16, 0, All, COL_MAP.is_and),
    (0x17, 0, All, COL_MAP.is_or),
    (0x18, 0, All, COL_MAP.is_xor),
    (0x19, 0, All, COL_MAP.is_not),
    (0x1a, 0, All, COL_MAP.is_byte),
    (0x1b, 0, All, COL_MAP.is_shl),
    (0x1c, 0, All, COL_MAP.is_shr),
    (0x1d, 0, All, COL_MAP.is_sar),
    (0x1e, 1, All, COL_MAP.is_invalid_1), // 0x1e-0x1f
    (0x20, 0, All, COL_MAP.is_keccak256),
    (0x21, 0, All, COL_MAP.is_invalid_2),
    (0x22, 1, All, COL_MAP.is_invalid_3), // 0x22-0x23
    (0x24, 2, All, COL_MAP.is_invalid_4), // 0x24-0x27
    (0x28, 3, All, COL_MAP.is_invalid_5), // 0x28-0x2f
    (0x30, 0, All, COL_MAP.is_address),
    (0x31, 0, All, COL_MAP.is_balance),
    (0x32, 0, All, COL_MAP.is_origin),
    (0x33, 0, All, COL_MAP.is_caller),
    (0x34, 0, All, COL_MAP.is_callvalue),
    (0x35, 0, All, COL_MAP.is_calldataload),
    (0x36, 0, All, COL_MAP.is_calldatasize),
    (0x37, 0, All, COL_MAP.is_calldatacopy),
    (0x38, 0, All, COL_MAP.is_codesize),
    (0x39, 0, All, COL_MAP.is_codecopy),
    (0x3a, 0, All, COL_MAP.is_gasprice),
    (0x3b, 0, All, COL_MAP.is_extcodesize),
    (0x3c, 0, All, COL_MAP.is_extcodecopy),
    (0x3d, 0, All, COL_MAP.is_returndatasize),
    (0x3e, 0, All, COL_MAP.is_returndatacopy),
    (0x3f, 0, All, COL_MAP.is_extcodehash),
    (0x40, 0, All, COL_MAP.is_blockhash),
    (0x41, 0, All, COL_MAP.is_coinbase),
    (0x42, 0, All, COL_MAP.is_timestamp),
    (0x43, 0, All, COL_MAP.is_number),
    (0x44, 0, All, COL_MAP.is_difficulty),
    (0x45, 0, All, COL_MAP.is_gaslimit),
    (0x46, 0, All, COL_MAP.is_chainid),
    (0x47, 0, All, COL_MAP.is_selfbalance),
    (0x48, 0, All, COL_MAP.is_basefee),
    (0x49, 0, User, COL_MAP.is_invalid_6),
    (0x49, 0, Kernel, COL_MAP.is_prover_input),
    (0x4a, 1, All, COL_MAP.is_invalid_7), // 0x4a-0x4b
    (0x4c, 2, All, COL_MAP.is_invalid_8), // 0x4c-0x4f
    (0x50, 0, All, COL_MAP.is_pop),
    (0x51, 0, All, COL_MAP.is_mload),
    (0x52, 0, All, COL_MAP.is_mstore),
    (0x53, 0, All, COL_MAP.is_mstore8),
    (0x54, 0, All, COL_MAP.is_sload),
    (0x55, 0, All, COL_MAP.is_sstore),
    (0x56, 0, All, COL_MAP.is_jump),
    (0x57, 0, All, COL_MAP.is_jumpi),
    (0x58, 0, All, COL_MAP.is_pc),
    (0x59, 0, All, COL_MAP.is_msize),
    (0x5a, 0, All, COL_MAP.is_gas),
    (0x5b, 0, All, COL_MAP.is_jumpdest),
    (0x5c, 2, User, COL_MAP.is_invalid_9), // 0x5c-5f
    (0x5c, 0, Kernel, COL_MAP.is_get_state_root),
    (0x5d, 0, Kernel, COL_MAP.is_set_state_root),
    (0x5e, 0, Kernel, COL_MAP.is_get_receipt_root),
    (0x5f, 0, Kernel, COL_MAP.is_set_receipt_root),
    (0x60, 5, All, COL_MAP.is_push), // 0x60-0x7f
    (0x80, 4, All, COL_MAP.is_dup),  // 0x80-0x8f
    (0x90, 4, All, COL_MAP.is_swap), // 0x90-0x9f
    (0xa0, 0, All, COL_MAP.is_log0),
    (0xa1, 0, All, COL_MAP.is_log1),
    (0xa2, 0, All, COL_MAP.is_log2),
    (0xa3, 0, All, COL_MAP.is_log3),
    (0xa4, 0, All, COL_MAP.is_log4),
    (0xa5, 0, User, COL_MAP.is_invalid_10),
    // Opcode 0xa5 is PANIC when Kernel. Make the proof unverifiable by giving it no flag to decode to.
    (0xa6, 1, All, COL_MAP.is_invalid_11), // 0xa6-0xa7
    (0xa8, 3, All, COL_MAP.is_invalid_12), // 0xa8-0xaf
    (0xb0, 4, All, COL_MAP.is_invalid_13), // 0xb0-0xbf
    (0xc0, 5, All, COL_MAP.is_invalid_14), // 0xc0-0xdf
    (0xe0, 4, All, COL_MAP.is_invalid_15), // 0xe0-0xef
    (0xf0, 0, All, COL_MAP.is_create),
    (0xf1, 0, All, COL_MAP.is_call),
    (0xf2, 0, All, COL_MAP.is_callcode),
    (0xf3, 0, All, COL_MAP.is_return),
    (0xf4, 0, All, COL_MAP.is_delegatecall),
    (0xf5, 0, All, COL_MAP.is_create2),
    (0xf6, 1, User, COL_MAP.is_invalid_16), // 0xf6-0xf7
    (0xf6, 0, Kernel, COL_MAP.is_get_context),
    (0xf7, 0, Kernel, COL_MAP.is_set_context),
    (0xf8, 1, User, COL_MAP.is_invalid_17), // 0xf8-0xf9
    (0xf8, 0, Kernel, COL_MAP.is_consume_gas),
    (0xf9, 0, Kernel, COL_MAP.is_exit_kernel),
    (0xfa, 0, All, COL_MAP.is_staticcall),
    (0xfb, 0, User, COL_MAP.is_invalid_18),
    (0xfb, 0, Kernel, COL_MAP.is_mload_general),
    (0xfc, 0, User, COL_MAP.is_invalid_19),
    (0xfc, 0, Kernel, COL_MAP.is_mstore_general),
    (0xfd, 0, All, COL_MAP.is_revert),
    (0xfe, 0, All, COL_MAP.is_invalid_20),
    (0xff, 0, All, COL_MAP.is_selfdestruct),
];

pub fn generate<F: RichField>(lv: &mut CpuColumnsView<F>) {
    let cycle_filter = lv.is_cpu_cycle;
    if cycle_filter == F::ZERO {
        // These columns cannot be shared.
        lv.is_eq = F::ZERO;
        lv.is_iszero = F::ZERO;
        return;
    }
    // This assert is not _strictly_ necessary, but I include it as a sanity check.
    assert_eq!(cycle_filter, F::ONE, "cycle_filter should be 0 or 1");

    let opcode = lv.opcode.to_canonical_u64();
    assert!(opcode < 256, "opcode should be in {{0, ..., 255}}");
    let opcode = opcode as u8;

    for (i, bit) in lv.opcode_bits.iter_mut().enumerate() {
        *bit = F::from_bool(opcode & (1 << i) != 0);
    }

    let top_bits: [u8; 9] = [
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

    let kernel = lv.is_kernel_mode.to_canonical_u64();
    assert!(kernel <= 1);
    let kernel = kernel != 0;

    for (oc, block_length, availability, col) in OPCODES {
        let available = match availability {
            All => true,
            User => !kernel,
            Kernel => kernel,
        };
        let opcode_match = top_bits[8 - block_length] == oc;
        lv[col] = F::from_bool(available && opcode_match);
    }
}

const fn bits_from_opcode(opcode: u8) -> [bool; 8] {
    [
        opcode & 1 != 0,
        opcode & 2 != 0,
        opcode & 4 != 0,
        opcode & 8 != 0,
        opcode & 16 != 0,
        opcode & 32 != 0,
        opcode & 64 != 0,
        opcode & 128 != 0,
    ]
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let cycle_filter = lv.is_cpu_cycle;

    // Ensure that the kernel flag is valid;
    let kernel_mode = lv.is_kernel_mode;
    yield_constr.constraint(cycle_filter * kernel_mode * (kernel_mode - P::ONES));

    // Ensure that the opcode bits are valid: each has to be either 0 or 1, and they must match
    // the opcode. Note that this also validates that this implicitly range-checks the opcode.
    let bits = lv.opcode_bits;
    // First check that the bits are either 0 or 1.
    for bit in bits {
        yield_constr.constraint(cycle_filter * bit * (bit - P::ONES));
    }

    // Now check that they match the opcode.
    {
        let opcode = lv.opcode;
        let reconstructed_opcode: P = bits
            .into_iter()
            .enumerate()
            .map(|(i, bit)| bit * P::Scalar::from_canonical_u64(1 << i))
            .sum();
        yield_constr.constraint(cycle_filter * (opcode - reconstructed_opcode));
    }

    // Check that the instruction flags are valid.
    // First, check that they are all either 0 or 1.
    for (_, _, _, flag_col) in OPCODES {
        let flag = lv[flag_col];
        yield_constr.constraint(cycle_filter * flag * (flag - P::ONES));
    }
    // Now check that exactly one is 1.
    let flag_sum: P = OPCODES
        .into_iter()
        .map(|(_, _, _, flag_col)| lv[flag_col])
        .sum();
    yield_constr.constraint(cycle_filter * (P::ONES - flag_sum));

    // Finally, classify all opcodes, together with the kernel flag, into blocks
    for (oc, block_length, availability, col) in OPCODES {
        let unavailable = match availability {
            All => P::ZEROS,
            User => kernel_mode,
            Kernel => P::ONES - kernel_mode,
        };
        let opcode_mismatch: P = bits
            .into_iter()
            .zip(bits_from_opcode(oc))
            .rev()
            .take(block_length + 1)
            .map(|(row_bit, flag_bit)| match flag_bit {
                false => row_bit,
                true => P::ONES - row_bit,
            })
            .sum();

        let constr = lv[col] * (unavailable + opcode_mismatch);
        yield_constr.constraint(cycle_filter * constr);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();

    let cycle_filter = lv.is_cpu_cycle;

    // Ensure that the kernel flag is valid;
    let kernel_mode = lv.is_kernel_mode;
    {
        let constr = builder.mul_sub_extension(kernel_mode, kernel_mode, kernel_mode);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // Ensure that the opcode bits are valid: each has to be either 0 or 1, and they must match
    // the opcode. Note that this also validates that this implicitly range-checks the opcode.
    let bits = lv.opcode_bits;
    // First check that the bits are either 0 or 1.
    for bit in bits {
        let constr = builder.mul_sub_extension(bit, bit, bit);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // Now check that they match the opcode.
    {
        let opcode = lv.opcode;
        let reconstructed_opcode =
            bits.into_iter()
                .enumerate()
                .fold(builder.zero_extension(), |cumul, (i, bit)| {
                    builder.mul_const_add_extension(F::from_canonical_u64(1 << i), bit, cumul)
                });
        let diff = builder.sub_extension(opcode, reconstructed_opcode);
        let constr = builder.mul_extension(cycle_filter, diff);
        yield_constr.constraint(builder, constr);
    }

    // Check that the instruction flags are valid.
    // First, check that they are all either 0 or 1.
    for (_, _, _, flag_col) in OPCODES {
        let flag = lv[flag_col];
        let constr = builder.mul_sub_extension(flag, flag, flag);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }
    // Now check that exactly one is 1.
    {
        let mut constr = builder.one_extension();
        for (_, _, _, flag_col) in OPCODES {
            let flag = lv[flag_col];
            constr = builder.sub_extension(constr, flag);
        }
        constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // Finally, classify all opcodes, together with the kernel flag, into blocks
    for (oc, block_length, availability, col) in OPCODES {
        let unavailable = match availability {
            All => builder.zero_extension(),
            User => kernel_mode,
            Kernel => builder.sub_extension(one, kernel_mode),
        };
        let opcode_mismatch = bits
            .into_iter()
            .zip(bits_from_opcode(oc))
            .rev()
            .take(block_length + 1)
            .fold(builder.zero_extension(), |cumul, (row_bit, flag_bit)| {
                let to_add = match flag_bit {
                    false => row_bit,
                    true => builder.sub_extension(one, row_bit),
                };
                builder.add_extension(cumul, to_add)
            });

        let constr = builder.add_extension(unavailable, opcode_mismatch);
        let constr = builder.mul_extension(lv[col], constr);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }
}
