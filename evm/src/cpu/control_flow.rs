use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP};
use crate::cpu::kernel::aggregator::KERNEL;

// TODO: This list is incomplete.
const NATIVE_INSTRUCTIONS: [usize; 25] = [
    COL_MAP.is_add,
    COL_MAP.is_mul,
    COL_MAP.is_sub,
    COL_MAP.is_div,
    COL_MAP.is_sdiv,
    COL_MAP.is_mod,
    COL_MAP.is_smod,
    COL_MAP.is_addmod,
    COL_MAP.is_mulmod,
    COL_MAP.is_signextend,
    COL_MAP.is_lt,
    COL_MAP.is_gt,
    COL_MAP.is_slt,
    COL_MAP.is_sgt,
    COL_MAP.is_eq,
    COL_MAP.is_iszero,
    COL_MAP.is_and,
    COL_MAP.is_or,
    COL_MAP.is_xor,
    COL_MAP.is_not,
    COL_MAP.is_byte,
    COL_MAP.is_shl,
    COL_MAP.is_shr,
    COL_MAP.is_sar,
    COL_MAP.is_pop,
];

fn get_halt_pcs<F: Field>() -> (F, F) {
    let halt_pc0 = KERNEL.global_labels["halt_pc0"];
    let halt_pc1 = KERNEL.global_labels["halt_pc1"];

    (
        F::from_canonical_usize(halt_pc0),
        F::from_canonical_usize(halt_pc1),
    )
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // Once we start executing instructions, then we continue until the end of the table.
    yield_constr.constraint_transition(lv.is_cpu_cycle * (nv.is_cpu_cycle - P::ONES));

    // If a row is a CPU cycle and executing a native instruction (implemented as a table row; not
    // microcoded) then the program counter is incremented by 1 to obtain the next row's program
    // counter. Also, the next row has the same kernel flag.
    let is_native_instruction: P = NATIVE_INSTRUCTIONS.iter().map(|&col_i| lv[col_i]).sum();
    yield_constr.constraint_transition(
        lv.is_cpu_cycle
            * is_native_instruction
            * (lv.program_counter - nv.program_counter + P::ONES),
    );
    yield_constr.constraint_transition(
        lv.is_cpu_cycle * is_native_instruction * (lv.is_kernel_mode - nv.is_kernel_mode),
    );

    // If a non-CPU cycle row is followed by a CPU cycle row, then:
    //  - the `program_counter` of the CPU cycle row is `route_txn` (the entry point of our kernel),
    //  - execution is in kernel mode, and
    //  - the stack is empty.
    let is_last_noncpu_cycle = (lv.is_cpu_cycle - P::ONES) * nv.is_cpu_cycle;
    let pc_diff =
        nv.program_counter - P::Scalar::from_canonical_usize(KERNEL.global_labels["route_txn"]);
    yield_constr.constraint_transition(is_last_noncpu_cycle * pc_diff);
    yield_constr.constraint_transition(is_last_noncpu_cycle * (nv.is_kernel_mode - P::ONES));
    yield_constr.constraint_transition(is_last_noncpu_cycle * nv.stack_len);

    // The last row must be a CPU cycle row.
    yield_constr.constraint_last_row(lv.is_cpu_cycle - P::ONES);
    // Also, the last row's `program_counter` must be inside the `halt` infinite loop. Note that
    // that loop consists of two instructions, so we must check for `halt` and `halt_inner` labels.
    let (halt_pc0, halt_pc1) = get_halt_pcs::<P::Scalar>();
    yield_constr
        .constraint_last_row((lv.program_counter - halt_pc0) * (lv.program_counter - halt_pc1));
    // Finally, the last row must be in kernel mode.
    yield_constr.constraint_last_row(lv.is_kernel_mode - P::ONES);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // Once we start executing instructions, then we continue until the end of the table.
    {
        let constr = builder.mul_sub_extension(lv.is_cpu_cycle, nv.is_cpu_cycle, lv.is_cpu_cycle);
        yield_constr.constraint_transition(builder, constr);
    }

    // If a row is a CPU cycle and executing a native instruction (implemented as a table row; not
    // microcoded) then the program counter is incremented by 1 to obtain the next row's program
    // counter. Also, the next row has the same kernel flag.
    {
        let is_native_instruction =
            builder.add_many_extension(NATIVE_INSTRUCTIONS.iter().map(|&col_i| lv[col_i]));
        let filter = builder.mul_extension(lv.is_cpu_cycle, is_native_instruction);
        let pc_diff = builder.sub_extension(lv.program_counter, nv.program_counter);
        let pc_constr = builder.mul_add_extension(filter, pc_diff, filter);
        yield_constr.constraint_transition(builder, pc_constr);
        let kernel_diff = builder.sub_extension(lv.is_kernel_mode, nv.is_kernel_mode);
        let kernel_constr = builder.mul_extension(filter, kernel_diff);
        yield_constr.constraint_transition(builder, kernel_constr);
    }

    // If a non-CPU cycle row is followed by a CPU cycle row, then:
    //  - the `program_counter` of the CPU cycle row is `route_txn` (the entry point of our kernel),
    //  - execution is in kernel mode, and
    //  - the stack is empty.
    {
        let is_last_noncpu_cycle =
            builder.mul_sub_extension(lv.is_cpu_cycle, nv.is_cpu_cycle, nv.is_cpu_cycle);

        // Start at `route_txn`.
        let route_txn = builder.constant_extension(F::Extension::from_canonical_usize(
            KERNEL.global_labels["route_txn"],
        ));
        let pc_diff = builder.sub_extension(nv.program_counter, route_txn);
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

    // The last row must be a CPU cycle row.
    {
        let one = builder.one_extension();
        let constr = builder.sub_extension(lv.is_cpu_cycle, one);
        yield_constr.constraint_last_row(builder, constr);
    }
    // Also, the last row's `program_counter` must be inside the `halt` infinite loop. Note that
    // that loop consists of two instructions, so we must check for `halt` and `halt_inner` labels.
    {
        let (halt_pc0, halt_pc1) = get_halt_pcs();
        let halt_pc0_target = builder.constant_extension(halt_pc0);
        let halt_pc1_target = builder.constant_extension(halt_pc1);

        let halt_pc0_offset = builder.sub_extension(lv.program_counter, halt_pc0_target);
        let halt_pc1_offset = builder.sub_extension(lv.program_counter, halt_pc1_target);
        let constr = builder.mul_extension(halt_pc0_offset, halt_pc1_offset);

        yield_constr.constraint_last_row(builder, constr);
    }
    // Finally, the last row must be in kernel mode.
    {
        let one = builder.one_extension();
        let constr = builder.sub_extension(lv.is_kernel_mode, one);
        yield_constr.constraint_last_row(builder, constr);
    }
}
