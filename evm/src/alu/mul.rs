use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::alu::columns;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ALU_COLUMNS]) {
    // NB: multiplication inputs are given as 16-bit limbs, not 32.
    let input0_limbs16 = columns::MUL_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs16 = columns::MUL_INPUT_1.map(|c| lv[c].to_canonical_u64());
    debug_assert_eq!(input0_limbs16.len(), columns::N_LIMBS_16);

    // Create the 32-bit limbed equivalent inputs
    let mut input0_limbs = [0u64; columns::N_LIMBS_32];
    let mut input1_limbs = [0u64; columns::N_LIMBS_32];
    for i in 0..columns::N_LIMBS_32 {
        input0_limbs[i] = input0_limbs16[2 * i] + (input0_limbs[2 * i + 1] << 16);
        input1_limbs[i] = input1_limbs16[2 * i] + (input1_limbs[2 * i + 1] << 16);
    }

    // Output has 16-bit limbs, same as the input
    let mut output_limbs = [0u16; columns::N_LIMBS_16];

    // Column-wise pen-and-paper long multiplication on 32-bit limbs.
    // We have heaps of space at the top of each limb, so by
    // calculating column-wise (instead of the usual row-wise) we
    // avoid a bunch of carry propagation handling (at the expense of
    // slightly worse cache coherency).
    let mut acc_col_hi = 0u64;
    for col in 0..columns::N_LIMBS_32 {
        let mut acc_col_lo = acc_col_hi;
        acc_col_hi = 0u64;
        for i in 0..col {
            // Invariant: i + j = col
            let j = col - i;
            let p = input0_limbs[i] * input1_limbs[j];
            acc_col_lo += (p as u32) as u64;
            acc_col_hi += (p >> 32) as u64;
        }
        acc_col_hi += acc_col_lo >> 32;
        output_limbs[2 * col] = acc_col_lo as u16;
        output_limbs[2 * col + 1] = (acc_col_lo >> 16) as u16;
    }
    // last acc_col_hi is dropped because this is multiplication modulo 2^256.

    for &(c, output_limb) in columns::MUL_OUTPUT.zip(output_limbs).iter() {
        lv[c] = F::from_canonical_u16(output_limb);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ALU_COLUMNS],
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

    debug_assert_eq!(constr_poly.len(), columns::N_LIMBS_16);

    // Invariant: i + j = deg
    for deg in 0..columns::N_LIMBS_16 {
        for i in 0..deg {
            let j = deg - i;
            constr_poly[deg] += input0_limbs[i] * input1_limbs[j];
        }
    }

    debug_assert_eq!(aux_in_limbs.len(), columns::N_LIMBS_16 - 1);

    // This subtracts (x - 2^16) * AUX_IN from constr_poly.
    let base = P::Scalar::from_canonical_u64(1 << 16);
    constr_poly[0] += base * aux_in_limbs[0];
    for deg in 1..columns::N_LIMBS_16 - 1 {
        constr_poly[deg] += (base * aux_in_limbs[deg]) - aux_in_limbs[deg - 1];
    }
    constr_poly[columns::N_LIMBS_16 - 1] -= aux_in_limbs[columns::N_LIMBS_16 - 2];

    for &c in &constr_poly {
        yield_constr.constraint(is_mul * c);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_mul = lv[columns::IS_MUL];
    let input0_limbs = columns::MUL_INPUT_0.map(|c| lv[c]);
    let input1_limbs = columns::MUL_INPUT_1.map(|c| lv[c]);
    let aux_in_limbs = columns::MUL_AUX_INPUT.map(|c| lv[c]);
    let output_limbs = columns::MUL_OUTPUT.map(|c| lv[c]);

    let zero = builder.zero_extension();
    let mut constr_poly = [zero; columns::N_LIMBS_16]; // pointless init

    // Invariant: i + j = deg
    for deg in 0..columns::N_LIMBS_16 {
        let mut acc = zero;
        for i in 0..deg {
            let j = deg - i;
            acc = builder.mul_add_extension(input0_limbs[i], input1_limbs[j], acc);
        }
        constr_poly[deg] = builder.sub_extension(acc, output_limbs[deg]);
    }

    let base = F::from_canonical_u64(1 << 16);
    constr_poly[0] = builder.mul_const_add_extension(base, aux_in_limbs[0], constr_poly[0]);
    for deg in 1..columns::N_LIMBS_16 - 1 {
        constr_poly[deg] =
            builder.mul_const_add_extension(base, aux_in_limbs[deg], constr_poly[deg]);
        constr_poly[deg] = builder.sub_extension(constr_poly[deg], aux_in_limbs[deg - 1]);
    }
    constr_poly[columns::N_LIMBS_16] = builder.sub_extension(
        constr_poly[columns::N_LIMBS_16],
        aux_in_limbs[columns::N_LIMBS_16 - 1],
    );

    for &c in &constr_poly {
        let filter = builder.mul_extension(is_mul, c);
        yield_constr.constraint(builder, filter);
    }
}
