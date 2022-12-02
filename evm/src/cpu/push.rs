use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;

use crate::constraint_consumer::ConstraintConsumer;
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

fn join_bits_into_byte_le<P: PackedField>(bits: [P; 8]) -> P {
    bits[0]
        + bits[1] * P::Scalar::from_canonical_u64(1 << 1)
        + bits[2] * P::Scalar::from_canonical_u64(1 << 2)
        + bits[3] * P::Scalar::from_canonical_u64(1 << 3)
        + bits[4] * P::Scalar::from_canonical_u64(1 << 4)
        + bits[5] * P::Scalar::from_canonical_u64(1 << 5)
        + bits[6] * P::Scalar::from_canonical_u64(1 << 6)
        + bits[7] * P::Scalar::from_canonical_u64(1 << 7)
}

fn join_bb_le<P: PackedField>(b0: P, b1: P) -> P {
    b0 + b1 * P::Scalar::from_canonical_u64(1 << 8)
}

fn join_bbbb_le<P: PackedField>(b0: P, b1: P, b2: P, b3: P) -> P {
    b0 + b1 * P::Scalar::from_canonical_u64(1 << 8)
        + b2 * P::Scalar::from_canonical_u64(1 << 16)
        + b1 * P::Scalar::from_canonical_u64(1 << 24)
}

fn join_bbh_le<P: PackedField>(b0: P, b1: P, h23: P) -> P {
    b0 + b1 * P::Scalar::from_canonical_u64(1 << 8) + h23 * P::Scalar::from_canonical_u64(1 << 16)
}

fn join_hh_le<P: PackedField>(h01: P, h23: P) -> P {
    h01 + h23 * P::Scalar::from_canonical_u64(1 << 16)
}

