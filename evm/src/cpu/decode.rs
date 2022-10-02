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
const OPCODES: [(u8, usize, bool, usize); 93] = [
    // (start index of block, number of top bits to check (log2), kernel-only, flag column)
    (0x00, 0, false, COL_MAP.op.stop),
    (0x01, 0, false, COL_MAP.op.add),
    (0x02, 0, false, COL_MAP.op.mul),
    (0x03, 0, false, COL_MAP.op.sub),
    (0x04, 0, false, COL_MAP.op.div),
    (0x05, 0, false, COL_MAP.op.sdiv),
    (0x06, 0, false, COL_MAP.op.mod_),
    (0x07, 0, false, COL_MAP.op.smod),
    (0x08, 0, false, COL_MAP.op.addmod),
    (0x09, 0, false, COL_MAP.op.mulmod),
    (0x0a, 0, false, COL_MAP.op.exp),
    (0x0b, 0, false, COL_MAP.op.signextend),
    (0x10, 0, false, COL_MAP.op.lt),
    (0x11, 0, false, COL_MAP.op.gt),
    (0x12, 0, false, COL_MAP.op.slt),
    (0x13, 0, false, COL_MAP.op.sgt),
    (0x14, 0, false, COL_MAP.op.eq),
    (0x15, 0, false, COL_MAP.op.iszero),
    (0x16, 0, false, COL_MAP.op.and),
    (0x17, 0, false, COL_MAP.op.or),
    (0x18, 0, false, COL_MAP.op.xor),
    (0x19, 0, false, COL_MAP.op.not),
    (0x1a, 0, false, COL_MAP.op.byte),
    (0x1b, 0, false, COL_MAP.op.shl),
    (0x1c, 0, false, COL_MAP.op.shr),
    (0x1d, 0, false, COL_MAP.op.sar),
    (0x20, 0, false, COL_MAP.op.keccak256),
    (0x21, 0, true, COL_MAP.op.keccak_general),
    (0x30, 0, false, COL_MAP.op.address),
    (0x31, 0, false, COL_MAP.op.balance),
    (0x32, 0, false, COL_MAP.op.origin),
    (0x33, 0, false, COL_MAP.op.caller),
    (0x34, 0, false, COL_MAP.op.callvalue),
    (0x35, 0, false, COL_MAP.op.calldataload),
    (0x36, 0, false, COL_MAP.op.calldatasize),
    (0x37, 0, false, COL_MAP.op.calldatacopy),
    (0x38, 0, false, COL_MAP.op.codesize),
    (0x39, 0, false, COL_MAP.op.codecopy),
    (0x3a, 0, false, COL_MAP.op.gasprice),
    (0x3b, 0, false, COL_MAP.op.extcodesize),
    (0x3c, 0, false, COL_MAP.op.extcodecopy),
    (0x3d, 0, false, COL_MAP.op.returndatasize),
    (0x3e, 0, false, COL_MAP.op.returndatacopy),
    (0x3f, 0, false, COL_MAP.op.extcodehash),
    (0x40, 0, false, COL_MAP.op.blockhash),
    (0x41, 0, false, COL_MAP.op.coinbase),
    (0x42, 0, false, COL_MAP.op.timestamp),
    (0x43, 0, false, COL_MAP.op.number),
    (0x44, 0, false, COL_MAP.op.difficulty),
    (0x45, 0, false, COL_MAP.op.gaslimit),
    (0x46, 0, false, COL_MAP.op.chainid),
    (0x47, 0, false, COL_MAP.op.selfbalance),
    (0x48, 0, false, COL_MAP.op.basefee),
    (0x49, 0, true, COL_MAP.op.prover_input),
    (0x50, 0, false, COL_MAP.op.pop),
    (0x51, 0, false, COL_MAP.op.mload),
    (0x52, 0, false, COL_MAP.op.mstore),
    (0x53, 0, false, COL_MAP.op.mstore8),
    (0x54, 0, false, COL_MAP.op.sload),
    (0x55, 0, false, COL_MAP.op.sstore),
    (0x56, 0, false, COL_MAP.op.jump),
    (0x57, 0, false, COL_MAP.op.jumpi),
    (0x58, 0, false, COL_MAP.op.pc),
    (0x59, 0, false, COL_MAP.op.msize),
    (0x5a, 0, false, COL_MAP.op.gas),
    (0x5b, 0, false, COL_MAP.op.jumpdest),
    (0x5c, 0, true, COL_MAP.op.get_state_root),
    (0x5d, 0, true, COL_MAP.op.set_state_root),
    (0x5e, 0, true, COL_MAP.op.get_receipt_root),
    (0x5f, 0, true, COL_MAP.op.set_receipt_root),
    (0x60, 5, false, COL_MAP.op.push), // 0x60-0x7f
    (0x80, 4, false, COL_MAP.op.dup),  // 0x80-0x8f
    (0x90, 4, false, COL_MAP.op.swap), // 0x90-0x9f
    (0xa0, 0, false, COL_MAP.op.log0),
    (0xa1, 0, false, COL_MAP.op.log1),
    (0xa2, 0, false, COL_MAP.op.log2),
    (0xa3, 0, false, COL_MAP.op.log3),
    (0xa4, 0, false, COL_MAP.op.log4),
    // Opcode 0xa5 is PANIC when Kernel. Make the proof unverifiable by giving it no flag to decode to.
    (0xf0, 0, false, COL_MAP.op.create),
    (0xf1, 0, false, COL_MAP.op.call),
    (0xf2, 0, false, COL_MAP.op.callcode),
    (0xf3, 0, false, COL_MAP.op.return_),
    (0xf4, 0, false, COL_MAP.op.delegatecall),
    (0xf5, 0, false, COL_MAP.op.create2),
    (0xf6, 0, true, COL_MAP.op.get_context),
    (0xf7, 0, true, COL_MAP.op.set_context),
    (0xf8, 0, true, COL_MAP.op.consume_gas),
    (0xf9, 0, true, COL_MAP.op.exit_kernel),
    (0xfa, 0, false, COL_MAP.op.staticcall),
    (0xfb, 0, true, COL_MAP.op.mload_general),
    (0xfc, 0, true, COL_MAP.op.mstore_general),
    (0xfd, 0, false, COL_MAP.op.revert),
    (0xff, 0, false, COL_MAP.op.selfdestruct),
];

