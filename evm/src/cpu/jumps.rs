use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

pub fn eval_packed_exit_kernel<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let input = lv.mem_channels[0].value;
    let filter = lv.is_cpu_cycle * lv.op.exit_kernel;

    // If we are executing `EXIT_KERNEL` then we simply restore the program counter, kernel mode
    // flag, and gas counter. The middle 4 (32-bit) limbs are ignored (this is not part of the spec,
    // but we trust the kernel to set them to zero).
    yield_constr.constraint_transition(filter * (input[0] - nv.program_counter));
    yield_constr.constraint_transition(filter * (input[1] - nv.is_kernel_mode));
    yield_constr.constraint_transition(filter * (input[6] - nv.gas));
    // High limb of gas must be 0 for convenient detection of overflow.
    yield_constr.constraint(filter * input[7]);
}

pub fn eval_ext_circuit_exit_kernel<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let input = lv.mem_channels[0].value;
    let filter = builder.mul_extension(lv.is_cpu_cycle, lv.op.exit_kernel);

    // If we are executing `EXIT_KERNEL` then we simply restore the program counter and kernel mode
    // flag. The top 6 (32-bit) limbs are ignored (this is not part of the spec, but we trust the
    // kernel to set them to zero).

    let pc_constr = builder.sub_extension(input[0], nv.program_counter);
    let pc_constr = builder.mul_extension(filter, pc_constr);
    yield_constr.constraint_transition(builder, pc_constr);

    let kernel_constr = builder.sub_extension(input[1], nv.is_kernel_mode);
    let kernel_constr = builder.mul_extension(filter, kernel_constr);
    yield_constr.constraint_transition(builder, kernel_constr);

    {
        let diff = builder.sub_extension(input[6], nv.gas);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint_transition(builder, constr);
    }
    {
        // High limb of gas must be 0 for convenient detection of overflow.
        let constr = builder.mul_extension(filter, input[7]);
        yield_constr.constraint(builder, constr);
    }
}

pub fn eval_packed_jump_jumpi<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let jumps_lv = lv.general.jumps();
    let dst = lv.mem_channels[0].value;
    let cond = lv.mem_channels[1].value;
    let filter = lv.op.jump + lv.op.jumpi; // `JUMP` or `JUMPI`
    let jumpdest_flag_channel = lv.mem_channels[NUM_GP_CHANNELS - 1];

    // If `JUMP`, re-use the `JUMPI` logic, but setting the second input (the predicate) to be 1.
    // In other words, we implement `JUMP(dst)` as `JUMPI(dst, cond=1)`.
    yield_constr.constraint(lv.op.jump * (cond[0] - P::ONES));
    for &limb in &cond[1..] {
        // Set all limbs (other than the least-significant limb) to 0.
        // NB: Technically, they don't have to be 0, as long as the sum
        // `cond[0] + ... + cond[7]` cannot overflow.
        yield_constr.constraint(lv.op.jump * limb);
    }

    // Check `should_jump`:
    yield_constr.constraint(filter * jumps_lv.should_jump * (jumps_lv.should_jump - P::ONES));
    let cond_sum: P = cond.into_iter().sum();
    yield_constr.constraint(filter * (jumps_lv.should_jump - P::ONES) * cond_sum);
    yield_constr.constraint(filter * (jumps_lv.cond_sum_pinv * cond_sum - jumps_lv.should_jump));

    // If we're jumping, then the high 7 limbs of the destination must be 0.
    let dst_hi_sum: P = dst[1..].iter().copied().sum();
    yield_constr.constraint(filter * jumps_lv.should_jump * dst_hi_sum);
    // Check that the destination address holds a `JUMPDEST` instruction. Note that this constraint
    // does not need to be conditioned on `should_jump` because no read takes place if we're not
    // jumping, so we're free to set the channel to 1.
    yield_constr.constraint(filter * (jumpdest_flag_channel.value[0] - P::ONES));

    // Make sure that the JUMPDEST flag channel is constrained.
    // Only need to read if we're about to jump and we're not in kernel mode.
    yield_constr.constraint(
        filter
            * (jumpdest_flag_channel.used - jumps_lv.should_jump * (P::ONES - lv.is_kernel_mode)),
    );
    yield_constr.constraint(filter * (jumpdest_flag_channel.is_read - P::ONES));
    yield_constr.constraint(filter * (jumpdest_flag_channel.addr_context - lv.context));
    yield_constr.constraint(
        filter
            * (jumpdest_flag_channel.addr_segment
                - P::Scalar::from_canonical_u64(Segment::JumpdestBits as u64)),
    );
    yield_constr.constraint(filter * (jumpdest_flag_channel.addr_virtual - dst[0]));

    // Disable unused memory channels
    for &channel in &lv.mem_channels[2..NUM_GP_CHANNELS - 1] {
        yield_constr.constraint(filter * channel.used);
    }
    // Channel 1 is unused by the `JUMP` instruction.
    yield_constr.constraint(lv.op.jump * lv.mem_channels[1].used);

    // Finally, set the next program counter.
    let fallthrough_dst = lv.program_counter + P::ONES;
    let jump_dest = dst[0];
    yield_constr.constraint_transition(
        filter * (jumps_lv.should_jump - P::ONES) * (nv.program_counter - fallthrough_dst),
    );
    yield_constr
        .constraint_transition(filter * jumps_lv.should_jump * (nv.program_counter - jump_dest));
}

