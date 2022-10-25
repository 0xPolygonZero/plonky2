use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::memory::segments::Segment;

fn generate_shl<F: RichField>(lv: &mut CpuColumnsView<F>) {
    if lv.op.shl == F::ZERO {
        return;
    }
    assert!(lv.op.shl == F::ONE);

    let _val = lv.mem_channels[0].value;
    let shift = lv.mem_channels[1].value;
    let shift_limb0 = shift[0].to_canonical_u64();
    let output = &mut lv.mem_channels[3].value;

    // NB: It is not strictly necessary to check that the shift amount
    // is less than 256, it is only necessary to check that it is less
    // than 2^32 since, when looking up shift table values for shifts
    // between 256 and 2^32-1, the result will be zero. We make the
    // check explicit here to clarify intent and because it's easy.

    let mut two_exp = [F::ZERO; 8]; // FIXME
    //let tail_limbs_sum: F = shift[1..].iter().sum(); // TODO: Why doesn't this work?
    let tail_limbs_sum: u64 = shift[1..].iter().map(|&c| c.to_canonical_u64()).sum();
    if shift_limb0 < 256 && tail_limbs_sum == 0 {
        // Shift amount was < 256...
        let table_val = &mut lv.mem_channels[2];

        table_val.addr_context = F::ZERO; // kernel context
        table_val.addr_segment = F::from_canonical_u64(Segment::ShiftTable as u64);
        // FIXME: double-check that each address refers to 256 bits, not 1 byte
        table_val.addr_virtual = shift[0];

        // FIXME: set two_exp to value at table_val
    }
    // else
    //     Shift amount was >= 256, so the result is zero regardless of
    //     the input value. Hence we just keep two_exp = 0.

    // FIXME: call arithmetic::mul::generate()
}

fn generate_shr<F: RichField>(lv: &mut CpuColumnsView<F>) {
    if lv.op.shr == F::ZERO {
        return;
    }
    assert!(lv.op.shr == F::ONE);

    todo!();
}

pub fn generate<F: RichField>(lv: &mut CpuColumnsView<F>) {
    if lv.is_cpu_cycle == F::ZERO {
        return;
    }
    assert_eq!(lv.is_cpu_cycle, F::ONE);

    generate_shl(lv);
    generate_shr(lv);
}

fn eval_packed_shl<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_shl = lv.op.shl;
    let val = lv.mem_channels[0].value;
    let shift = lv.mem_channels[1];    // holds the shift displacement d
    let two_exp = lv.mem_channels[2];  // holds 2^d

    let shift_table_segment = P::Scalar::from_canonical_u64(Segment::ShiftTable as u64);

    // Constrain two_exp mem_channel to be shift table lookup
    yield_constr.constraint(is_shl * two_exp.addr_context); // kernel mode only
    yield_constr.constraint(is_shl * (two_exp.addr_segment - shift_table_segment));
    yield_constr.constraint(is_shl * (two_exp.addr_virtual - shift.value[0]));

    //let tail_limbs_sum: P = shift.value[1..].iter().sum(); // TODO: Why this no work?
    let tail_limbs_sum: P = shift.value[1..].iter().map(|&x| x).sum();
    // FIXME: We need to handle this being non-zero
    yield_constr.constraint(is_shl * tail_limbs_sum);

    // arithmetic::mul::eval_packed_generic(...)

    todo!();
}

fn eval_packed_shr<P: PackedField>(
    _lv: &CpuColumnsView<P>,
    _yield_constr: &mut ConstraintConsumer<P>,
) {
    todo!();
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_shl(lv, yield_constr);
    eval_packed_shr(lv, yield_constr);
}

fn eval_ext_circuit_shl<F: RichField + Extendable<D>, const D: usize>(
    _builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    _lv: &CpuColumnsView<ExtensionTarget<D>>,
    _yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    todo!();
}

fn eval_ext_circuit_shr<F: RichField + Extendable<D>, const D: usize>(
    _builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    _lv: &CpuColumnsView<ExtensionTarget<D>>,
    _yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    todo!();
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_shl(builder, lv, yield_constr);
    eval_ext_circuit_shr(builder, lv, yield_constr);
}