/// Bitfield of invalid opcodes, in little-endian order.
pub(crate) const fn invalid_opcodes_user() -> [u8; 32] {
    let mut res = [u8::MAX; 32]; // Start with all opcodes marked invalid.

    let mut i = 0;
    while i < OPCODES.len() {
        let (block_start, lb_block_len, kernel_only, _) = OPCODES[i];
        i += 1;

        if kernel_only {
            continue;
        }

        let block_len = 1 << lb_block_len;
        let block_start = block_start as usize;
        let block_end = block_start + block_len;
        let mut j = block_start;
        while j < block_end {
            let byte = j / u8::BITS as usize;
            let bit = j % u8::BITS as usize;
            res[byte] &= !(1 << bit); // Mark opcode as invalid by zeroing the bit.
            j += 1;
        }
    }
    res
}

pub fn generate<F: RichField>(lv: &mut CpuColumnsView<F>) {
    let cycle_filter = lv.is_cpu_cycle;
    if cycle_filter == F::ZERO {
        // These columns cannot be shared.
        lv.op.eq = F::ZERO;
        lv.op.iszero = F::ZERO;
        return;
    }
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

    let mut any_flag_set = false;
    for (oc, block_length, kernel_only, col) in OPCODES {
        let available = !kernel_only || kernel;
        let opcode_match = top_bits[8 - block_length] == oc;
        let flag = available && opcode_match;
        lv[col] = F::from_bool(flag);
        if flag && any_flag_set {
            panic!("opcode matched multiple flags");
        }
        any_flag_set = any_flag_set || flag;
    }
    // is_invalid is a catch-all for opcodes we can't decode.
    lv.op.invalid = F::from_bool(!any_flag_set);
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
    let cycle_filter = lv.is_cpu_cycle;

    // Ensure that the kernel flag is valid (either 0 or 1).
    let kernel_mode = lv.is_kernel_mode;
    yield_constr.constraint(cycle_filter * kernel_mode * (kernel_mode - P::ONES));

    // Ensure that the opcode bits are valid: each has to be either 0 or 1.
    for bit in lv.opcode_bits {
        yield_constr.constraint(cycle_filter * bit * (bit - P::ONES));
    }

    // Check that the instruction flags are valid.
    // First, check that they are all either 0 or 1.
    for (_, _, _, flag_col) in OPCODES {
        let flag = lv[flag_col];
        yield_constr.constraint(cycle_filter * flag * (flag - P::ONES));
    }
    yield_constr.constraint(cycle_filter * lv.op.invalid * (lv.op.invalid - P::ONES));
    // Now check that exactly one is 1.
    let flag_sum: P = OPCODES
        .into_iter()
        .map(|(_, _, _, flag_col)| lv[flag_col])
        .sum::<P>()
        + lv.op.invalid;
    yield_constr.constraint(cycle_filter * (P::ONES - flag_sum));

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
            .take(block_length + 1)
            .map(|(row_bit, flag_bit)| match flag_bit {
                // 1 if the bit does not match, and 0 otherwise
                false => row_bit,
                true => P::ONES - row_bit,
            })
            .sum();

        // If unavailable + opcode_mismatch is 0, then the opcode bits all match and we are in the
        // correct mode.
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

    // Ensure that the kernel flag is valid (either 0 or 1).
    let kernel_mode = lv.is_kernel_mode;
    {
        let constr = builder.mul_sub_extension(kernel_mode, kernel_mode, kernel_mode);
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // Ensure that the opcode bits are valid: each has to be either 0 or 1.
    for bit in lv.opcode_bits {
        let constr = builder.mul_sub_extension(bit, bit, bit);
        let constr = builder.mul_extension(cycle_filter, constr);
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
    {
        let constr = builder.mul_sub_extension(lv.op.invalid, lv.op.invalid, lv.op.invalid);
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
        constr = builder.sub_extension(constr, lv.op.invalid);
        constr = builder.mul_extension(cycle_filter, constr);
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
            .take(block_length + 1)
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
        let constr = builder.mul_extension(cycle_filter, constr);
        yield_constr.constraint(builder, constr);
    }
}
