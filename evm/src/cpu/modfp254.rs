use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;

// Python:
// >>> P = 21888242871839275222246405745257275088696311157297823662689037894645226208583
// >>> "[" + ", ".join(hex((P >> n) % 2**32) for n in range(0, 256, 32)) + "]"
const P_LIMBS: [u32; 8] = [
    0xd87cfd47, 0x3c208c16, 0x6871ca8d, 0x97816a91, 0x8181585d, 0xb85045b6, 0xe131a029, 0x30644e72,
];

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.is_cpu_cycle * (lv.op.addfp254 + lv.op.mulfp254 + lv.op.subfp254);

    // We want to use all the same logic as the usual mod operations, but without needing to read
    // the modulus from the stack. We simply constrain `mem_channels[2]` to be our prime (that's
    // where the modulus goes in the generalized operations).
    let channel_val = lv.mem_channels[2].value;
    for (channel_limb, p_limb) in izip!(channel_val, P_LIMBS) {
        let p_limb = P::Scalar::from_canonical_u32(p_limb);
        yield_constr.constraint(filter * (channel_limb - p_limb));
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = {
        let flag_sum = builder.add_many_extension([lv.op.addfp254, lv.op.mulfp254, lv.op.subfp254]);
        builder.mul_extension(lv.is_cpu_cycle, flag_sum)
    };

    // We want to use all the same logic as the usual mod operations, but without needing to read
    // the modulus from the stack. We simply constrain `mem_channels[2]` to be our prime (that's
    // where the modulus goes in the generalized operations).
    let channel_val = lv.mem_channels[2].value;
    for (channel_limb, p_limb) in izip!(channel_val, P_LIMBS) {
        let p_limb = F::from_canonical_u32(p_limb);
        let constr = builder.arithmetic_extension(F::ONE, -p_limb, filter, channel_limb, filter);
        yield_constr.constraint(builder, constr);
    }
}
