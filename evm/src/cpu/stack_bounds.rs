//! Checks for stack underflow and overflow.
//!
//! The constraints defined herein validate that stack exceptions (underflow and overflow) do not
//! occur. For example, if `is_add` is set but an addition would underflow, these constraints would
//! make the proof unverifiable.
//!
//! Faults are handled under a separate operation flag, `is_exception` (this is still TODO), which
//! traps to the kernel. The kernel then handles the exception. However, before it may do so, the
//! kernel must verify in software that an exception did in fact occur (i.e. the trap was
//! warranted) and `PANIC` otherwise; this prevents the prover from faking an exception on a valid
//! operation.

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, COL_MAP};

const MAX_USER_STACK_SIZE: u64 = 1024;

// Below only includes the operations that pop the top of the stack **without reading the value from
// memory**, i.e. `POP`.
//   Other operations that have a minimum stack size (e.g. `MULMOD`, which has three inputs) read
// all their inputs from memory. On underflow, the cross-table lookup fails, as -1, ..., -17 are
// invalid memory addresses.
const DECREMENTING_FLAGS: [usize; 1] = [COL_MAP.is_pop];

// Operations that increase the stack length by 1, but excluding:
//  - privileged (kernel-only) operations (superfluous; doesn't affect correctness),
//  - operations that from userspace to the kernel (required for correctness).
// TODO: This list is incomplete.
const INCREMENTING_FLAGS: [usize; 2] = [COL_MAP.is_pc, COL_MAP.is_dup];

/// Calculates `lv.stack_len_bounds_aux`. Note that this must be run after decode.
pub fn generate<F: Field>(lv: &mut CpuColumnsView<F>) {
    let cycle_filter = lv.is_cpu_cycle;
    if cycle_filter == F::ZERO {
        return;
    }

    let check_underflow: F = DECREMENTING_FLAGS.map(|i| lv[i]).into_iter().sum();
    let check_overflow: F = INCREMENTING_FLAGS.map(|i| lv[i]).into_iter().sum();
    let no_check = F::ONE - (check_underflow + check_overflow);

    let disallowed_len = check_overflow * F::from_canonical_u64(MAX_USER_STACK_SIZE) - no_check;
    let diff = lv.stack_len - disallowed_len;

    let user_mode = F::ONE - lv.is_kernel_mode;
    let rhs = user_mode + check_underflow;

    lv.stack_len_bounds_aux = match diff.try_inverse() {
        Some(diff_inv) => diff_inv * rhs, // `rhs` may be a value other than 1 or 0
        None => {
            assert_eq!(rhs, F::ZERO);
            F::ZERO
        }
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // `check_underflow`, `check_overflow`, and `no_check` are mutually exclusive.
    let check_underflow: P = DECREMENTING_FLAGS.map(|i| lv[i]).into_iter().sum();
    let check_overflow: P = INCREMENTING_FLAGS.map(|i| lv[i]).into_iter().sum();
    let no_check = P::ONES - (check_underflow + check_overflow);

    // If `check_underflow`, then the instruction we are executing pops a value from the stack
    // without reading it from memory, and the usual underflow checks do not work. We must show that
    // `lv.stack_len` is not 0. We choose to perform this check whether or not we're in kernel mode.
    // (The check in kernel mode is not necessary if the kernel is correct, but this is an easy
    // sanity check.
    //   If `check_overflow`, then the instruction we are executing increases the stack length by 1.
    // If we are in user mode, then we must show that the stack length is not currently
    // `MAX_USER_STACK_SIZE`, as this is the maximum for the user stack. Note that this check must
    // not run in kernel mode as the kernel's stack length is unrestricted.
    //   If `no_check`, then we don't need to check anything. The constraint is written to always
    // test that `lv.stack_len` does not equal _something_ so we just show that it's not -1, which
    // is always true.

    // 0 if `check_underflow`, `MAX_USER_STACK_SIZE` if `check_overflow`, and -1 if `no_check`.
    let disallowed_len =
        check_overflow * P::Scalar::from_canonical_u64(MAX_USER_STACK_SIZE) - no_check;
    // This `lhs` must equal some `rhs`. If `rhs` is nonzero, then this shows that `lv.stack_len` is
    // not `disallowed_len`.
    let lhs = (lv.stack_len - disallowed_len) * lv.stack_len_bounds_aux;

    // We want this constraint to be active if we're in user mode OR the instruction might overflow.
    // (In other words, we want to _skip_ overflow checks in kernel mode).
    let user_mode = P::ONES - lv.is_kernel_mode;
    // `rhs` is may be 0, 1, or 2. It's 0 if we're in kernel mode and we would be checking for
    // overflow.
    // Note: if `user_mode` and `check_underflow` then, `rhs` is 2. This is fine: we're still
    // showing that `lv.stack_len - disallowed_len` is nonzero.
    let rhs = user_mode + check_underflow;

    yield_constr.constraint(lv.is_cpu_cycle * (lhs - rhs));
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();
    let max_stack_size =
        builder.constant_extension(F::from_canonical_u64(MAX_USER_STACK_SIZE).into());

    // `check_underflow`, `check_overflow`, and `no_check` are mutually exclusive.
    let check_underflow = builder.add_many_extension(DECREMENTING_FLAGS.map(|i| lv[i]));
    let check_overflow = builder.add_many_extension(INCREMENTING_FLAGS.map(|i| lv[i]));
    let no_check = {
        let any_check = builder.add_extension(check_underflow, check_overflow);
        builder.sub_extension(one, any_check)
    };

    // If `check_underflow`, then the instruction we are executing pops a value from the stack
    // without reading it from memory, and the usual underflow checks do not work. We must show that
    // `lv.stack_len` is not 0. We choose to perform this check whether or not we're in kernel mode.
    // (The check in kernel mode is not necessary if the kernel is correct, but this is an easy
    // sanity check.
    //   If `check_overflow`, then the instruction we are executing increases the stack length by 1.
    // If we are in user mode, then we must show that the stack length is not currently
    // `MAX_USER_STACK_SIZE`, as this is the maximum for the user stack. Note that this check must
    // not run in kernel mode as the kernel's stack length is unrestricted.
    //   If `no_check`, then we don't need to check anything. The constraint is written to always
    // test that `lv.stack_len` does not equal _something_ so we just show that it's not -1, which
    // is always true.

    // 0 if `check_underflow`, `MAX_USER_STACK_SIZE` if `check_overflow`, and -1 if `no_check`.
    let disallowed_len = builder.mul_sub_extension(check_overflow, max_stack_size, no_check);
    // This `lhs` must equal some `rhs`. If `rhs` is nonzero, then this shows that `lv.stack_len` is
    // not `disallowed_len`.
    let lhs = {
        let diff = builder.sub_extension(lv.stack_len, disallowed_len);
        builder.mul_extension(diff, lv.stack_len_bounds_aux)
    };

    // We want this constraint to be active if we're in user mode OR the instruction might overflow.
    // (In other words, we want to _skip_ overflow checks in kernel mode).
    let user_mode = builder.sub_extension(one, lv.is_kernel_mode);
    // `rhs` is may be 0, 1, or 2. It's 0 if we're in kernel mode and we would be checking for
    // overflow.
    // Note: if `user_mode` and `check_underflow` then, `rhs` is 2. This is fine: we're still
    // showing that `lv.stack_len - disallowed_len` is nonzero.
    let rhs = builder.add_extension(user_mode, check_underflow);

    let constr = {
        let diff = builder.sub_extension(lhs, rhs);
        builder.mul_extension(lv.is_cpu_cycle, diff)
    };
    yield_constr.constraint(builder, constr);
}
