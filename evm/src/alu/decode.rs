use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::alu::columns;

// TODO: Give a name to the number of 32-bit and 16-bit limbs in a
// 256-bit number and replace all the magic numbers.

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ALU_COLUMNS]) {
    let input0_limbs = columns::ADD_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs = columns::ADD_INPUT_1.map(|c| lv[c].to_canonical_u64());
    debug_assert_eq!(input0_limbs.len(), input1_limbs.len(),
                     "internal error: inputs have different number of limbs");
    // Input given as 32-bit limbs, so need 8 of them to make a 256-bit value.
    debug_assert_eq!(input0_limbs.len(), 8,
                     "internal error: inputs have wrong number of limbs");

    // Output has 16-bit limbs, so twice as many limbs as the input
    let mut output_lo_limbs = [0u16; 8];
    let mut output_hi_limbs = [0u16; 8];

    let cy = 0u64;
    for (i, &(a, b)) in input0_limbs.zip(input1_limbs).iter().enumerate() {
        let s = a + b + cy;
        let cy = s >> 32;
        debug_assert!(cy <= 1u64, "input limbs were larger than 32 bits");

        output_lo_limbs[i] = s as u16;
        output_hi_limbs[i] = (s >> 16) as u16;
    }
    // last carry is dropped because this is addition modulo 2^256.

    for &(c, output_lo_limb) in columns::ADD_OUTPUT_LO.zip(output_lo_limbs).iter() {
        lv[c] = F::from_canonical_u16(output_lo_limb);
    }
    for &(c, output_hi_limb) in columns::ADD_OUTPUT_HI.zip(output_hi_limbs).iter() {
        lv[c] = F::from_canonical_u16(output_hi_limb);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_add = lv[columns::IS_ADD];
    let input0_limbs = columns::ADD_INPUT_0.map(|c| lv[c]);
    let input1_limbs = columns::ADD_INPUT_1.map(|c| lv[c]);
    let output_lo_limbs = columns::ADD_OUTPUT_LO.map(|c| lv[c]);
    let output_hi_limbs = columns::ADD_OUTPUT_HI.map(|c| lv[c]);

    // The sums can't overflow because the input limbs and output
    // limbs have been range-checked to be 32 and 16 bits respectively.
    let base = P::Scalar::from_canonical_u64(1 << 16);
    let output_received = output_lo_limbs.zip(output_hi_limbs).map(|(lo, hi)| lo + hi*base);
    let output_computed = input0_limbs.zip(input1_limbs).map(|(a, b)| a + b);

    for &(out_r, out_c) in output_received.zip(output_computed).iter() {
        yield_constr.constraint(is_add * (out_r - out_c));
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_add = lv[columns::IS_ADD];
    let input0_limbs = columns::ADD_INPUT_0.map(|c| lv[c]);
    let input1_limbs = columns::ADD_INPUT_1.map(|c| lv[c]);
    let output_lo_limbs = columns::ADD_OUTPUT_LO.map(|c| lv[c]);
    let output_hi_limbs = columns::ADD_OUTPUT_HI.map(|c| lv[c]);

    let base = F::from_canonical_u64(1 << 16);
    let output_received = output_lo_limbs
        .zip(output_hi_limbs)
        .map(|(lo, hi)| builder.mul_const_add_extension(base, hi, lo));
    let output_computed = input0_limbs
        .zip(input1_limbs)
        .map(|(a, b)| builder.add_extension(a, b));

    for &(out_r, out_c) in output_received.zip(output_computed).iter() {
        let diff = builder.sub_extension(out_r, out_c);
        let filtered_diff = builder.mul_extension(is_add, diff);
        yield_constr.constraint(builder, filtered_diff);
    }
}
