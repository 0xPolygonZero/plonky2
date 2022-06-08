use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::alu::columns;
use crate::alu::utils;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ALU_COLUMNS]) {
    let input0_limbs = columns::SUB_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs = columns::SUB_INPUT_1.map(|c| lv[c].to_canonical_u64());

    // Output has 16-bit limbs, so twice as many limbs as the input
    let mut output_limbs = [0u16; 2 * columns::N_LIMBS];

    const LIMB_BOUNDARY: u64 = 1 << columns::LIMB_BITS;

    let br = 0u64;
    for (i, &(a, b)) in input0_limbs.zip(input1_limbs).iter().enumerate() {
        let d = LIMB_BOUNDARY + a - b - br;
        // if a < b, then d < 2^32 so br = 1
        // if a >= b, then d >= 2^32 so br = 0
        let br = 1u64 - (d >> columns::LIMB_BITS);
        debug_assert!(br <= 1u64, "input limbs were larger than 32 bits");

        debug_assert_eq!(columns::LIMB_BITS, 32, "code assumption violated");
        output_limbs[2 * i] = d as u16;
        output_limbs[2 * i + 1] = (d >> 16) as u16;
    }
    // last borrow is dropped because this is subtraction modulo 2^256.

    for &(c, output_limb) in columns::SUB_OUTPUT.zip(output_limbs).iter() {
        lv[c] = F::from_canonical_u16(output_limb);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_sub = lv[columns::IS_SUB];
    let input0_limbs = columns::SUB_INPUT_0.map(|c| lv[c]);
    let input1_limbs = columns::SUB_INPUT_1.map(|c| lv[c]);
    let output_limbs = columns::SUB_OUTPUT.map(|c| lv[c]);

    // Range checks on the inputs and outputs guarantee that these
    // formulae can't overflow. For the same reason, if they
    // underflow, then they will be non-zero and hence the constraint
    // will fail as expected.
    let base = P::Scalar::from_canonical_u64(1 << 16);
    let limb_boundary = P::Scalar::from_canonical_u64(1 << columns::LIMB_BITS);
    let mut output_received = [P::ZEROS; columns::N_LIMBS]; // pointless init
    for i in 0..columns::N_LIMBS {
        let lo = output_limbs[2 * i];
        let hi = output_limbs[2 * i + 1];
        output_received[i] = lo + hi * base;
    }
    let output_computed = input0_limbs
        .zip(input1_limbs)
        .map(|(a, b)| limb_boundary + a - b);

    utils::eval_packed_generic_are_equal(yield_constr, is_sub, &output_computed, &output_received);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_sub = lv[columns::IS_SUB];
    let input0_limbs = columns::SUB_INPUT_0.map(|c| lv[c]);
    let input1_limbs = columns::SUB_INPUT_1.map(|c| lv[c]);
    let output_limbs = columns::SUB_OUTPUT.map(|c| lv[c]);

    // 2^32 in the base field
    let limb_boundary = F::from_canonical_u64(1 << columns::LIMB_BITS);

    let base = F::from_canonical_u64(1 << 16);
    let zero = builder.zero_extension();
    let mut output_received = [zero; columns::N_LIMBS]; // pointless init
    for i in 0..columns::N_LIMBS {
        let lo = output_limbs[2 * i];
        let hi = output_limbs[2 * i + 1];
        output_received[i] = builder.mul_const_add_extension(base, hi, lo);
    }
    let output_computed = input0_limbs.zip(input1_limbs).map(|(a, b)| {
        let t = builder.add_const_extension(a, limb_boundary);
        builder.sub_extension(t, b)
    });

    utils::eval_ext_circuit_are_equal(
        builder,
        yield_constr,
        is_sub,
        &output_computed,
        &output_received,
    );
}
