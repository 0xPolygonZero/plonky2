use once_cell::sync::Lazy;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP};
use crate::cpu::kernel::aggregator::KERNEL;

const NUM_TRAPS: usize = 2;

fn make_trap_list() -> [(usize, usize); NUM_TRAPS] {
    let kernel = Lazy::force(&KERNEL);
    [
        (COL_MAP.is_stop, "handle_stop"),
        (COL_MAP.is_exp, "handle_exp"),
    ]
    .map(|(col_index, handler_name)| (col_index, kernel.global_labels[handler_name]))
}

static TRAP_LIST: Lazy<[(usize, usize); NUM_TRAPS]> = Lazy::new(make_trap_list);

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let lv_traps = lv.general.traps();
    let trap_list = Lazy::force(&TRAP_LIST);
    let should_trap: P = trap_list.iter().map(|&(col_index, _)| lv[col_index]).sum();
    let filter = lv.is_cpu_cycle * should_trap;

    // If trapping: set program counter to the handler address
    let trap_dst: P = trap_list
        .iter()
        .map(|&(col_index, handler_addr)| {
            lv[col_index] * P::Scalar::from_canonical_usize(handler_addr)
        })
        .sum();
    yield_constr.constraint_transition(filter * (nv.program_counter - trap_dst));
    // If trapping: set kernel mode
    yield_constr.constraint_transition(filter * (nv.is_kernel_mode - P::ONES));
    // If trapping: push current PC to stack
    yield_constr.constraint(filter * (lv_traps.output[0] - lv.program_counter));
    // If trapping: push current kernel flag to stack (share register with PC)
    yield_constr.constraint(filter * (lv_traps.output[1] - lv.is_kernel_mode));
    // If trapping: zero the rest of that register
    for &limb in &lv_traps.output[2..] {
        yield_constr.constraint(filter * limb);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let lv_traps = lv.general.traps();
    let trap_list = Lazy::force(&TRAP_LIST);
    let should_trap =
        builder.add_many_extension(trap_list.iter().map(|&(col_index, _)| lv[col_index]));
    let filter = builder.mul_extension(lv.is_cpu_cycle, should_trap);

    // If trapping: set program counter to the handler address
    {
        let trap_dst = trap_list.iter().fold(
            builder.zero_extension(),
            |cumul, &(col_index, handler_addr)| {
                let handler_addr = F::from_canonical_usize(handler_addr);
                builder.mul_const_add_extension(handler_addr, lv[col_index], cumul)
            },
        );
        let constr = builder.sub_extension(nv.program_counter, trap_dst);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint_transition(builder, constr);
    }
    // If trapping: set kernel mode
    {
        let constr = builder.mul_sub_extension(filter, nv.is_kernel_mode, filter);
        yield_constr.constraint_transition(builder, constr);
    }
    // If trapping: push current PC to stack
    {
        let constr = builder.sub_extension(lv_traps.output[0], lv.program_counter);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    // If trapping: push current kernel flag to stack (share register with PC)
    {
        let constr = builder.sub_extension(lv_traps.output[1], lv.is_kernel_mode);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    // If trapping: zero the rest of that register
    for &limb in &lv_traps.output[2..] {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }
}
