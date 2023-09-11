use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::ops::OpsColumnsView;
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory::segments::Segment;

#[derive(Clone, Copy)]
pub(crate) struct StackBehavior {
    num_pops: usize,
    pushes: bool,
    disable_other_channels: bool,
}

const BASIC_UNARY_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 1,
    pushes: true,
    disable_other_channels: true,
});
const BASIC_BINARY_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 2,
    pushes: true,
    disable_other_channels: true,
});
const BASIC_TERNARY_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 3,
    pushes: true,
    disable_other_channels: true,
});
pub(crate) const JUMP_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 1,
    pushes: false,
    disable_other_channels: false,
});
pub(crate) const JUMPI_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 2,
    pushes: false,
    disable_other_channels: false,
});

pub(crate) const MLOAD_GENERAL_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 3,
    pushes: true,
    disable_other_channels: false,
});

pub(crate) const MSTORE_GENERAL_OP: Option<StackBehavior> = Some(StackBehavior {
    num_pops: 4,
    pushes: false,
    disable_other_channels: false,
});

// AUDITORS: If the value below is `None`, then the operation must be manually checked to ensure
// that every general-purpose memory channel is either disabled or has its read flag and address
// propertly constrained. The same applies  when `disable_other_channels` is set to `false`,
// except the first `num_pops` and the last `pushes as usize` channels have their read flag and
// address constrained automatically in this file.
const STACK_BEHAVIORS: OpsColumnsView<Option<StackBehavior>> = OpsColumnsView {
    add: BASIC_BINARY_OP,
    mul: BASIC_BINARY_OP,
    sub: BASIC_BINARY_OP,
    div: BASIC_BINARY_OP,
    mod_: BASIC_BINARY_OP,
    addmod: BASIC_TERNARY_OP,
    mulmod: BASIC_TERNARY_OP,
    addfp254: BASIC_BINARY_OP,
    mulfp254: BASIC_BINARY_OP,
    subfp254: BASIC_BINARY_OP,
    submod: BASIC_TERNARY_OP,
    lt: BASIC_BINARY_OP,
    gt: BASIC_BINARY_OP,
    eq_iszero: None, // EQ is binary, IS_ZERO is unary.
    logic_op: BASIC_BINARY_OP,
    not: BASIC_UNARY_OP,
    byte: BASIC_BINARY_OP,
    shl: Some(StackBehavior {
        num_pops: 2,
        pushes: true,
        disable_other_channels: false,
    }),
    shr: Some(StackBehavior {
        num_pops: 2,
        pushes: true,
        disable_other_channels: false,
    }),
    keccak_general: Some(StackBehavior {
        num_pops: 4,
        pushes: true,
        disable_other_channels: true,
    }),
    prover_input: None, // TODO
    pop: Some(StackBehavior {
        num_pops: 1,
        pushes: false,
        disable_other_channels: true,
    }),
    jumps: None, // Depends on whether it's a JUMP or a JUMPI.
    pc: Some(StackBehavior {
        num_pops: 0,
        pushes: true,
        disable_other_channels: true,
    }),
    jumpdest: Some(StackBehavior {
        num_pops: 0,
        pushes: false,
        disable_other_channels: true,
    }),
    push0: Some(StackBehavior {
        num_pops: 0,
        pushes: true,
        disable_other_channels: true,
    }),
    push: None, // TODO
    dup: None,
    swap: None,
    context_op: None, // SET_CONTEXT is special since it involves the old and the new stack.
    exit_kernel: Some(StackBehavior {
        num_pops: 1,
        pushes: false,
        disable_other_channels: true,
    }),
    m_op_general: None,
    syscall: Some(StackBehavior {
        num_pops: 0,
        pushes: true,
        disable_other_channels: false,
    }),
    exception: Some(StackBehavior {
        num_pops: 0,
        pushes: true,
        disable_other_channels: false,
    }),
};

pub(crate) const EQ_STACK_BEHAVIOR: Option<StackBehavior> = BASIC_BINARY_OP;
pub(crate) const IS_ZERO_STACK_BEHAVIOR: Option<StackBehavior> = BASIC_UNARY_OP;

