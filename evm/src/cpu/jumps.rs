use once_cell::sync::Lazy;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::aggregator::KERNEL;

static INVALID_DST_HANDLER_ADDR: Lazy<usize> =
    Lazy::new(|| KERNEL.global_labels["fault_exception"]);

pub fn eval_packed_exit_kernel<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let input = lv.mem_channels[0].value;

    // If we are executing `EXIT_KERNEL` then we simply restore the program counter and kernel mode
    // flag. The top 6 (32-bit) limbs are ignored (this is not part of the spec, but we trust the
    // kernel to set them to zero).
    yield_constr.constraint_transition(
        lv.is_cpu_cycle * lv.is_exit_kernel * (input[0] - nv.program_counter),
    );
    yield_constr.constraint_transition(
        lv.is_cpu_cycle * lv.is_exit_kernel * (input[1] - nv.is_kernel_mode),
    );
}

pub fn eval_ext_circuit_exit_kernel<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let input = lv.mem_channels[0].value;
    let filter = builder.mul_extension(lv.is_cpu_cycle, lv.is_exit_kernel);

    // If we are executing `EXIT_KERNEL` then we simply restore the program counter and kernel mode
    // flag. The top 6 (32-bit) limbs are ignored (this is not part of the spec, but we trust the
    // kernel to set them to zero).

    let pc_constr = builder.sub_extension(input[0], nv.program_counter);
    let pc_constr = builder.mul_extension(filter, pc_constr);
    yield_constr.constraint_transition(builder, pc_constr);

    let kernel_constr = builder.sub_extension(input[1], nv.is_kernel_mode);
    let kernel_constr = builder.mul_extension(filter, kernel_constr);
    yield_constr.constraint_transition(builder, kernel_constr);
}

pub fn eval_packed_jump_jumpi<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let jumps_lv = lv.general.jumps();
    let input0 = lv.mem_channels[0].value;
    let input1 = lv.mem_channels[1].value;
    let filter = lv.is_jump + lv.is_jumpi; // `JUMP` or `JUMPI`

    // If `JUMP`, re-use the `JUMPI` logic, but setting the second input (the predicate) to be 1.
    // In other words, we implement `JUMP(addr)` as `JUMPI(addr, cond=1)`.
    yield_constr.constraint(lv.is_jump * (input1[0] - P::ONES));
    for &limb in &input1[1..] {
        // Set all limbs (other than the least-significant limb) to 0.
        // NB: Technically, they don't have to be 0, as long as the sum
        // `input1[0] + ... + input1[7]` cannot overflow.
        yield_constr.constraint(lv.is_jump * limb);
    }

    // Check `input0_upper_zero`
    // `input0_upper_zero` is either 0 or 1.
    yield_constr
        .constraint(filter * jumps_lv.input0_upper_zero * (jumps_lv.input0_upper_zero - P::ONES));
    // The below sum cannot overflow due to the limb size.
    let input0_upper_sum: P = input0[1..].iter().copied().sum();
    // `input0_upper_zero` = 1 implies `input0_upper_sum` = 0.
    yield_constr.constraint(filter * jumps_lv.input0_upper_zero * input0_upper_sum);
    // `input0_upper_zero` = 0 implies `input0_upper_sum_inv * input0_upper_sum` = 1, which can only
    // happen when `input0_upper_sum` is nonzero.
    yield_constr.constraint(
        filter
            * (jumps_lv.input0_upper_sum_inv * input0_upper_sum + jumps_lv.input0_upper_zero
                - P::ONES),
    );

    // Check `dst_valid_or_kernel` (this is just a logical OR)
    yield_constr.constraint(
        filter
            * (jumps_lv.dst_valid + lv.is_kernel_mode
                - jumps_lv.dst_valid * lv.is_kernel_mode
                - jumps_lv.dst_valid_or_kernel),
    );

    // Check `input0_jumpable` (this is just `dst_valid_or_kernel` AND `input0_upper_zero`)
    yield_constr.constraint(
        filter
            * (jumps_lv.dst_valid_or_kernel * jumps_lv.input0_upper_zero
                - jumps_lv.input0_jumpable),
    );

    // Make sure that `should_continue`, `should_jump`, `should_trap` are all binary and exactly one
    // is set.
    yield_constr
        .constraint(filter * jumps_lv.should_continue * (jumps_lv.should_continue - P::ONES));
    yield_constr.constraint(filter * jumps_lv.should_jump * (jumps_lv.should_jump - P::ONES));
    yield_constr.constraint(filter * jumps_lv.should_trap * (jumps_lv.should_trap - P::ONES));
    yield_constr.constraint(
        filter * (jumps_lv.should_continue + jumps_lv.should_jump + jumps_lv.should_trap - P::ONES),
    );

    // Validate `should_continue`
    // This sum cannot overflow (due to limb size).
    let input1_sum: P = input1.into_iter().sum();
    // `should_continue` = 1 implies `input1_sum` = 0.
    yield_constr.constraint(filter * jumps_lv.should_continue * input1_sum);
    // `should_continue` = 0 implies `input1_sum * input1_sum_inv` = 1, which can only happen if
    // input1_sum is nonzero.
    yield_constr.constraint(
        filter * (input1_sum * jumps_lv.input1_sum_inv + jumps_lv.should_continue - P::ONES),
    );

    // Validate `should_jump` and `should_trap` by splitting on `input0_jumpable`.
    // Note that `should_jump` = 1 and `should_trap` = 1 both imply that `should_continue` = 0, so
    // `input1` is nonzero.
    yield_constr.constraint(filter * jumps_lv.should_jump * (jumps_lv.input0_jumpable - P::ONES));
    yield_constr.constraint(filter * jumps_lv.should_trap * jumps_lv.input0_jumpable);

    // Handle trap
    // Set program counter and kernel flag
    yield_constr
        .constraint_transition(filter * jumps_lv.should_trap * (nv.is_kernel_mode - P::ONES));
    yield_constr.constraint_transition(
        filter
            * jumps_lv.should_trap
            * (nv.program_counter - P::Scalar::from_canonical_usize(*INVALID_DST_HANDLER_ADDR)),
    );

    // Handle continue and jump
    let continue_or_jump = jumps_lv.should_continue + jumps_lv.should_jump;
    // Keep kernel mode.
    yield_constr
        .constraint_transition(filter * continue_or_jump * (nv.is_kernel_mode - lv.is_kernel_mode));
    // Set program counter depending on whether we're continuing or jumping.
    yield_constr.constraint_transition(
        filter * jumps_lv.should_continue * (nv.program_counter - lv.program_counter - P::ONES),
    );
    yield_constr
        .constraint_transition(filter * jumps_lv.should_jump * (nv.program_counter - input0[0]));
}

