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

    // Input and output have 16-bit limbs
    let mut output_limbs = [0u64; columns::N_LIMBS];

    const LIMB_BOUNDARY: u64 = 1 << columns::LIMB_BITS;
    const MASK: u64 = LIMB_BOUNDARY - 1u64;

    let br = 0u64;
    for (i, (&a, &b)) in input0_limbs.iter().zip(input1_limbs.iter()).enumerate() {
        let d = LIMB_BOUNDARY + a - b - br;
        // if a < b, then d < 2^16 so br = 1
        // if a >= b, then d >= 2^16 so br = 0
        let br = 1u64 - (d >> columns::LIMB_BITS);
        debug_assert!(br <= 1u64, "input limbs were larger than 16 bits");
        output_limbs[i] = d & MASK;
    }
    // last borrow is dropped because this is subtraction modulo 2^256.

    for (&c, &output_limb) in columns::SUB_OUTPUT.iter().zip(output_limbs.iter()) {
        lv[c] = F::from_canonical_u64(output_limb);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_sub = lv[columns::IS_SUB];
    let input0_limbs = columns::SUB_INPUT_0.iter().map(|&c| lv[c]);
    let input1_limbs = columns::SUB_INPUT_1.iter().map(|&c| lv[c]);
    let output_limbs = columns::SUB_OUTPUT.iter().map(|&c| lv[c]);

    let limb_boundary = P::Scalar::from_canonical_u64(1 << columns::LIMB_BITS);
    let output_computed = input0_limbs
        .zip(input1_limbs)
        .map(|(a, b)| limb_boundary + a - b);

    utils::eval_packed_generic_are_equal(yield_constr, is_sub, output_computed, output_limbs);
}

#[allow(clippy::needless_collect)]
pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_sub = lv[columns::IS_SUB];
    let input0_limbs = columns::SUB_INPUT_0.iter().map(|&c| lv[c]);
    let input1_limbs = columns::SUB_INPUT_1.iter().map(|&c| lv[c]);
    let output_limbs = columns::SUB_OUTPUT.iter().map(|&c| lv[c]);

    // 2^16 in the base field
    let limb_boundary = F::from_canonical_u64(1 << columns::LIMB_BITS);

    let output_computed = input0_limbs
        .zip(input1_limbs)
        .map(|(a, b)| {
            let t = builder.add_const_extension(a, limb_boundary);
            builder.sub_extension(t, b)
        })
        .collect::<Vec<ExtensionTarget<D>>>();

    utils::eval_ext_circuit_are_equal(
        builder,
        yield_constr,
        is_sub,
        output_computed.into_iter(),
        output_limbs,
    );
}
