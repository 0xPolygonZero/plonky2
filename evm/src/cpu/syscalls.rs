//! Handle instructions that are implemented in terms of system calls.
//!
//! These are usually the ones that are too complicated to implement in one CPU table row.

use once_cell::sync::Lazy;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP};
use crate::cpu::kernel::aggregator::KERNEL;

const NUM_SYSCALLS: usize = 3;

fn make_syscall_list() -> [(usize, usize); NUM_SYSCALLS] {
    let kernel = Lazy::force(&KERNEL);
    [
        (COL_MAP.is_stop, "sys_stop"),
        (COL_MAP.is_exp, "sys_exp"),
        (COL_MAP.is_invalid, "handle_invalid"),
    ]
    .map(|(col_index, handler_name)| (col_index, kernel.global_labels[handler_name]))
}

static TRAP_LIST: Lazy<[(usize, usize); NUM_SYSCALLS]> = Lazy::new(make_syscall_list);

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let syscall_list = Lazy::force(&TRAP_LIST);
    // 1 if _any_ syscall, else 0.
    let should_syscall: P = syscall_list
        .iter()
        .map(|&(col_index, _)| lv[col_index])
        .sum();
    let filter = lv.is_cpu_cycle * should_syscall;

    // If syscall: set program counter to the handler address
    // Note that at most one of the `lv[col_index]`s will be 1 and all others will be 0.
    let syscall_dst: P = syscall_list
        .iter()
        .map(|&(col_index, handler_addr)| {
            lv[col_index] * P::Scalar::from_canonical_usize(handler_addr)
        })
        .sum();
    yield_constr.constraint_transition(filter * (nv.program_counter - syscall_dst));
    // If syscall: set kernel mode
    yield_constr.constraint_transition(filter * (nv.is_kernel_mode - P::ONES));

    let output = lv.mem_channels[0].value;
    // If syscall: push current PC to stack
    yield_constr.constraint(filter * (output[0] - lv.program_counter));
    // If syscall: push current kernel flag to stack (share register with PC)
    yield_constr.constraint(filter * (output[1] - lv.is_kernel_mode));
    // If syscall: zero the rest of that register
    for &limb in &output[2..] {
        yield_constr.constraint(filter * limb);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let syscall_list = Lazy::force(&TRAP_LIST);
    // 1 if _any_ syscall, else 0.
    let should_syscall =
        builder.add_many_extension(syscall_list.iter().map(|&(col_index, _)| lv[col_index]));
    let filter = builder.mul_extension(lv.is_cpu_cycle, should_syscall);

    // If syscall: set program counter to the handler address
    {
        // Note that at most one of the `lv[col_index]`s will be 1 and all others will be 0.
        let syscall_dst = syscall_list.iter().fold(
            builder.zero_extension(),
            |cumul, &(col_index, handler_addr)| {
                let handler_addr = F::from_canonical_usize(handler_addr);
                builder.mul_const_add_extension(handler_addr, lv[col_index], cumul)
            },
        );
        let constr = builder.sub_extension(nv.program_counter, syscall_dst);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint_transition(builder, constr);
    }
    // If syscall: set kernel mode
    {
        let constr = builder.mul_sub_extension(filter, nv.is_kernel_mode, filter);
        yield_constr.constraint_transition(builder, constr);
    }

    let output = lv.mem_channels[0].value;
    // If syscall: push current PC to stack
    {
        let constr = builder.sub_extension(output[0], lv.program_counter);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    // If syscall: push current kernel flag to stack (share register with PC)
    {
        let constr = builder.sub_extension(output[1], lv.is_kernel_mode);
        let constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
    }
    // If syscall: zero the rest of that register
    for &limb in &output[2..] {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }
}
