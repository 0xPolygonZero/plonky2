use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP};

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
/// Note: invalid opcodes are not represented here. _Any_ opcode is permitted to decode to
/// `is_invalid`. The kernel then verifies that the opcode was _actually_ invalid.
const OPCODES: [(u8, usize, bool, usize); 19] = [
    // (start index of block, number of top bits to check (log2), kernel-only, flag column)
    // ADD, MUL, SUB, DIV, MOD, LT and GT flags are handled partly manually here, and partly through the Arithmetic table CTL.
    // ADDMOD, MULMOD and SUBMOD flags are handled partly manually here, and partly through the Arithmetic table CTL.
    // FP254 operation flags are handled partly manually here, and partly through the Arithmetic table CTL.
    (0x14, 1, false, COL_MAP.op.eq_iszero),
    // AND, OR and XOR flags are handled partly manually here, and partly through the Logic table CTL.
    (0x19, 0, false, COL_MAP.op.not),
    (0x1a, 0, false, COL_MAP.op.byte),
    // SHL and SHR flags are handled partly manually here, and partly through the Logic table CTL.
    (0x21, 0, true, COL_MAP.op.keccak_general),
    (0x49, 0, true, COL_MAP.op.prover_input),
    (0x50, 0, false, COL_MAP.op.pop),
    (0x56, 1, false, COL_MAP.op.jumps), // 0x56-0x57
    (0x58, 0, false, COL_MAP.op.pc),
    (0x5b, 0, false, COL_MAP.op.jumpdest),
    (0x5f, 0, false, COL_MAP.op.push0),
    (0x60, 5, false, COL_MAP.op.push),      // 0x60-0x7f
    (0x80, 4, false, COL_MAP.op.dup),       // 0x80-0x8f
    (0x90, 4, false, COL_MAP.op.swap),      // 0x90-0x9f
    (0xf6, 1, true, COL_MAP.op.context_op), // 0xf6-0xf7
    (0xf9, 0, true, COL_MAP.op.exit_kernel),
    (0xfb, 0, true, COL_MAP.op.mload_general),
    (0xfc, 0, true, COL_MAP.op.mstore_general),
];

/// List of combined opcodes requiring a special handling.
/// Each index in the list corresponds to an arbitrary combination
/// of opcodes defined in evm/src/cpu/columns/ops.rs.
const COMBINED_OPCODES: [usize; 5] = [
    COL_MAP.op.logic_op,
    COL_MAP.op.fp254_op,
    COL_MAP.op.binary_op,
    COL_MAP.op.ternary_op,
    COL_MAP.op.shift,
];

pub fn generate<F: RichField>(lv: &mut CpuColumnsView<F>) {
    let cycle_filter: F = COL_MAP.op.iter().map(|&col_i| lv[col_i]).sum();

    // This assert is not _strictly_ necessary, but I include it as a sanity check.
    assert_eq!(cycle_filter, F::ONE, "cycle_filter should be 0 or 1");

    // Validate all opcode bits.
    for bit in lv.opcode_bits.into_iter() {
        assert!(bit.to_canonical_u64() <= 1);
    }
    let opcode = lv
        .opcode_bits
        .into_iter()
        .enumerate()
        .map(|(i, bit)| bit.to_canonical_u64() << i)
        .sum::<u64>() as u8;

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

    for (oc, block_length, kernel_only, col) in OPCODES {
        let available = !kernel_only || kernel;
        let opcode_match = top_bits[8 - block_length] == oc;
        let flag = available && opcode_match;
        lv[col] = F::from_bool(flag);
    }
}

