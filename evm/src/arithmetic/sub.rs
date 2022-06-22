use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::columns::*;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

/// Given two sequences `larger` and `smaller` of equal length (not
/// checked), verifies that \sum_i larger[i] 2^(LIMB_BITS * i) ==
/// \sum_i smaller[i] 2^(LIMB_BITS * i).
///
/// The sequences must have been produced by `eval_packed_generic()`
/// below; specifically, if `a` and `b` are two numbers, then `larger`
/// should be a sequence consisting of 2^LIMB_BITS + a_i - b_i where
/// a_i and b_i are corresponding digits, while `smaller` should be
/// the digits of the reduced value `a - b` after carry propagation
/// has taken place.
fn eval_packed_generic_are_equal<P, I, J>(
    yield_constr: &mut ConstraintConsumer<P>,
    is_op: P,
    larger: I,
    smaller: J,
) where
    P: PackedField,
    I: Iterator<Item = P>,
    J: Iterator<Item = P>,
{
    let mut br = P::ZEROS;
    for (a, b) in larger.zip(smaller) {
        // t should be either 0 or 1
        let t = a - (b + br);
        yield_constr.constraint(is_op * (t - t * t));
        br = t;
    }
}

fn eval_ext_circuit_are_equal<F, const D: usize, I, J>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    is_op: ExtensionTarget<D>,
    larger: I,
    smaller: J,
) where
    F: RichField + Extendable<D>,
    I: Iterator<Item = ExtensionTarget<D>>,
    J: Iterator<Item = ExtensionTarget<D>>,
{
    let mut br = builder.zero_extension();
    for (a, b) in larger.zip(smaller) {
        // t0 = b + br
        let t0 = builder.add_extension(b, br);
        // t  = a - t0
        let t = builder.sub_extension(a, t0);
        // t1 = t * t
        let t1 = builder.mul_extension(t, t);
        // t2 = t1 - t
        let t2 = builder.sub_extension(t1, t);

        let filtered_limb_constraint = builder.mul_extension(is_op, t2);
        yield_constr.constraint(builder, filtered_limb_constraint);

        br = t;
    }
}

pub fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS]) {
    let input0_limbs = SUB_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs = SUB_INPUT_1.map(|c| lv[c].to_canonical_u64());

    // Input and output have 16-bit limbs
    let mut output_limbs = [0u64; N_LIMBS];

    const LIMB_BOUNDARY: u64 = 1 << LIMB_BITS;
    const MASK: u64 = LIMB_BOUNDARY - 1u64;

    let br = 0u64;
    for (i, (&a, &b)) in input0_limbs.iter().zip(input1_limbs.iter()).enumerate() {
        let d = LIMB_BOUNDARY + a - b - br;
        // if a < b, then d < 2^16 so br = 1
        // if a >= b, then d >= 2^16 so br = 0
        let br = 1u64 - (d >> LIMB_BITS);
        debug_assert!(br <= 1u64, "input limbs were larger than 16 bits");
        output_limbs[i] = d & MASK;
    }
    // last borrow is dropped because this is subtraction modulo 2^256.

    for (&c, &output_limb) in SUB_OUTPUT.iter().zip(output_limbs.iter()) {
        lv[c] = F::from_canonical_u64(output_limb);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    range_check_error!(SUB_INPUT_0, 16);
    range_check_error!(SUB_INPUT_1, 16);
    range_check_error!(SUB_OUTPUT, 16);

    let is_sub = lv[IS_SUB];
    let input0_limbs = SUB_INPUT_0.iter().map(|&c| lv[c]);
    let input1_limbs = SUB_INPUT_1.iter().map(|&c| lv[c]);
    let output_limbs = SUB_OUTPUT.iter().map(|&c| lv[c]);

    let limb_boundary = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    let output_computed = input0_limbs
        .zip(input1_limbs)
        .map(|(a, b)| limb_boundary + a - b);

    eval_packed_generic_are_equal(yield_constr, is_sub, output_computed, output_limbs);
}

#[allow(clippy::needless_collect)]
pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_sub = lv[IS_SUB];
    let input0_limbs = SUB_INPUT_0.iter().map(|&c| lv[c]);
    let input1_limbs = SUB_INPUT_1.iter().map(|&c| lv[c]);
    let output_limbs = SUB_OUTPUT.iter().map(|&c| lv[c]);

    // 2^16 in the base field
    let limb_boundary = F::from_canonical_u64(1 << LIMB_BITS);

    let output_computed = input0_limbs
        .zip(input1_limbs)
        .map(|(a, b)| {
            let t = builder.add_const_extension(a, limb_boundary);
            builder.sub_extension(t, b)
        })
        .collect::<Vec<ExtensionTarget<D>>>();

    eval_ext_circuit_are_equal(
        builder,
        yield_constr,
        is_sub,
        output_computed.into_iter(),
        output_limbs,
    );
}