pub fn eval_ext_circuit_jump_jumpi<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let jumps_lv = lv.general.jumps();
    let input0 = lv.mem_channels[0].value;
    let input1 = lv.mem_channels[1].value;
    let filter = builder.add_extension(lv.is_jump, lv.is_jumpi); // `JUMP` or `JUMPI`

    // If `JUMP`, re-use the `JUMPI` logic, but setting the second input (the predicate) to be 1.
    // In other words, we implement `JUMP(addr)` as `JUMPI(addr, cond=1)`.
    {
        let constr = builder.mul_sub_extension(lv.is_jump, input1[0], lv.is_jump);
        yield_constr.constraint(builder, constr);
    }
    for &limb in &input1[1..] {
        // Set all limbs (other than the least-significant limb) to 0.
        // NB: Technically, they don't have to be 0, as long as the sum
        // `input1[0] + ... + input1[7]` cannot overflow.
        let constr = builder.mul_extension(lv.is_jump, limb);
        yield_constr.constraint(builder, constr);
    }

    // Check `input0_upper_zero`
    // `input0_upper_zero` is either 0 or 1.
    {
        let constr = builder.mul_sub_extension(
            jumps_lv.input0_upper_zero,
            jumps_lv.input0_upper_zero,
            jumps_lv.input0_upper_zero,
        );
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    {
        // The below sum cannot overflow due to the limb size.
        let input0_upper_sum = builder.add_many_extension(input0[1..].iter());

        // `input0_upper_zero` = 1 implies `input0_upper_sum` = 0.
        let constr = builder.mul_extension(jumps_lv.input0_upper_zero, input0_upper_sum);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);

        // `input0_upper_zero` = 0 implies `input0_upper_sum_inv * input0_upper_sum` = 1, which can
        // only happen when `input0_upper_sum` is nonzero.
        let constr = builder.mul_add_extension(
            jumps_lv.input0_upper_sum_inv,
            input0_upper_sum,
            jumps_lv.input0_upper_zero,
        );
        let constr = builder.mul_sub_extension(filter, constr, filter);
        yield_constr.constraint(builder, constr);
    };

    // Check `dst_valid_or_kernel` (this is just a logical OR)
    {
        let constr = builder.mul_add_extension(
            jumps_lv.dst_valid,
            lv.is_kernel_mode,
            jumps_lv.dst_valid_or_kernel,
        );
        let constr = builder.sub_extension(jumps_lv.dst_valid, constr);
        let constr = builder.add_extension(lv.is_kernel_mode, constr);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // Check `input0_jumpable` (this is just `dst_valid_or_kernel` AND `input0_upper_zero`)
    {
        let constr = builder.mul_sub_extension(
            jumps_lv.dst_valid_or_kernel,
            jumps_lv.input0_upper_zero,
            jumps_lv.input0_jumpable,
        );
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // Make sure that `should_continue`, `should_jump`, `should_trap` are all binary and exactly one
    // is set.
    for flag in [
        jumps_lv.should_continue,
        jumps_lv.should_jump,
        jumps_lv.should_trap,
    ] {
        let constr = builder.mul_sub_extension(flag, flag, flag);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.add_extension(jumps_lv.should_continue, jumps_lv.should_jump);
        let constr = builder.add_extension(constr, jumps_lv.should_trap);
        let constr = builder.mul_sub_extension(filter, constr, filter);
        yield_constr.constraint(builder, constr);
    }

    // Validate `should_continue`
    {
        // This sum cannot overflow (due to limb size).
        let input1_sum = builder.add_many_extension(input1.into_iter());

        // `should_continue` = 1 implies `input1_sum` = 0.
        let constr = builder.mul_extension(jumps_lv.should_continue, input1_sum);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);

        // `should_continue` = 0 implies `input1_sum * input1_sum_inv` = 1, which can only happen if
        // input1_sum is nonzero.
        let constr = builder.mul_add_extension(
            input1_sum,
            jumps_lv.input1_sum_inv,
            jumps_lv.should_continue,
        );
        let constr = builder.mul_sub_extension(filter, constr, filter);
        yield_constr.constraint(builder, constr);
    }

    // Validate `should_jump` and `should_trap` by splitting on `input0_jumpable`.
    // Note that `should_jump` = 1 and `should_trap` = 1 both imply that `should_continue` = 0, so
    // `input1` is nonzero.
    {
        let constr = builder.mul_sub_extension(
            jumps_lv.should_jump,
            jumps_lv.input0_jumpable,
            jumps_lv.should_jump,
        );
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    {
        let constr = builder.mul_extension(jumps_lv.should_trap, jumps_lv.input0_jumpable);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // Handle trap
    {
        let trap_filter = builder.mul_extension(filter, jumps_lv.should_trap);

        // Set kernel flag
        let constr = builder.mul_sub_extension(trap_filter, nv.is_kernel_mode, trap_filter);
        yield_constr.constraint_transition(builder, constr);

        // Set program counter
        let constr = builder.arithmetic_extension(
            F::ONE,
            -F::from_canonical_usize(*INVALID_DST_HANDLER_ADDR),
            trap_filter,
            nv.program_counter,
            trap_filter,
        );
        yield_constr.constraint_transition(builder, constr);
    }

    // Handle continue and jump
    {
        // Keep kernel mode.
        let continue_or_jump =
            builder.add_extension(jumps_lv.should_continue, jumps_lv.should_jump);
        let constr = builder.sub_extension(nv.is_kernel_mode, lv.is_kernel_mode);
        let constr = builder.mul_extension(continue_or_jump, constr);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint_transition(builder, constr);
    }
    // Set program counter depending on whether we're continuing...
    {
        let constr = builder.sub_extension(nv.program_counter, lv.program_counter);
        let constr =
            builder.mul_sub_extension(jumps_lv.should_continue, constr, jumps_lv.should_continue);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint_transition(builder, constr);
    }
    // ...or jumping.
    {
        let constr = builder.sub_extension(nv.program_counter, input0[0]);
        let constr = builder.mul_extension(jumps_lv.should_jump, constr);
        let constr = builder.mul_extension(filter, constr);
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