/// Break up an opcode (which is 8 bits long) into its eight bits.
const fn bits_from_opcode(opcode: u8) -> [bool; 8] {
    [
        opcode & (1 << 0) != 0,
        opcode & (1 << 1) != 0,
        opcode & (1 << 2) != 0,
        opcode & (1 << 3) != 0,
        opcode & (1 << 4) != 0,
        opcode & (1 << 5) != 0,
        opcode & (1 << 6) != 0,
        opcode & (1 << 7) != 0,
    ]
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // Ensure that the kernel flag is valid (either 0 or 1).
    let kernel_mode = lv.is_kernel_mode;
    yield_constr.constraint(kernel_mode * (kernel_mode - P::ONES));

    // Ensure that the opcode bits are valid: each has to be either 0 or 1.
    for bit in lv.opcode_bits {
        yield_constr.constraint(bit * (bit - P::ONES));
    }

    // Check that the instruction flags are valid.
    // First, check that they are all either 0 or 1.
    for (_, _, _, flag_col) in OPCODES {
        let flag = lv[flag_col];
        yield_constr.constraint(flag * (flag - P::ONES));
    }
    // Also check that the combined instruction flags are valid.
    for flag_idx in COMBINED_OPCODES {
        yield_constr.constraint(lv[flag_idx] * (lv[flag_idx] - P::ONES));
    }

    // Now check that they sum to 0 or 1, including the combined flags.
    let flag_sum: P = OPCODES
        .into_iter()
        .map(|(_, _, _, flag_col)| lv[flag_col])
        .chain(COMBINED_OPCODES.into_iter().map(|flag_col| lv[flag_col]))
        .sum::<P>();
    yield_constr.constraint(flag_sum * (flag_sum - P::ONES));

    // Finally, classify all opcodes, together with the kernel flag, into blocks
    for (oc, block_length, kernel_only, col) in OPCODES {
        // 0 if the block/flag is available to us (is always available or we are in kernel mode) and
        // 1 otherwise.
        let unavailable = match kernel_only {
            false => P::ZEROS,
            true => P::ONES - kernel_mode,
        };
        // 0 if all the opcode bits match, and something in {1, ..., 8}, otherwise.
        let opcode_mismatch: P = lv
            .opcode_bits
            .into_iter()
            .zip(bits_from_opcode(oc))
            .rev()
            .take(8 - block_length)
            .map(|(row_bit, flag_bit)| match flag_bit {
                // 1 if the bit does not match, and 0 otherwise
                false => row_bit,
                true => P::ONES - row_bit,
            })
            .sum();

        // If unavailable + opcode_mismatch is 0, then the opcode bits all match and we are in the
        // correct mode.
        yield_constr.constraint(lv[col] * (unavailable + opcode_mismatch));
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();

    // Note: The constraints below do not need to be restricted to CPU cycles.

    // Ensure that the kernel flag is valid (either 0 or 1).
    let kernel_mode = lv.is_kernel_mode;
    {
        let constr = builder.mul_sub_extension(kernel_mode, kernel_mode, kernel_mode);
        yield_constr.constraint(builder, constr);
    }

    // Ensure that the opcode bits are valid: each has to be either 0 or 1.
    for bit in lv.opcode_bits {
        let constr = builder.mul_sub_extension(bit, bit, bit);
        yield_constr.constraint(builder, constr);
    }

    // Check that the instruction flags are valid.
    // First, check that they are all either 0 or 1.
    for (_, _, _, flag_col) in OPCODES {
        let flag = lv[flag_col];
        let constr = builder.mul_sub_extension(flag, flag, flag);
        yield_constr.constraint(builder, constr);
    }
    // Also check that the combined instruction flags are valid.
    for flag_idx in COMBINED_OPCODES {
        let constr = builder.mul_sub_extension(lv[flag_idx], lv[flag_idx], lv[flag_idx]);
        yield_constr.constraint(builder, constr);
    }

    // Now check that they sum to 0 or 1, including the combined flags.
    {
        let mut flag_sum =
            builder.add_many_extension(COMBINED_OPCODES.into_iter().map(|idx| lv[idx]));
        for (_, _, _, flag_col) in OPCODES {
            let flag = lv[flag_col];
            flag_sum = builder.add_extension(flag_sum, flag);
        }
        let constr = builder.mul_sub_extension(flag_sum, flag_sum, flag_sum);
        yield_constr.constraint(builder, constr);
    }

    // Finally, classify all opcodes, together with the kernel flag, into blocks
    for (oc, block_length, kernel_only, col) in OPCODES {
        // 0 if the block/flag is available to us (is always available or we are in kernel mode) and
        // 1 otherwise.
        let unavailable = match kernel_only {
            false => builder.zero_extension(),
            true => builder.sub_extension(one, kernel_mode),
        };
        // 0 if all the opcode bits match, and something in {1, ..., 8}, otherwise.
        let opcode_mismatch = lv
            .opcode_bits
            .into_iter()
            .zip(bits_from_opcode(oc))
            .rev()
            .take(8 - block_length)
            .fold(builder.zero_extension(), |cumul, (row_bit, flag_bit)| {
                let to_add = match flag_bit {
                    false => row_bit,
                    true => builder.sub_extension(one, row_bit),
                };
                builder.add_extension(cumul, to_add)
            });

        // If unavailable + opcode_mismatch is 0, then the opcode bits all match and we are in the
        // correct mode.
        let constr = builder.add_extension(unavailable, opcode_mismatch);
        let constr = builder.mul_extension(lv[col], constr);
        yield_constr.constraint(builder, constr);
    }
}
