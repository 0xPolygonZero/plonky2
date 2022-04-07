use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

/// Interpret the first 32 elements of `bits` as bits from low to high
/// of a u32 and return \sum_i bits[i] 2^i as an element of P.
fn binary_to_u32<F, P>(bits: [P; 32]) -> P
where
    F: Field,
    P: PackedField<Scalar = F>,
{
    bits.into_iter()
        .enumerate()
        .map(|(i, b)| b * F::from_canonical_u64(1u64 << i))
        .sum()
}

/// As for `binary_to_u32` but uses `builder`.
fn binary_to_u32_recursively<F, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    bits: [ExtensionTarget<D>; 32],
) -> ExtensionTarget<D>
where
    F: RichField + Extendable<D>,
{
    let terms = bits
        .into_iter()
        .enumerate()
        .map(|(i, b)| builder.mul_const_extension(F::from_canonical_u64(1u64 << i), b))
        .collect::<Vec<_>>();
    builder.add_many_extension(&terms)
}

fn generate_bitop_32<F: PrimeField64>(
    values: &mut [F; NUM_COLUMNS],
    bitop: usize,
    input_a_regs: [usize; 32],
    input_b_regs: [usize; 32],
    output_0_reg: usize,
    output_1_reg: usize,
) {
    let a_bits = input_a_regs.map(|r| values[r]);
    let b_bits = input_b_regs.map(|r| values[r]);

    let a = binary_to_u32(a_bits).to_canonical_u64() as u32;
    let b = binary_to_u32(b_bits).to_canonical_u64() as u32;

    let out = match bitop {
        IS_BITAND => a & b,
        IS_BITIOR => a | b,
        IS_BITXOR => a ^ b,
        IS_BITANDNOT => a & !b,
        _ => panic!("unrecognized bitop instruction code"),
    };

    values[output_0_reg] = F::from_canonical_u16(out as u16);
    values[output_1_reg] = F::from_canonical_u16((out >> 16) as u16);
}

/// Use the `COL_BIT_DECOMP_INPUT_[AB]_{LO,HI}_*` registers to read
/// bits from `values`, apply `bitop` to the reconstructed u32's (both
/// lo and hi, for 64 bits total), and write the result to the
/// `COL_BITOP_OUTPUT_*` registers.
pub(crate) fn generate_bitop<F: PrimeField64>(values: &mut [F; NUM_COLUMNS], bitop: usize) {
    generate_bitop_32(
        values,
        bitop,
        COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS,
        COL_BITOP_OUTPUT_0,
        COL_BITOP_OUTPUT_1,
    );
    generate_bitop_32(
        values,
        bitop,
        COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS,
        COL_BITOP_OUTPUT_2,
        COL_BITOP_OUTPUT_3,
    );
}

fn eval_bitop_32<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    input_a_regs: [usize; 32],
    input_b_regs: [usize; 32],
    output_0_reg: usize,
    output_1_reg: usize,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // Filter
    let is_and = lv[IS_BITAND];
    let is_ior = lv[IS_BITIOR];
    let is_xor = lv[IS_BITXOR];
    let is_andnot = lv[IS_BITANDNOT];

    // Inputs
    let a_bits = input_a_regs.map(|r| lv[r]);
    let b_bits = input_b_regs.map(|r| lv[r]);

    // Output
    let base = F::from_canonical_u64(1 << 16);
    let output = lv[output_0_reg] + lv[output_1_reg] * base;

    let a = binary_to_u32(a_bits);
    let b = binary_to_u32(b_bits);
    let a_and_b = binary_to_u32(a_bits.zip(b_bits).map(|(b0, b1)| b0 * b1));

    let constraint = is_and * (a_and_b - output)
        + is_ior * (a + b - a_and_b - output)
        + is_xor * (a + b - a_and_b * F::TWO - output)
        + is_andnot * (a - a_and_b - output);

    yield_constr.constraint(constraint);
}

/// Verify an AND, IOR, XOR, or ANDNOT  instruction.
pub(crate) fn eval_bitop<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_bitop_32(
        lv,
        COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS,
        COL_BITOP_OUTPUT_0,
        COL_BITOP_OUTPUT_1,
        yield_constr,
    );
    eval_bitop_32(
        lv,
        COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS,
        COL_BITOP_OUTPUT_2,
        COL_BITOP_OUTPUT_3,
        yield_constr,
    );
}

/// As for `eval_bitop`, but build with `builder`.
fn eval_bitop_32_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    input_a_regs: [usize; 32],
    input_b_regs: [usize; 32],
    output_0_reg: usize,
    output_1_reg: usize,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // Filter
    let is_and = lv[IS_BITAND];
    let is_ior = lv[IS_BITIOR];
    let is_xor = lv[IS_BITXOR];
    let is_andnot = lv[IS_BITANDNOT];

    // Inputs
    let a_bits = input_a_regs.map(|r| lv[r]);
    let b_bits = input_b_regs.map(|r| lv[r]);

    // Output
    let base = builder.constant_extension(F::Extension::from_canonical_u64(1 << 16));
    let output = builder.mul_add_extension(lv[output_1_reg], base, lv[output_0_reg]);

    let a = binary_to_u32_recursively(builder, a_bits);
    let b = binary_to_u32_recursively(builder, b_bits);
    let a_and_b_bits = a_bits
        .zip(b_bits)
        .map(|(b0, b1)| builder.mul_extension(b0, b1));
    let a_and_b = binary_to_u32_recursively(builder, a_and_b_bits);

    let and_constr = {
        let t = builder.sub_extension(a_and_b, output);
        builder.mul_extension(t, is_and)
    };

    let ior_constr = {
        let t0 = builder.add_extension(a, b);
        let t1 = builder.sub_extension(t0, a_and_b);
        let t2 = builder.sub_extension(t1, output);
        builder.mul_extension(t2, is_ior)
    };

    let xor_constr = {
        let t0 = builder.add_extension(a, b);
        let t1 = builder.mul_const_extension(F::TWO, a_and_b);
        let t2 = builder.sub_extension(t0, t1);
        let t3 = builder.sub_extension(t2, output);
        builder.mul_extension(t3, is_xor)
    };

    let andnot_constr = {
        let t0 = builder.sub_extension(a, a_and_b);
        let t1 = builder.sub_extension(t0, output);
        builder.mul_extension(t1, is_andnot)
    };

    let constr = builder.add_many_extension(&[and_constr, ior_constr, xor_constr, andnot_constr]);
    yield_constr.constraint(builder, constr);
}

/// As for `eval_bitop` but with a builder.
pub(crate) fn eval_bitop_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_bitop_32_recursively(
        builder,
        lv,
        COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS,
        COL_BITOP_OUTPUT_0,
        COL_BITOP_OUTPUT_1,
        yield_constr,
    );
    eval_bitop_32_recursively(
        builder,
        lv,
        COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS,
        COL_BITOP_OUTPUT_2,
        COL_BITOP_OUTPUT_3,
        yield_constr,
    );
}