pub fn eval_ext_circuit_jump_jumpi<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let jumps_lv = lv.general.jumps();
    let dst = lv.mem_channels[0].value;
    let cond = lv.mem_channels[1].value;
    let filter = builder.add_extension(lv.op.jump, lv.op.jumpi); // `JUMP` or `JUMPI`
    let jumpdest_flag_channel = lv.mem_channels[NUM_GP_CHANNELS - 1];

    // If `JUMP`, re-use the `JUMPI` logic, but setting the second input (the predicate) to be 1.
    // In other words, we implement `JUMP(dst)` as `JUMPI(dst, cond=1)`.
    {
        let constr = builder.mul_sub_extension(lv.op.jump, cond[0], lv.op.jump);
        yield_constr.constraint(builder, constr);
    }
    for &limb in &cond[1..] {
        // Set all limbs (other than the least-significant limb) to 0.
        // NB: Technically, they don't have to be 0, as long as the sum
        // `cond[0] + ... + cond[7]` cannot overflow.
        let constr = builder.mul_extension(lv.op.jump, limb);
        yield_constr.constraint(builder, constr);
    }

    // Check `should_jump`:
    {
        let constr = builder.mul_sub_extension(
            jumps_lv.should_jump,
            jumps_lv.should_jump,
            jumps_lv.should_jump,
        );
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    let cond_sum = builder.add_many_extension(cond);
    {
        let constr = builder.mul_sub_extension(cond_sum, jumps_lv.should_jump, cond_sum);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr =
            builder.mul_sub_extension(jumps_lv.cond_sum_pinv, cond_sum, jumps_lv.should_jump);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // If we're jumping, then the high 7 limbs of the destination must be 0.
    let dst_hi_sum = builder.add_many_extension(&dst[1..]);
    {
        let constr = builder.mul_extension(jumps_lv.should_jump, dst_hi_sum);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    // Check that the destination address holds a `JUMPDEST` instruction. Note that this constraint
    // does not need to be conditioned on `should_jump` because no read takes place if we're not
    // jumping, so we're free to set the channel to 1.
    {
        let constr = builder.mul_sub_extension(filter, jumpdest_flag_channel.value[0], filter);
        yield_constr.constraint(builder, constr);
    }

    // Make sure that the JUMPDEST flag channel is constrained.
    // Only need to read if we're about to jump and we're not in kernel mode.
    {
        let constr = builder.mul_sub_extension(
            jumps_lv.should_jump,
            lv.is_kernel_mode,
            jumps_lv.should_jump,
        );
        let constr = builder.add_extension(jumpdest_flag_channel.used, constr);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_sub_extension(filter, jumpdest_flag_channel.is_read, filter);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.sub_extension(jumpdest_flag_channel.addr_context, lv.context);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.arithmetic_extension(
            F::ONE,
            -F::from_canonical_u64(Segment::JumpdestBits as u64),
            filter,
            jumpdest_flag_channel.addr_segment,
            filter,
        );
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.sub_extension(jumpdest_flag_channel.addr_virtual, dst[0]);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // Disable unused memory channels
    for &channel in &lv.mem_channels[2..NUM_GP_CHANNELS - 1] {
        let constr = builder.mul_extension(filter, channel.used);
        yield_constr.constraint(builder, constr);
    }
    // Channel 1 is unused by the `JUMP` instruction.
    {
        let constr = builder.mul_extension(lv.op.jump, lv.mem_channels[1].used);
        yield_constr.constraint(builder, constr);
    }

    // Finally, set the next program counter.
    let fallthrough_dst = builder.add_const_extension(lv.program_counter, F::ONE);
    let jump_dest = dst[0];
    {
        let constr_a = builder.mul_sub_extension(filter, jumps_lv.should_jump, filter);
        let constr_b = builder.sub_extension(nv.program_counter, fallthrough_dst);
        let constr = builder.mul_extension(constr_a, constr_b);
        yield_constr.constraint_transition(builder, constr);
    }
    {
        let constr_a = builder.mul_extension(filter, jumps_lv.should_jump);
        let constr_b = builder.sub_extension(nv.program_counter, jump_dest);
        let constr = builder.mul_extension(constr_a, constr_b);
        yield_constr.constraint_transition(builder, constr);
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_exit_kernel(lv, nv, yield_constr);
    eval_packed_jump_jumpi(lv, nv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_exit_kernel(builder, lv, nv, yield_constr);
    eval_ext_circuit_jump_jumpi(builder, lv, nv, yield_constr);
}
