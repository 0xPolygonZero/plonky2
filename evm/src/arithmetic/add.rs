use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::columns;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

/// NB: Tests for equality, but only on the assumption that the limbs
/// in `larger` are all at least as big as those in `smaller`, and
/// that the limbs in `larger` are at most (LIMB_BITS + 1) bits.
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
    let overflow = P::Scalar::from_canonical_u64(1 << columns::LIMB_BITS);
    let overflow_inv = overflow.inverse();
    let mut cy = P::ZEROS;
    for (a, b) in larger.zip(smaller) {
        // t should be either 0 or 2^LIMB_BITS
        let t = cy + a - b;
        yield_constr.constraint(is_op * t * (overflow - t));
        // cy <-- 0 or 1
        cy = t * overflow_inv;
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
    // 2^LIMB_BITS in the base field
    let overflow_base = F::from_canonical_u64(1 << columns::LIMB_BITS);
    // 2^LIMB_BITS in the extension field as an ExtensionTarget
    let overflow = builder.constant_extension(F::Extension::from(overflow_base));
    // 2^-LIMB_BITS in the base field.
    let overflow_inv = F::inverse_2exp(columns::LIMB_BITS);

    let mut cy = builder.zero_extension();
    for (a, b) in larger.zip(smaller) {
        // t0 = cy + a
        let t0 = builder.add_extension(cy, a);
        // t  = t0 - b
        let t = builder.sub_extension(t0, b);
        // t1 = overflow - t
        let t1 = builder.sub_extension(overflow, t);
        // t2 = t * t1
        let t2 = builder.mul_extension(t, t1);

        let filtered_limb_constraint = builder.mul_extension(is_op, t2);
        yield_constr.constraint(builder, filtered_limb_constraint);

        cy = builder.mul_const_extension(overflow_inv, t);
    }
}

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ARITH_COLUMNS]) {
    let input0_limbs = columns::ADD_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs = columns::ADD_INPUT_1.map(|c| lv[c].to_canonical_u64());

    // Input and output have 16-bit limbs
    let mut output_limbs = [0u64; columns::N_LIMBS];

    const MASK: u64 = (1u64 << columns::LIMB_BITS) - 1u64;
    let cy = 0u64;
    for (i, (&a, &b)) in input0_limbs.iter().zip(input1_limbs.iter()).enumerate() {
        let s = a + b + cy;
        let cy = s >> columns::LIMB_BITS;
        debug_assert!(cy <= 1u64, "input limbs were larger than 16 bits");
        output_limbs[i] = s & MASK;
    }
    // last carry is dropped because this is addition modulo 2^256.

    for (&c, &output_limb) in columns::ADD_OUTPUT.iter().zip(output_limbs.iter()) {
        lv[c] = F::from_canonical_u64(output_limb);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_add = lv[columns::IS_ADD];
    let input0_limbs = columns::ADD_INPUT_0.iter().map(|&c| lv[c]);
    let input1_limbs = columns::ADD_INPUT_1.iter().map(|&c| lv[c]);
    let output_limbs = columns::ADD_OUTPUT.iter().map(|&c| lv[c]);

    // This computed output is not yet reduced; i.e. some limbs may be
    // more than 16 bits.
    let output_computed = input0_limbs.zip(input1_limbs).map(|(a, b)| a + b);

    eval_packed_generic_are_equal(yield_constr, is_add, output_computed, output_limbs);
}

#[allow(clippy::needless_collect)]
pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_add = lv[columns::IS_ADD];
    let input0_limbs = columns::ADD_INPUT_0.iter().map(|&c| lv[c]);
    let input1_limbs = columns::ADD_INPUT_1.iter().map(|&c| lv[c]);
    let output_limbs = columns::ADD_OUTPUT.iter().map(|&c| lv[c]);

    let output_computed = input0_limbs
        .zip(input1_limbs)
        .map(|(a, b)| builder.add_extension(a, b))
        .collect::<Vec<ExtensionTarget<D>>>();

    eval_ext_circuit_are_equal(
        builder,
        yield_constr,
        is_add,
        output_computed.into_iter(),
        output_limbs,
    );
}
