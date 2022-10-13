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
struct StackBehavior {
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
    num_pops: 2,
    pushes: true,
    disable_other_channels: true,
});

// AUDITORS: If the value below is `None`, then the operation must be manually checked to ensure
// that every general-purpose memory channel is either disabled or has its read flag and address
// propertly constrained. The same applies  when `disable_other_channels` is set to `false`,
// except the first `num_pops` and the last `pushes as usize` channels have their read flag and
// address constrained automatically in this file.
const STACK_BEHAVIORS: OpsColumnsView<Option<StackBehavior>> = OpsColumnsView {
    stop: None, // TODO
    add: BASIC_BINARY_OP,
    mul: BASIC_BINARY_OP,
    sub: BASIC_BINARY_OP,
    div: BASIC_BINARY_OP,
    sdiv: BASIC_BINARY_OP,
    mod_: BASIC_BINARY_OP,
    smod: BASIC_BINARY_OP,
    addmod: BASIC_TERNARY_OP,
    mulmod: BASIC_TERNARY_OP,
    exp: None, // TODO
    signextend: BASIC_BINARY_OP,
    addfp254: BASIC_BINARY_OP,
    mulfp254: BASIC_BINARY_OP,
    subfp254: BASIC_BINARY_OP,
    lt: BASIC_BINARY_OP,
    gt: BASIC_BINARY_OP,
    slt: BASIC_BINARY_OP,
    sgt: BASIC_BINARY_OP,
    eq: BASIC_BINARY_OP,
    iszero: BASIC_UNARY_OP,
    and: BASIC_BINARY_OP,
    or: BASIC_BINARY_OP,
    xor: BASIC_BINARY_OP,
    not: BASIC_TERNARY_OP,
    byte: BASIC_BINARY_OP,
    shl: BASIC_BINARY_OP,
    shr: BASIC_BINARY_OP,
    sar: BASIC_BINARY_OP,
    keccak256: None,        // TODO
    keccak_general: None,   // TODO
    address: None,          // TODO
    balance: None,          // TODO
    origin: None,           // TODO
    caller: None,           // TODO
    callvalue: None,        // TODO
    calldataload: None,     // TODO
    calldatasize: None,     // TODO
    calldatacopy: None,     // TODO
    codesize: None,         // TODO
    codecopy: None,         // TODO
    gasprice: None,         // TODO
    extcodesize: None,      // TODO
    extcodecopy: None,      // TODO
    returndatasize: None,   // TODO
    returndatacopy: None,   // TODO
    extcodehash: None,      // TODO
    blockhash: None,        // TODO
    coinbase: None,         // TODO
    timestamp: None,        // TODO
    number: None,           // TODO
    difficulty: None,       // TODO
    gaslimit: None,         // TODO
    chainid: None,          // TODO
    selfbalance: None,      // TODO
    basefee: None,          // TODO
    prover_input: None,     // TODO
    pop: None,              // TODO
    mload: None,            // TODO
    mstore: None,           // TODO
    mstore8: None,          // TODO
    sload: None,            // TODO
    sstore: None,           // TODO
    jump: None,             // TODO
    jumpi: None,            // TODO
    pc: None,               // TODO
    msize: None,            // TODO
    gas: None,              // TODO
    jumpdest: None,         // TODO
    get_state_root: None,   // TODO
    set_state_root: None,   // TODO
    get_receipt_root: None, // TODO
    set_receipt_root: None, // TODO
    push: None,             // TODO
    dup: None,
    swap: None,
    log0: None,           // TODO
    log1: None,           // TODO
    log2: None,           // TODO
    log3: None,           // TODO
    log4: None,           // TODO
    create: None,         // TODO
    call: None,           // TODO
    callcode: None,       // TODO
    return_: None,        // TODO
    delegatecall: None,   // TODO
    create2: None,        // TODO
    get_context: None,    // TODO
    set_context: None,    // TODO
    consume_gas: None,    // TODO
    exit_kernel: None,    // TODO
    staticcall: None,     // TODO
    mload_general: None,  // TODO
    mstore_general: None, // TODO
    revert: None,         // TODO
    selfdestruct: None,   // TODO
    invalid: None,        // TODO
};

fn eval_packed_one<P: PackedField>(
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
            let filter = lv.is_cpu_cycle * op;
            eval_packed_one(lv, filter, stack_behavior, yield_constr);
        }
    }
}

fn eval_ext_circuit_one<F: RichField + Extendable<D>, const D: usize>(
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
            let filter = builder.mul_extension(lv.is_cpu_cycle, op);
            eval_ext_circuit_one(builder, lv, filter, stack_behavior, yield_constr);
        }
    }
}