pub fn eval_packed_push<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let nv_push = nv.general.push();

    let is_push = lv.op.push;

    let immediate = lv.opcode_bits[0]
        + lv.opcode_bits[1] * P::Scalar::from_canonical_u64(2)
        + lv.opcode_bits[2] * P::Scalar::from_canonical_u64(4)
        + lv.opcode_bits[3] * P::Scalar::from_canonical_u64(8)
        + lv.opcode_bits[4] * P::Scalar::from_canonical_u64(16);

    let bytes_to_push = P::ONES + immediate;

    // Set channel read flag and address. Note that we're not setting `used`.
    // The last GP channel is skipped, as that's the one that writes to the stack.
    let base_addr = lv.program_counter + bytes_to_push;
    for i in 0..NUM_GP_CHANNELS - 1 {
        let channel = lv.mem_channels[i];
        yield_constr.constraint(is_push * (channel.is_read - P::ONES));
        yield_constr.constraint(is_push * (channel.addr_context - lv.code_context));
        yield_constr.constraint(
            is_push
                * (channel.addr_segment - P::Scalar::from_canonical_usize(Segment::Code as usize)),
        );

        let address = base_addr - P::Scalar::from_canonical_usize(i);
        yield_constr.constraint(is_push * (channel.addr_virtual - address));
    }

    // First channel is always used (pushing at least one byte).
    yield_constr.constraint(is_push * (lv.mem_channels[0].used - P::ONES));

    // If a channel is unused, then all the following channels must also be unused.
    // Note that channel 0 is always used, so channel 1 is unconstrained here.
    for i in 2..NUM_GP_CHANNELS - 1 {
        let this_channel = lv.mem_channels[i];
        let previous_channel = lv.mem_channels[i - 1];

        // previous_channel.used = 0 => this_channel.used = 0
        // <=> previous_channel.used != 0 or this_channel.used = 0
        yield_constr.constraint(is_push * (previous_channel.used - P::ONES) * this_channel.used);
    }

    // If a channel is disabled, then its value is zero.
    for i in 1..NUM_GP_CHANNELS - 1 {
        let channel = lv.mem_channels[i];
        // NB: We do not need to constrain `channel.value` at indices other than 0 as those are
        // ignored.
        yield_constr.constraint(is_push * (channel.used - P::ONES) * channel.value[0]);
    }

    // Verify that the number of used channels corresponds to the number of bytes being pushed.
    let other_channels_used_sum: P = lv.mem_channels[1..NUM_GP_CHANNELS - 1]
        .iter()
        .map(|channel| channel.used)
        .sum();
    yield_constr.constraint_transition(
        is_push * (immediate - other_channels_used_sum - nv.is_push_cont * nv_push.rem_bytes),
    );
    yield_constr.constraint_last_row(is_push * (immediate - other_channels_used_sum));

    // If this PUSH is continued, then we must have exhausted our memory channels
    yield_constr.constraint_transition(
        is_push * nv.is_push_cont * (lv.mem_channels[NUM_GP_CHANNELS - 2].used - P::ONES),
    );

    // If this PUSH is continued, we constrain the program counter and `next_pc`.
    let next_row_pc = base_addr - P::Scalar::from_canonical_usize(NUM_GP_CHANNELS - 1);
    let next_instr_pc = base_addr + P::ONES;
    yield_constr
        .constraint_transition(is_push * nv.is_push_cont * (nv.program_counter - next_row_pc));
    yield_constr
        .constraint_transition(is_push * nv.is_push_cont * (nv_push.next_pc - next_instr_pc));
    // Otherwise, we constrain the program counter.
    yield_constr.constraint_transition(
        is_push * (nv.is_push_cont - P::ONES) * (nv.program_counter - next_instr_pc),
    );

    // Finally, we constrain the writing channel.
    let write_val = lv.mem_channels[NUM_GP_CHANNELS - 1].value;
    yield_constr.constraint(
        is_push
            * (write_val[0]
                - join_bbbb_le(
                    lv.mem_channels[0].value[0],
                    lv.mem_channels[1].value[0],
                    lv.mem_channels[2].value[0],
                    lv.mem_channels[3].value[0],
                )),
    );
    yield_constr.constraint_transition(
        is_push
            * (write_val[1]
                - nv.is_push_cont
                    * join_bbbb_le(
                        join_bits_into_byte_le(nv.opcode_bits),
                        nv.mem_channels[0].value[0],
                        nv.mem_channels[1].value[0],
                        nv.mem_channels[2].value[0],
                    )),
    );
    yield_constr.constraint_transition(
        is_push
            * (write_val[2]
                - nv.is_push_cont
                    * join_bbh_le(
                        nv.mem_channels[3].value[0],
                        nv.mem_channels[4].value[0],
                        nv_push.higher_limbs[0],
                    )),
    );
    yield_constr.constraint_transition(
        is_push
            * (write_val[3]
                - nv.is_push_cont * join_hh_le(nv_push.higher_limbs[1], nv_push.higher_limbs[2])),
    );
    yield_constr.constraint_transition(
        is_push
            * (write_val[4]
                - nv.is_push_cont * join_hh_le(nv_push.higher_limbs[3], nv_push.higher_limbs[4])),
    );
    yield_constr.constraint_transition(
        is_push
            * (write_val[5]
                - nv.is_push_cont * join_hh_le(nv_push.higher_limbs[5], nv_push.higher_limbs[6])),
    );
    yield_constr.constraint_transition(
        is_push
            * (write_val[6]
                - nv.is_push_cont * join_hh_le(nv_push.higher_limbs[7], nv_push.higher_limbs[8])),
    );
    yield_constr.constraint_transition(
        is_push
            * (write_val[7]
                - nv.is_push_cont * join_hh_le(nv_push.higher_limbs[9], nv_push.higher_limbs[10])),
    );
    yield_constr.constraint_last_row(is_push * write_val[1]);
    yield_constr.constraint_last_row(is_push * write_val[2]);
    yield_constr.constraint_last_row(is_push * write_val[3]);
    yield_constr.constraint_last_row(is_push * write_val[4]);
    yield_constr.constraint_last_row(is_push * write_val[5]);
    yield_constr.constraint_last_row(is_push * write_val[6]);
    yield_constr.constraint_last_row(is_push * write_val[7]);
}

