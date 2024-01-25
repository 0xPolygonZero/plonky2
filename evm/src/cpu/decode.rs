use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

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
const OPCODES: [(u8, usize, bool, usize); 5] = [
    // (start index of block, number of top bits to check (log2), kernel-only, flag column)
    // ADD, MUL, SUB, DIV, MOD, LT, GT and BYTE flags are handled partly manually here, and partly through the Arithmetic table CTL.
    // ADDMOD, MULMOD and SUBMOD flags are handled partly manually here, and partly through the Arithmetic table CTL.
    // FP254 operation flags are handled partly manually here, and partly through the Arithmetic table CTL.
    (0x14, 1, false, COL_MAP.op.eq_iszero),
    // AND, OR and XOR flags are handled partly manually here, and partly through the Logic table CTL.
    // NOT and POP are handled manually here.
    // SHL and SHR flags are handled partly manually here, and partly through the Logic table CTL.
    // JUMPDEST and KECCAK_GENERAL are handled manually here.
    (0x56, 1, false, COL_MAP.op.jumps),     // 0x56-0x57
    (0x80, 5, false, COL_MAP.op.dup_swap),  // 0x80-0x9f
    (0xf6, 1, true, COL_MAP.op.context_op), //0xf6-0xf7
    (0xf9, 0, true, COL_MAP.op.exit_kernel),
    // MLOAD_GENERAL and MSTORE_GENERAL flags are handled manually here.
];

/// List of combined opcodes requiring a special handling.
/// Each index in the list corresponds to an arbitrary combination
/// of opcodes defined in evm/src/cpu/columns/ops.rs.
const COMBINED_OPCODES: [usize; 11] = [
    COL_MAP.op.logic_op,
    COL_MAP.op.fp254_op,
    COL_MAP.op.binary_op,
    COL_MAP.op.ternary_op,
    COL_MAP.op.shift,
    COL_MAP.op.m_op_general,
    COL_MAP.op.jumpdest_keccak_general,
    COL_MAP.op.not_pop,
    COL_MAP.op.pc_push0,
    COL_MAP.op.m_op_32bytes,
    COL_MAP.op.push_prover_input,
];

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

/// Evaluates the constraints for opcode decoding.
pub(crate) fn eval_packed_generic<P: PackedField>(
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
        .chain(COMBINED_OPCODES.map(|op| lv[op]))
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

    let opcode_high_bits = |num_high_bits| -> P {
        lv.opcode_bits
            .into_iter()
            .enumerate()
            .rev()
            .take(num_high_bits)
            .map(|(i, bit)| bit * P::Scalar::from_canonical_u64(1 << i))
            .sum()
    };

    // Manually check lv.op.m_op_constr
    let opcode = opcode_high_bits(8);
    yield_constr.constraint((P::ONES - kernel_mode) * lv.op.m_op_general);

    let m_op_constr = (opcode - P::Scalar::from_canonical_usize(0xfb_usize))
        * (opcode - P::Scalar::from_canonical_usize(0xfc_usize))
        * lv.op.m_op_general;
    yield_constr.constraint(m_op_constr);

    // Manually check lv.op.jumpdest_keccak_general.
    // KECCAK_GENERAL is a kernel-only instruction, but not JUMPDEST.
    // JUMPDEST is differentiated from KECCAK_GENERAL by its second bit set to 1.
    yield_constr.constraint(
        (P::ONES - kernel_mode) * lv.op.jumpdest_keccak_general * (P::ONES - lv.opcode_bits[1]),
    );

    // Check the JUMPDEST and KERNEL_GENERAL opcodes.
    let jumpdest_opcode = P::Scalar::from_canonical_usize(0x5b);
    let keccak_general_opcode = P::Scalar::from_canonical_usize(0x21);
    let jumpdest_keccak_general_constr = (opcode - keccak_general_opcode)
        * (opcode - jumpdest_opcode)
        * lv.op.jumpdest_keccak_general;
    yield_constr.constraint(jumpdest_keccak_general_constr);

    // Manually check lv.op.pc_push0.
    // Both PC and PUSH0 can be called outside of the kernel mode:
    // there is no need to constrain them in that regard.
    let pc_push0_constr = (opcode - P::Scalar::from_canonical_usize(0x58_usize))
        * (opcode - P::Scalar::from_canonical_usize(0x5f_usize))
        * lv.op.pc_push0;
    yield_constr.constraint(pc_push0_constr);

    // Manually check lv.op.not_pop.
    // Both NOT and POP can be called outside of the kernel mode:
    // there is no need to constrain them in that regard.
    let not_pop_op = (opcode - P::Scalar::from_canonical_usize(0x19_usize))
        * (opcode - P::Scalar::from_canonical_usize(0x50_usize))
        * lv.op.not_pop;
    yield_constr.constraint(not_pop_op);

    // Manually check lv.op.m_op_32bytes.
    // Both are kernel-only.
    yield_constr.constraint((P::ONES - kernel_mode) * lv.op.m_op_32bytes);

    // Check the MSTORE_32BYTES and MLOAD-32BYTES opcodes.
    let opcode_high_three = opcode_high_bits(3);
    let op_32bytes = (opcode_high_three - P::Scalar::from_canonical_usize(0xc0_usize))
        * (opcode - P::Scalar::from_canonical_usize(0xf8_usize))
        * lv.op.m_op_32bytes;
    yield_constr.constraint(op_32bytes);

    // Manually check PUSH and PROVER_INPUT.
    // PROVER_INPUT is a kernel-only instruction, but not PUSH.
    let push_prover_input_constr = (opcode - P::Scalar::from_canonical_usize(0x49_usize))
        * (opcode_high_three - P::Scalar::from_canonical_usize(0x60_usize))
        * lv.op.push_prover_input;
    yield_constr.constraint(push_prover_input_constr);
    let prover_input_constr =
        lv.op.push_prover_input * (lv.opcode_bits[5] - P::ONES) * (P::ONES - kernel_mode);
    yield_constr.constraint(prover_input_constr);
}

