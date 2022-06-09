use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::alu::columns;
use crate::alu::utils;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ALU_COLUMNS]) {
    let input0_limbs = columns::ADD_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs = columns::ADD_INPUT_1.map(|c| lv[c].to_canonical_u64());

    // Output has 16-bit limbs, so twice as many limbs as the input
    let mut output_limbs = [0u16; columns::N_LIMBS_16];

    let cy = 0u64;
    for (i, &(a, b)) in input0_limbs.zip(input1_limbs).iter().enumerate() {
        let s = a + b + cy;
        let cy = s >> 32;
        debug_assert!(cy <= 1u64, "input limbs were larger than 32 bits");

        output_limbs[2 * i] = s as u16;
        output_limbs[2 * i + 1] = (s >> 16) as u16;
    }
    // last carry is dropped because this is addition modulo 2^256.

    for &(c, output_limb) in columns::ADD_OUTPUT.zip(output_limbs).iter() {
        lv[c] = F::from_canonical_u16(output_limb);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_add = lv[columns::IS_ADD];
    let input0_limbs = columns::ADD_INPUT_0.map(|c| lv[c]);
    let input1_limbs = columns::ADD_INPUT_1.map(|c| lv[c]);
    let output_limbs = columns::ADD_OUTPUT.map(|c| lv[c]);

    // The sums can't overflow because the input limbs and output
    // limbs have been range-checked to be 32 and 16 bits respectively.
    let base = P::Scalar::from_canonical_u64(1 << 16);
    let mut output_received = [P::ZEROS; columns::N_LIMBS_32]; // pointless init
    for i in 0..columns::N_LIMBS_32 {
        let lo = output_limbs[2 * i];
        let hi = output_limbs[2 * i + 1];
        output_received[i] = lo + hi * base;
    }
    // This computed output is not yet reduced; i.e. some limbs may be
    // more than 32 bits.
    let output_computed = input0_limbs.zip(input1_limbs).map(|(a, b)| a + b);

    utils::eval_packed_generic_are_equal(yield_constr, is_add, &output_computed, &output_received);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_add = lv[columns::IS_ADD];
    let input0_limbs = columns::ADD_INPUT_0.map(|c| lv[c]);
    let input1_limbs = columns::ADD_INPUT_1.map(|c| lv[c]);
    let output_limbs = columns::ADD_OUTPUT.map(|c| lv[c]);

    let base = F::from_canonical_u64(1 << 16);
    let zero = builder.zero_extension();
    let mut output_received = [zero; columns::N_LIMBS_32]; // pointless init
    for i in 0..columns::N_LIMBS_32 {
        let lo = output_limbs[2 * i];
        let hi = output_limbs[2 * i + 1];
        output_received[i] = builder.mul_const_add_extension(base, hi, lo);
    }
    let output_computed = input0_limbs
        .zip(input1_limbs)
        .map(|(a, b)| builder.add_extension(a, b));

    utils::eval_ext_circuit_are_equal(
        builder,
        yield_constr,
        is_add,
        &output_computed,
        &output_received,
    );
}