pub fn eval_packed_push_cont<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let lv_push = nv.general.push();
    let nv_push = nv.general.push();

    // Set channel read flag and address. Note that we're not setting `used`.
    // The last GP channel is skipped, as that's the one that writes to the stack.
    let gp_base_addr = lv.program_counter - P::ONES;
    for i in 0..NUM_GP_CHANNELS {
        let channel = lv.mem_channels[i];
        yield_constr.constraint(lv.is_push_cont * (channel.is_read - P::ONES));
        yield_constr.constraint(lv.is_push_cont * (channel.addr_context - lv.code_context));
        yield_constr.constraint(
            lv.is_push_cont
                * (channel.addr_segment - P::Scalar::from_canonical_usize(Segment::Code as usize)),
        );

        let address = gp_base_addr - P::Scalar::from_canonical_usize(i);
        yield_constr.constraint(lv.is_push_cont * (channel.addr_virtual - address));
    }

    // If a channel is unused, then all the following channels must also be unused.
    for i in 1..NUM_GP_CHANNELS {
        let this_channel = lv.mem_channels[i];
        let previous_channel = lv.mem_channels[i - 1];

        // previous_channel.used = 0 => this_channel.used = 0
        // <=> previous_channel.used != 0 or this_channel.used = 0
        yield_constr
            .constraint(lv.is_push_cont * (previous_channel.used - P::ONES) * this_channel.used);
    }

    // If a channel is disabled, then its value is zero.
    for i in 0..NUM_GP_CHANNELS {
        let channel = lv.mem_channels[i];
        // NB: We do not need to constrain `channel.value` at indices other than 0 as those are
        // ignored.
        yield_constr.constraint(lv.is_push_cont * (channel.used - P::ONES) * channel.value[0]);
    }

    // Verify that the number of used channels corresponds to the number of bytes being pushed.
    let channels_used_num: P = lv
        .mem_channels
        .iter()
        .map(|channel| channel.used)
        .sum::<P>()
        + P::ONES;
    yield_constr.constraint_transition(
        lv.is_push_cont
            * (lv_push.rem_bytes - channels_used_num - nv.is_push_cont * nv_push.rem_bytes),
    );
    yield_constr.constraint_last_row(lv.is_push_cont * (lv_push.rem_bytes - channels_used_num));

    // If this PUSH is continued further, then we must have exhausted our memory channels
    yield_constr.constraint_transition(
        lv.is_push_cont * nv.is_push_cont * (lv.mem_channels[NUM_GP_CHANNELS - 1].used - P::ONES),
    );

    // If this PUSH is continued further, we constrain the program counter and `next_pc`.
    let next_row_pc = gp_base_addr - P::Scalar::from_canonical_usize(NUM_GP_CHANNELS);
    yield_constr.constraint_transition(
        lv.is_push_cont * nv.is_push_cont * (nv.program_counter - next_row_pc),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont * nv.is_push_cont * (nv_push.next_pc - lv_push.next_pc),
    );
    // Otherwise, we constrain the program counter.
    yield_constr.constraint_transition(
        lv.is_push_cont * (nv.is_push_cont - P::ONES) * (nv.program_counter - lv_push.next_pc),
    );

    // Finally, we constrain `higher_limbs`.
    yield_constr.constraint_transition(
        lv.is_push_cont
            * (lv_push.higher_limbs[0]
                - nv.is_push_cont
                    * join_bb_le(
                        join_bits_into_byte_le(nv.opcode_bits),
                        nv.mem_channels[0].value[0],
                    )),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont
            * (lv_push.higher_limbs[1]
                - nv.is_push_cont
                    * join_bb_le(nv.mem_channels[1].value[0], nv.mem_channels[2].value[0])),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont
            * (lv_push.higher_limbs[2]
                - nv.is_push_cont
                    * join_bb_le(nv.mem_channels[3].value[0], nv.mem_channels[4].value[0])),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont * (lv_push.higher_limbs[3] - nv.is_push_cont * nv_push.higher_limbs[0]),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont * (lv_push.higher_limbs[4] - nv.is_push_cont * nv_push.higher_limbs[1]),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont * (lv_push.higher_limbs[5] - nv.is_push_cont * nv_push.higher_limbs[2]),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont * (lv_push.higher_limbs[6] - nv.is_push_cont * nv_push.higher_limbs[3]),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont * (lv_push.higher_limbs[7] - nv.is_push_cont * nv_push.higher_limbs[4]),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont * (lv_push.higher_limbs[8] - nv.is_push_cont * nv_push.higher_limbs[5]),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont * (lv_push.higher_limbs[9] - nv.is_push_cont * nv_push.higher_limbs[6]),
    );
    yield_constr.constraint_transition(
        lv.is_push_cont * (lv_push.higher_limbs[10] - nv.is_push_cont * nv_push.higher_limbs[7]),
    );
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[0]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[1]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[2]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[3]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[4]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[5]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[6]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[7]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[8]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[9]);
    yield_constr.constraint_last_row(lv.is_push_cont * lv_push.higher_limbs[10]);
}
