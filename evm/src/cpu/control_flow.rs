use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP};
use crate::cpu::kernel::aggregator::KERNEL;

const NATIVE_INSTRUCTIONS: [usize; 28] = [
    COL_MAP.op.add,
    COL_MAP.op.mul,
    COL_MAP.op.sub,
    COL_MAP.op.div,
    COL_MAP.op.mod_,
    COL_MAP.op.addmod,
    COL_MAP.op.mulmod,
    COL_MAP.op.addfp254,
    COL_MAP.op.mulfp254,
    COL_MAP.op.subfp254,
    COL_MAP.op.lt,
    COL_MAP.op.gt,
    COL_MAP.op.eq_iszero,
    COL_MAP.op.logic_op,
    COL_MAP.op.not,
    COL_MAP.op.shl,
    COL_MAP.op.shr,
    COL_MAP.op.keccak_general,
    COL_MAP.op.prover_input,
    COL_MAP.op.pop,
    // not JUMP (need to jump)
    // not JUMPI (possible need to jump)
    COL_MAP.op.pc,
    COL_MAP.op.jumpdest,
    COL_MAP.op.push0,
    // not PUSH (need to increment by more than 1)
    COL_MAP.op.dup,
    COL_MAP.op.swap,
    COL_MAP.op.context_op,
    // not EXIT_KERNEL (performs a jump)
    COL_MAP.op.mload_general,
    COL_MAP.op.mstore_general,
    // not SYSCALL (performs a jump)
    // not exceptions (also jump)
];

pub(crate) fn get_halt_pc<F: Field>() -> F {
    let halt_pc = KERNEL.global_labels["halt"];
    F::from_canonical_usize(halt_pc)
}

pub(crate) fn get_start_pc<F: Field>() -> F {
    let start_pc = KERNEL.global_labels["main"];

    F::from_canonical_usize(start_pc)
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // The null special instruction must be boolean.
    yield_constr.constraint(lv.null * (lv.null - P::ONES));
    // Once we reach a null row, there must be only null rows.
    yield_constr.constraint_transition(lv.null * (nv.null - P::ONES));

    let is_cpu_cycle: P = COL_MAP.op.iter().map(|&col_i| lv[col_i]).sum();
    let is_cpu_cycle_next: P = COL_MAP.op.iter().map(|&col_i| nv[col_i]).sum();
    // Once we start executing instructions, then we continue until the end of the table
    // or we reach dummy padding rows.
    yield_constr
        .constraint_transition(is_cpu_cycle * (is_cpu_cycle_next - P::ONES) * (nv.null - P::ONES));

    // If a row is a CPU cycle and executing a native instruction (implemented as a table row; not
    // microcoded) then the program counter is incremented by 1 to obtain the next row's program
    // counter. Also, the next row has the same kernel flag.
    let is_native_instruction: P = NATIVE_INSTRUCTIONS.iter().map(|&col_i| lv[col_i]).sum();
    yield_constr.constraint_transition(
        (nv.null - P::ONES)
            * is_native_instruction
            * (lv.program_counter - nv.program_counter + P::ONES),
    );
    yield_constr.constraint_transition(
        (nv.null - P::ONES) * is_native_instruction * (lv.is_kernel_mode - nv.is_kernel_mode),
    );

    // If a non-CPU cycle row is followed by a CPU cycle row, then:
    //  - the `program_counter` of the CPU cycle row is `main` (the entry point of our kernel),
    //  - execution is in kernel mode, and
    //  - the stack is empty.
    let is_last_noncpu_cycle = (is_cpu_cycle - P::ONES) * is_cpu_cycle_next;
    let pc_diff = nv.program_counter - get_start_pc::<P::Scalar>();
    yield_constr.constraint_transition(is_last_noncpu_cycle * pc_diff);
    yield_constr.constraint_transition(is_last_noncpu_cycle * (nv.is_kernel_mode - P::ONES));
    yield_constr.constraint_transition(is_last_noncpu_cycle * nv.stack_len);

    // The last row must be a CPU cycle row or a dummy padding row.
    yield_constr.constraint_last_row((is_cpu_cycle - P::ONES) * (lv.null - P::ONES));
    // Also, the last row's `program_counter` must be inside the `halt` infinite loop.
    let halt_pc = get_halt_pc::<P::Scalar>();
    yield_constr.constraint_last_row(lv.program_counter - halt_pc);
    // Finally, the last row must be in kernel mode.
    yield_constr.constraint_last_row(lv.is_kernel_mode - P::ONES);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // The null special instruction must be boolean.
    let constr = builder.mul_sub_extension(lv.null, lv.null, lv.null);
    yield_constr.constraint(builder, constr);
    // Once we reach a null row, there must be only null rows.
    let constr = builder.mul_sub_extension(lv.null, nv.null, lv.null);
    yield_constr.constraint_transition(builder, constr);

    let is_cpu_cycle = builder.add_many_extension(COL_MAP.op.iter().map(|&col_i| lv[col_i]));
    let is_cpu_cycle_next = builder.add_many_extension(COL_MAP.op.iter().map(|&col_i| nv[col_i]));
    // Once we start executing instructions, then we continue until the end of the table
    // or we reach dummy padding rows.
    {
        let constr = builder.mul_sub_extension(is_cpu_cycle, is_cpu_cycle_next, is_cpu_cycle);
        let constr = builder.mul_sub_extension(constr, nv.null, constr);
        yield_constr.constraint_transition(builder, constr);
    }

    // If a row is a CPU cycle and executing a native instruction (implemented as a table row; not
    // microcoded) then the program counter is incremented by 1 to obtain the next row's program
    // counter. Also, the next row has the same kernel flag.
    {
        let filter = builder.add_many_extension(NATIVE_INSTRUCTIONS.iter().map(|&col_i| lv[col_i]));
        let pc_diff = builder.sub_extension(lv.program_counter, nv.program_counter);
        let pc_constr = builder.mul_add_extension(filter, pc_diff, filter);
        let pc_constr = builder.mul_sub_extension(pc_constr, nv.null, pc_constr);
        yield_constr.constraint_transition(builder, pc_constr);
        let kernel_diff = builder.sub_extension(lv.is_kernel_mode, nv.is_kernel_mode);
        let kernel_constr = builder.mul_extension(filter, kernel_diff);
        let kernel_constr = builder.mul_sub_extension(kernel_constr, nv.null, kernel_constr);
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

    // The last row must be a CPU cycle row or a dummy padding row.
    {
        let one = builder.one_extension();
        let constr = builder.sub_extension(is_cpu_cycle, one);
        let constr = builder.mul_sub_extension(constr, lv.null, constr);
        yield_constr.constraint_last_row(builder, constr);
    }
    // Also, the last row's `program_counter` must be inside the `halt` infinite loop.
    {
        let halt_pc = get_halt_pc();
        let halt_pc_target = builder.constant_extension(halt_pc);
        let constr = builder.sub_extension(lv.program_counter, halt_pc_target);

        yield_constr.constraint_last_row(builder, constr);
    }
    // Finally, the last row must be in kernel mode.
    {
        let one = builder.one_extension();
        let constr = builder.sub_extension(lv.is_kernel_mode, one);
        yield_constr.constraint_last_row(builder, constr);
    }
}
