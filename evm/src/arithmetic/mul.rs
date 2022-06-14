use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::columns;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

#[allow(clippy::needless_range_loop)]
pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ARITH_COLUMNS]) {
    let input0_limbs = columns::MUL_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs = columns::MUL_INPUT_1.map(|c| lv[c].to_canonical_u64());

    const MASK: u64 = (1u64 << columns::LIMB_BITS) - 1u64;

    // Input and output have 16-bit limbs
    let mut aux_in_limbs = [0u64; columns::N_LIMBS];
    let mut output_limbs = [0u64; columns::N_LIMBS];

    // Column-wise pen-and-paper long multiplication on 16-bit limbs.
    // We have heaps of space at the top of each limb, so by
    // calculating column-wise (instead of the usual row-wise) we
    // avoid a bunch of carry propagation handling (at the expense of
    // slightly worse cache coherency).
    let mut cy = 0u64;
    for col in 0..columns::N_LIMBS {
        for i in 0..col {
            // Invariant: i + j = col
            let j = col - i;
            let ai_x_bj = input0_limbs[i] * input1_limbs[j];
            aux_in_limbs[col] += ai_x_bj;
        }
        let t = aux_in_limbs[col] + cy;
        cy = t >> columns::LIMB_BITS;
        output_limbs[col] = t & MASK;
    }
    // last cy is dropped because this is multiplication modulo 2^256.

    for (&c, &output_limb) in columns::MUL_OUTPUT.iter().zip(output_limbs.iter()) {
        lv[c] = F::from_canonical_u64(output_limb);
    }
    for deg in 0..columns::N_LIMBS {
        // deg'th element <- a*b - c
        aux_in_limbs[deg] -= output_limbs[deg];
    }
    aux_in_limbs[0] >>= columns::LIMB_BITS;
    for deg in 1..columns::N_LIMBS - 1 {
        aux_in_limbs[deg] = (aux_in_limbs[deg] - aux_in_limbs[deg - 1]) >> columns::LIMB_BITS;
    }
    // Can ignore the last element of aux_in_limbs

    for deg in 0..columns::N_LIMBS - 1 {
        let c = columns::MUL_AUX_INPUT[deg];
        lv[c] = F::from_canonical_u64(aux_in_limbs[deg]);
    }
}

#[allow(clippy::needless_range_loop)]
pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_mul = lv[columns::IS_MUL];
    let input0_limbs = columns::MUL_INPUT_0.map(|c| lv[c]);
    let input1_limbs = columns::MUL_INPUT_1.map(|c| lv[c]);
    let aux_in_limbs = columns::MUL_AUX_INPUT.map(|c| lv[c]);

    // Constraint poly holds the coefficients of the polynomial that
    // must be identically zero for this multiplication to be
    // verified. It is initialised to the /negative/ of the claimed
    // output.
    let mut constr_poly = columns::MUL_OUTPUT.map(|c| -lv[c]);

    debug_assert_eq!(constr_poly.len(), columns::N_LIMBS);

    // Invariant: i + j = deg
    for deg in 0..columns::N_LIMBS {
        for i in 0..deg {
            let j = deg - i;
            constr_poly[deg] += input0_limbs[i] * input1_limbs[j];
        }
    }

    // At this point constr_poly holds the coefficients of the
    // polynomial A(x)B(x) - C(x), where A, B and C are the polynomials
    //
    //   A(x) = \sum_i input0_limbs[i] * 2^LIMB_BITS
    //   B(x) = \sum_i input1_limbs[i] * 2^LIMB_BITS
    //   C(x) = \sum_i output_limbs[i] * 2^LIMB_BITS
    //
    // This polynomial should equal (x - 2^LIMB_BITS) * Q(x) where Q is
    //
    //   Q(x) = \sum_i aux_in_limbs[i] * 2^LIMB_BITS

    // This subtracts (x - 2^LIMB_BITS) * AUX_IN from constr_poly.
    let base = P::Scalar::from_canonical_u64(1 << columns::LIMB_BITS);
    constr_poly[0] += base * aux_in_limbs[0];
    for deg in 1..columns::N_LIMBS - 1 {
        constr_poly[deg] += (base * aux_in_limbs[deg]) - aux_in_limbs[deg - 1];
    }
    constr_poly[columns::N_LIMBS - 1] -= aux_in_limbs[columns::N_LIMBS - 2];

    // At this point constr_poly holds the coefficients of the
    // polynomial A(x)B(x) - C(x) - (x - 2^LIMB_BITS) Q(x). The
    // multiplication is valid if and only if all of those
    // coefficients are zero.
    for &c in &constr_poly {
        yield_constr.constraint(is_mul * c);
    }
}

#[allow(clippy::needless_range_loop)]
pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_mul = lv[columns::IS_MUL];
    let input0_limbs = columns::MUL_INPUT_0.map(|c| lv[c]);
    let input1_limbs = columns::MUL_INPUT_1.map(|c| lv[c]);
    let aux_in_limbs = columns::MUL_AUX_INPUT.map(|c| lv[c]);
    let output_limbs = columns::MUL_OUTPUT.map(|c| lv[c]);

    let zero = builder.zero_extension();
    let mut constr_poly = [zero; columns::N_LIMBS]; // pointless init

    // Invariant: i + j = deg
    for deg in 0..columns::N_LIMBS {
        let mut acc = zero;
        for i in 0..deg {
            let j = deg - i;
            acc = builder.mul_add_extension(input0_limbs[i], input1_limbs[j], acc);
        }
        constr_poly[deg] = builder.sub_extension(acc, output_limbs[deg]);
    }

    let base = F::from_canonical_u64(1 << columns::LIMB_BITS);
    constr_poly[0] = builder.mul_const_add_extension(base, aux_in_limbs[0], constr_poly[0]);
    for deg in 1..columns::N_LIMBS - 1 {
        constr_poly[deg] =
            builder.mul_const_add_extension(base, aux_in_limbs[deg], constr_poly[deg]);
        constr_poly[deg] = builder.sub_extension(constr_poly[deg], aux_in_limbs[deg - 1]);
    }
    constr_poly[columns::N_LIMBS] = builder.sub_extension(
        constr_poly[columns::N_LIMBS],
        aux_in_limbs[columns::N_LIMBS - 1],
    );

    for &c in &constr_poly {
        let filter = builder.mul_extension(is_mul, c);
        yield_constr.constraint(builder, filter);
    }
}
