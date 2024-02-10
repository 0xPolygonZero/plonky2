use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP};
use crate::cpu::kernel::aggregator::KERNEL;

const NATIVE_INSTRUCTIONS: [usize; 12] = [
    COL_MAP.op.binary_op,
    COL_MAP.op.ternary_op,
    COL_MAP.op.fp254_op,
    COL_MAP.op.eq_iszero,
    COL_MAP.op.logic_op,
    COL_MAP.op.not_pop,
    COL_MAP.op.shift,
    COL_MAP.op.jumpdest_keccak_general,
    // Not PROVER_INPUT: it is dealt with manually below.
    // not JUMPS (possible need to jump)
    COL_MAP.op.pc_push0,
    // not PUSH (need to increment by more than 1)
    COL_MAP.op.dup_swap,
    COL_MAP.op.context_op,
    // not EXIT_KERNEL (performs a jump)
    COL_MAP.op.m_op_general,
    // not SYSCALL (performs a jump)
    // not exceptions (also jump)
];

/// Returns `halt`'s program counter.
pub(crate) fn get_halt_pc<F: Field>() -> F {
    let halt_pc = KERNEL.global_labels["halt"];
    F::from_canonical_usize(halt_pc)
}

/// Returns `main`'s program counter.
pub(crate) fn get_start_pc<F: Field>() -> F {
    let start_pc = KERNEL.global_labels["main"];

    F::from_canonical_usize(start_pc)
}

/// Evaluates the constraints related to the flow of instructions.
pub(crate) fn eval_packed_generic<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_cpu_cycle: P = COL_MAP.op.iter().map(|&col_i| lv[col_i]).sum();
    let is_cpu_cycle_next: P = COL_MAP.op.iter().map(|&col_i| nv[col_i]).sum();

    let next_halt_state = P::ONES - is_cpu_cycle_next;

    // Once we start executing instructions, then we continue until the end of the table
    // or we reach dummy padding rows. This, along with the constraints on the first row,
    // enforces that operation flags and the halt flag are mutually exclusive over the entire
    // CPU trace.
    yield_constr
        .constraint_transition(is_cpu_cycle * (is_cpu_cycle_next + next_halt_state - P::ONES));

    // If a row is a CPU cycle and executing a native instruction (implemented as a table row; not
    // microcoded) then the program counter is incremented by 1 to obtain the next row's program
    // counter. Also, the next row has the same kernel flag.
    let is_native_instruction: P = NATIVE_INSTRUCTIONS.iter().map(|&col_i| lv[col_i]).sum();
    yield_constr.constraint_transition(
        is_native_instruction * (lv.program_counter - nv.program_counter + P::ONES),
    );
    yield_constr
        .constraint_transition(is_native_instruction * (lv.is_kernel_mode - nv.is_kernel_mode));

    // Apply the same checks as before, for PROVER_INPUT.
    let is_prover_input: P = lv.op.push_prover_input * (lv.opcode_bits[5] - P::ONES);
    yield_constr.constraint_transition(
        is_prover_input * (lv.program_counter - nv.program_counter + P::ONES),
    );
    yield_constr.constraint_transition(is_prover_input * (lv.is_kernel_mode - nv.is_kernel_mode));

    // If a non-CPU cycle row is followed by a CPU cycle row, then:
    //  - the `program_counter` of the CPU cycle row is `main` (the entry point of our kernel),
    //  - execution is in kernel mode, and
    //  - the stack is empty.
    let is_last_noncpu_cycle = (is_cpu_cycle - P::ONES) * is_cpu_cycle_next;
    let pc_diff = nv.program_counter - get_start_pc::<P::Scalar>();
    yield_constr.constraint_transition(is_last_noncpu_cycle * pc_diff);
    yield_constr.constraint_transition(is_last_noncpu_cycle * (nv.is_kernel_mode - P::ONES));
    yield_constr.constraint_transition(is_last_noncpu_cycle * nv.stack_len);
}

/// Circuit version of `eval_packed`.
/// Evaluates the constraints related to the flow of instructions.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();

    let is_cpu_cycle = builder.add_many_extension(COL_MAP.op.iter().map(|&col_i| lv[col_i]));
    let is_cpu_cycle_next = builder.add_many_extension(COL_MAP.op.iter().map(|&col_i| nv[col_i]));

    let next_halt_state = builder.sub_extension(one, is_cpu_cycle_next);

    // Once we start executing instructions, then we continue until the end of the table
    // or we reach dummy padding rows. This, along with the constraints on the first row,
    // enforces that operation flags and the halt flag are mutually exclusive over the entire
    // CPU trace.
    {
        let constr = builder.add_extension(is_cpu_cycle_next, next_halt_state);
        let constr = builder.mul_sub_extension(is_cpu_cycle, constr, is_cpu_cycle);
        yield_constr.constraint_transition(builder, constr);
    }

    // If a row is a CPU cycle and executing a native instruction (implemented as a table row; not
    // microcoded) then the program counter is incremented by 1 to obtain the next row's program
    // counter. Also, the next row has the same kernel flag.
    {
        let filter = builder.add_many_extension(NATIVE_INSTRUCTIONS.iter().map(|&col_i| lv[col_i]));
        let pc_diff = builder.sub_extension(lv.program_counter, nv.program_counter);
        let pc_constr = builder.mul_add_extension(filter, pc_diff, filter);
        yield_constr.constraint_transition(builder, pc_constr);
        let kernel_diff = builder.sub_extension(lv.is_kernel_mode, nv.is_kernel_mode);
        let kernel_constr = builder.mul_extension(filter, kernel_diff);
        yield_constr.constraint_transition(builder, kernel_constr);

        // Same constraints as before, for PROVER_INPUT.
        let is_prover_input = builder.mul_sub_extension(
            lv.op.push_prover_input,
            lv.opcode_bits[5],
            lv.op.push_prover_input,
        );
        let pc_constr = builder.mul_add_extension(is_prover_input, pc_diff, is_prover_input);
        yield_constr.constraint_transition(builder, pc_constr);
        let kernel_constr = builder.mul_extension(is_prover_input, kernel_diff);
        yield_constr.constraint_transition(builder, kernel_constr);
    }

    // If a non-CPU cycle row is followed by a CPU cycle row, then:
    //  - the `program_counter` of the CPU cycle row is `main` (the entry point of our kernel),
    //  - execution is in kernel mode, and
    //  - the stack is empty.
    {
        let is_last_noncpu_cycle =
            builder.mul_sub_extension(is_cpu_cycle, is_cpu_cycle_next, is_cpu_cycle_next);

        // Start at `main`.
        let main = builder.constant_extension(get_start_pc::<F>().into());
        let pc_diff = builder.sub_extension(nv.program_counter, main);
        let pc_constr = builder.mul_extension(is_last_noncpu_cycle, pc_diff);
        yield_constr.constraint_transition(builder, pc_constr);

        // Start in kernel mode
        let kernel_constr = builder.mul_sub_extension(
            is_last_noncpu_cycle,
            nv.is_kernel_mode,
            is_last_noncpu_cycle,
        );
        yield_constr.constraint_transition(builder, kernel_constr);

        // Start with empty stack
        let kernel_constr = builder.mul_extension(is_last_noncpu_cycle, nv.stack_len);
        yield_constr.constraint_transition(builder, kernel_constr);
    }
}
