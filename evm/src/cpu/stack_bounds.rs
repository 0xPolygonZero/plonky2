//! Checks for stack overflow.
//!
//! The constraints defined herein validate that stack overflow did not occur. For example, if `dup`
//! is set but the copy would overflow, these constraints would make the proof unverifiable.
//!
//! Faults are handled under a separate operation flag, `exception` , which traps to the kernel. The
//! kernel then handles the exception. However, before it may do so, it must verify in software that
//! an exception did in fact occur (i.e. the trap was warranted) and `PANIC` otherwise; this
//! prevents the prover from faking an exception on a valid operation.

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;

pub const MAX_USER_STACK_SIZE: usize = 1024;

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // If we're in user mode, ensure that the stack length is not 1025. Note that a stack length of
    // 1024 is valid. 1025 means we've gone one over, which is necessary for overflow, as an EVM
    // opcode increases the stack length by at most one.

    let filter = lv.is_cpu_cycle;
    let diff = lv.stack_len - P::Scalar::from_canonical_usize(MAX_USER_STACK_SIZE + 1);
    let lhs = diff * lv.stack_len_bounds_aux;
    let rhs = P::ONES - lv.is_kernel_mode;

    yield_constr.constraint(filter * (lhs - rhs));
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // If we're in user mode, ensure that the stack length is not 1025. Note that a stack length of
    // 1024 is valid. 1025 means we've gone one over, which is necessary for overflow, as an EVM
    // opcode increases the stack length by at most one.

    let filter = lv.is_cpu_cycle;

    let lhs = builder.arithmetic_extension(
        F::ONE,
        -F::from_canonical_usize(MAX_USER_STACK_SIZE + 1),
        lv.stack_len,
        lv.stack_len_bounds_aux,
        lv.stack_len_bounds_aux,
    );
    let constr = builder.add_extension(lhs, lv.is_kernel_mode);
    let constr = builder.mul_sub_extension(filter, constr, filter);
    yield_constr.constraint(builder, constr);
}