fn opcode_high_bits_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    num_high_bits: usize,
) -> ExtensionTarget<D> {
    lv.opcode_bits
        .into_iter()
        .enumerate()
        .rev()
        .take(num_high_bits)
        .fold(builder.zero_extension(), |cumul, (i, bit)| {
            builder.mul_const_add_extension(F::from_canonical_usize(1 << i), bit, cumul)
        })
}

/// Circuit version of `eval_packed_generic`.
/// Evaluates the constraints for opcode decoding.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
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

    // Manually check lv.op.m_op_constr
    let opcode = opcode_high_bits_circuit(builder, lv, 8);

    let mload_opcode = builder.constant_extension(F::Extension::from_canonical_usize(0xfb_usize));
    let mstore_opcode = builder.constant_extension(F::Extension::from_canonical_usize(0xfc_usize));

    let one_extension = builder.constant_extension(F::Extension::ONE);
    let is_not_kernel_mode = builder.sub_extension(one_extension, kernel_mode);
    let constr = builder.mul_extension(is_not_kernel_mode, lv.op.m_op_general);
    yield_constr.constraint(builder, constr);

    let mload_constr = builder.sub_extension(opcode, mload_opcode);
    let mstore_constr = builder.sub_extension(opcode, mstore_opcode);
    let mut m_op_constr = builder.mul_extension(mload_constr, mstore_constr);
    m_op_constr = builder.mul_extension(m_op_constr, lv.op.m_op_general);

    yield_constr.constraint(builder, m_op_constr);

    // Manually check lv.op.jumpdest_keccak_general.
    // KECCAK_GENERAL is a kernel-only instruction, but not JUMPDEST.
    // JUMPDEST is differentiated from KECCAK_GENERAL by its second bit set to 1.
    let jumpdest_opcode =
        builder.constant_extension(F::Extension::from_canonical_usize(0x5b_usize));
    let keccak_general_opcode =
        builder.constant_extension(F::Extension::from_canonical_usize(0x21_usize));

    // Check that KECCAK_GENERAL is kernel-only.
    let mut kernel_general_filter = builder.sub_extension(one, lv.opcode_bits[1]);
    kernel_general_filter =
        builder.mul_extension(lv.op.jumpdest_keccak_general, kernel_general_filter);
    let constr = builder.mul_extension(is_not_kernel_mode, kernel_general_filter);
    yield_constr.constraint(builder, constr);

    // Check the JUMPDEST and KERNEL_GENERAL opcodes.
    let jumpdest_constr = builder.sub_extension(opcode, jumpdest_opcode);
    let keccak_general_constr = builder.sub_extension(opcode, keccak_general_opcode);
    let mut jumpdest_keccak_general_constr =
        builder.mul_extension(jumpdest_constr, keccak_general_constr);
    jumpdest_keccak_general_constr = builder.mul_extension(
        jumpdest_keccak_general_constr,
        lv.op.jumpdest_keccak_general,
    );

    yield_constr.constraint(builder, jumpdest_keccak_general_constr);

    // Manually check lv.op.pc_push0.
    // Both PC and PUSH0 can be called outside of the kernel mode:
    // there is no need to constrain them in that regard.
    let pc_opcode = builder.constant_extension(F::Extension::from_canonical_usize(0x58_usize));
    let push0_opcode = builder.constant_extension(F::Extension::from_canonical_usize(0x5f_usize));
    let pc_constr = builder.sub_extension(opcode, pc_opcode);
    let push0_constr = builder.sub_extension(opcode, push0_opcode);
    let mut pc_push0_constr = builder.mul_extension(pc_constr, push0_constr);
    pc_push0_constr = builder.mul_extension(pc_push0_constr, lv.op.pc_push0);
    yield_constr.constraint(builder, pc_push0_constr);

    // Manually check lv.op.not_pop.
    // Both NOT and POP can be called outside of the kernel mode:
    // there is no need to constrain them in that regard.
    let not_opcode = builder.constant_extension(F::Extension::from_canonical_usize(0x19_usize));
    let pop_opcode = builder.constant_extension(F::Extension::from_canonical_usize(0x50_usize));

    let not_constr = builder.sub_extension(opcode, not_opcode);
    let pop_constr = builder.sub_extension(opcode, pop_opcode);

    let mut not_pop_constr = builder.mul_extension(not_constr, pop_constr);
    not_pop_constr = builder.mul_extension(lv.op.not_pop, not_pop_constr);
    yield_constr.constraint(builder, not_pop_constr);

    // Manually check lv.op.m_op_32bytes.
    // Both are kernel-only.
    let constr = builder.mul_extension(is_not_kernel_mode, lv.op.m_op_32bytes);
    yield_constr.constraint(builder, constr);

    // Check the MSTORE_32BYTES and MLOAD-32BYTES opcodes.
    let opcode_high_three = opcode_high_bits_circuit(builder, lv, 3);
    let mstore_32bytes_opcode =
        builder.constant_extension(F::Extension::from_canonical_usize(0xc0_usize));
    let mload_32bytes_opcode =
        builder.constant_extension(F::Extension::from_canonical_usize(0xf8_usize));
    let mstore_32bytes_constr = builder.sub_extension(opcode_high_three, mstore_32bytes_opcode);
    let mload_32bytes_constr = builder.sub_extension(opcode, mload_32bytes_opcode);
    let constr = builder.mul_extension(mstore_32bytes_constr, mload_32bytes_constr);
    let constr = builder.mul_extension(constr, lv.op.m_op_32bytes);
    yield_constr.constraint(builder, constr);

    // Manually check PUSH and PROVER_INPUT.
    // PROVER_INPUT is a kernel-only instruction, but not PUSH.
    let prover_input_opcode =
        builder.constant_extension(F::Extension::from_canonical_usize(0x49usize));
    let push_opcodes = builder.constant_extension(F::Extension::from_canonical_usize(0x60usize));

    let push_constr = builder.sub_extension(opcode_high_three, push_opcodes);
    let prover_input_constr = builder.sub_extension(opcode, prover_input_opcode);

    let push_prover_input_constr =
        builder.mul_many_extension([lv.op.push_prover_input, prover_input_constr, push_constr]);
    yield_constr.constraint(builder, push_prover_input_constr);
    let prover_input_filter = builder.mul_sub_extension(
        lv.op.push_prover_input,
        lv.opcode_bits[5],
        lv.op.push_prover_input,
    );
    let constr = builder.mul_extension(prover_input_filter, is_not_kernel_mode);
    yield_constr.constraint(builder, constr);
}