pub(crate) fn eval_packed_one<P: PackedField>(
    lv: &CpuColumnsView<P>,
    filter: P,
    stack_behavior: StackBehavior,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let num_operands = stack_behavior.num_pops + (stack_behavior.pushes as usize);
    assert!(num_operands <= NUM_GP_CHANNELS);

    // Pops
    for i in 0..stack_behavior.num_pops {
        let channel = lv.mem_channels[i];

        yield_constr.constraint(filter * (channel.used - P::ONES));
        yield_constr.constraint(filter * (channel.is_read - P::ONES));

        yield_constr.constraint(filter * (channel.addr_context - lv.context));
        yield_constr.constraint(
            filter * (channel.addr_segment - P::Scalar::from_canonical_u64(Segment::Stack as u64)),
        );
        // E.g. if `stack_len == 1` and `i == 0`, we want `add_virtual == 0`.
        let addr_virtual = lv.stack_len - P::Scalar::from_canonical_usize(i + 1);
        yield_constr.constraint(filter * (channel.addr_virtual - addr_virtual));
    }

    // Pushes
    if stack_behavior.pushes {
        let channel = lv.mem_channels[NUM_GP_CHANNELS - 1];

        yield_constr.constraint(filter * (channel.used - P::ONES));
        yield_constr.constraint(filter * channel.is_read);

        yield_constr.constraint(filter * (channel.addr_context - lv.context));
        yield_constr.constraint(
            filter * (channel.addr_segment - P::Scalar::from_canonical_u64(Segment::Stack as u64)),
        );
        let addr_virtual = lv.stack_len - P::Scalar::from_canonical_usize(stack_behavior.num_pops);
        yield_constr.constraint(filter * (channel.addr_virtual - addr_virtual));
    }

    // Unused channels
    if stack_behavior.disable_other_channels {
        for i in stack_behavior.num_pops..NUM_GP_CHANNELS - (stack_behavior.pushes as usize) {
            let channel = lv.mem_channels[i];
            yield_constr.constraint(filter * channel.used);
        }
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    for (op, stack_behavior) in izip!(lv.op.into_iter(), STACK_BEHAVIORS.into_iter()) {
        if let Some(stack_behavior) = stack_behavior {
            eval_packed_one(lv, op, stack_behavior, yield_constr);
        }
    }
}

pub(crate) fn eval_ext_circuit_one<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    filter: ExtensionTarget<D>,
    stack_behavior: StackBehavior,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let num_operands = stack_behavior.num_pops + (stack_behavior.pushes as usize);
    assert!(num_operands <= NUM_GP_CHANNELS);

    // Pops
    for i in 0..stack_behavior.num_pops {
        let channel = lv.mem_channels[i];

        {
            let constr = builder.mul_sub_extension(filter, channel.used, filter);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.mul_sub_extension(filter, channel.is_read, filter);
            yield_constr.constraint(builder, constr);
        }

        {
            let diff = builder.sub_extension(channel.addr_context, lv.context);
            let constr = builder.mul_extension(filter, diff);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.arithmetic_extension(
                F::ONE,
                -F::from_canonical_u64(Segment::Stack as u64),
                filter,
                channel.addr_segment,
                filter,
            );
            yield_constr.constraint(builder, constr);
        }
        {
            let diff = builder.sub_extension(channel.addr_virtual, lv.stack_len);
            let constr = builder.arithmetic_extension(
                F::ONE,
                F::from_canonical_usize(i + 1),
                filter,
                diff,
                filter,
            );
            yield_constr.constraint(builder, constr);
        }
    }

    // Pushes
    if stack_behavior.pushes {
        let channel = lv.mem_channels[NUM_GP_CHANNELS - 1];

        {
            let constr = builder.mul_sub_extension(filter, channel.used, filter);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.mul_extension(filter, channel.is_read);
            yield_constr.constraint(builder, constr);
        }

        {
            let diff = builder.sub_extension(channel.addr_context, lv.context);
            let constr = builder.mul_extension(filter, diff);
            yield_constr.constraint(builder, constr);
        }
        {
            let constr = builder.arithmetic_extension(
                F::ONE,
                -F::from_canonical_u64(Segment::Stack as u64),
                filter,
                channel.addr_segment,
                filter,
            );
            yield_constr.constraint(builder, constr);
        }
        {
            let diff = builder.sub_extension(channel.addr_virtual, lv.stack_len);
            let constr = builder.arithmetic_extension(
                F::ONE,
                F::from_canonical_usize(stack_behavior.num_pops),
                filter,
                diff,
                filter,
            );
            yield_constr.constraint(builder, constr);
        }
    }

    // Unused channels
    if stack_behavior.disable_other_channels {
        for i in stack_behavior.num_pops..NUM_GP_CHANNELS - (stack_behavior.pushes as usize) {
            let channel = lv.mem_channels[i];
            let constr = builder.mul_extension(filter, channel.used);
            yield_constr.constraint(builder, constr);
        }
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    for (op, stack_behavior) in izip!(lv.op.into_iter(), STACK_BEHAVIORS.into_iter()) {
        if let Some(stack_behavior) = stack_behavior {
            eval_ext_circuit_one(builder, lv, op, stack_behavior, yield_constr);
        }
    }
}
