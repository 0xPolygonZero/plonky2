use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::kernel::aggregator::KERNEL;

fn get_halt_addresses<F: Field>() -> (F, F) {
    let halt_addr = KERNEL.global_labels["halt"];
    let halt_inner_addr = KERNEL.global_labels["halt_inner"];

    (
        F::from_canonical_usize(halt_addr),
        F::from_canonical_usize(halt_inner_addr),
    )
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // Once we start executing instructions, then we continue until the end of the table.
    yield_constr.constraint_transition(lv.is_cpu_cycle * (nv.is_cpu_cycle - P::ONES));

    // If a row is a CPU cycle, then its `next_program_counter` becomes the `program_counter` of the
    // next row.
    yield_constr
        .constraint_transition(lv.is_cpu_cycle * (nv.program_counter - lv.next_program_counter));

    // If a non-CPU cycle row is followed by a CPU cycle row, then the `program_counter` of the CPU
    // cycle row is 0.
    yield_constr
        .constraint_transition((lv.is_cpu_cycle - P::ONES) * nv.is_cpu_cycle * nv.program_counter);

    // The first row has nowhere to continue execution from, so if it's a cycle row, then its
    // `program_counter` must be 0.
    // NB: I know the first few rows will be used for initialization and will not be CPU cycle rows.
    // Once that's done, then this constraint can be removed. Until then, it is needed to ensure
    // that execution starts at 0 and not at any arbitrary offset.
    yield_constr.constraint_first_row(lv.is_cpu_cycle * lv.program_counter);

    // The last row must be a CPU cycle row.
    yield_constr.constraint_last_row(lv.is_cpu_cycle - P::ONES);
    // Also, the last row's `program_counter` must be inside the `halt` infinite loop. Note that
    // that loop consists of two instructions, so we must check for `halt` and `halt_inner` labels.
    let (halt_addr0, halt_addr1) = get_halt_addresses::<P::Scalar>();
    yield_constr
        .constraint_last_row((lv.program_counter - halt_addr0) * (lv.program_counter - halt_addr1));
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

    // If a row is a CPU cycle, then its `next_program_counter` becomes the `program_counter` of the
    // next row.
    {
        let constr = builder.sub_extension(nv.program_counter, lv.next_program_counter);
        let constr = builder.mul_extension(lv.is_cpu_cycle, constr);
        yield_constr.constraint_transition(builder, constr);
    }

    // If a non-CPU cycle row is followed by a CPU cycle row, then the `program_counter` of the CPU
    // cycle row is 0.
    {
        let constr = builder.mul_extension(nv.is_cpu_cycle, nv.program_counter);
        let constr = builder.mul_sub_extension(lv.is_cpu_cycle, constr, constr);
        yield_constr.constraint_transition(builder, constr);
    }

    // The first row has nowhere to continue execution from, so if it's a cycle row, then its
    // `program_counter` must be 0.
    // NB: I know the first few rows will be used for initialization and will not be CPU cycle rows.
    // Once that's done, then this constraint can be removed. Until then, it is needed to ensure
    // that execution starts at 0 and not at any arbitrary offset.
    {
        let constr = builder.mul_extension(lv.is_cpu_cycle, lv.program_counter);
        yield_constr.constraint_first_row(builder, constr);
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
        let (halt_addr0, halt_addr1) = get_halt_addresses();
        let halt_addr0_target = builder.constant_extension(halt_addr0);
        let halt_addr1_target = builder.constant_extension(halt_addr1);

        let halt_addr0_offset = builder.sub_extension(lv.program_counter, halt_addr0_target);
        let halt_addr1_offset = builder.sub_extension(lv.program_counter, halt_addr1_target);
        let constr = builder.mul_extension(halt_addr0_offset, halt_addr1_offset);

        yield_constr.constraint_last_row(builder, constr);
    }
}
