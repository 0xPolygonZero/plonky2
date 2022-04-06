use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

const COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS: [usize; 32] = [
    COL_BIT_DECOMP_INPUT_A_LO_00, COL_BIT_DECOMP_INPUT_A_LO_01,
    COL_BIT_DECOMP_INPUT_A_LO_02, COL_BIT_DECOMP_INPUT_A_LO_03,
    COL_BIT_DECOMP_INPUT_A_LO_04, COL_BIT_DECOMP_INPUT_A_LO_05,
    COL_BIT_DECOMP_INPUT_A_LO_06, COL_BIT_DECOMP_INPUT_A_LO_07,
    COL_BIT_DECOMP_INPUT_A_LO_08, COL_BIT_DECOMP_INPUT_A_LO_09,
    COL_BIT_DECOMP_INPUT_A_LO_10, COL_BIT_DECOMP_INPUT_A_LO_11,
    COL_BIT_DECOMP_INPUT_A_LO_12, COL_BIT_DECOMP_INPUT_A_LO_13,
    COL_BIT_DECOMP_INPUT_A_LO_14, COL_BIT_DECOMP_INPUT_A_LO_15,
    COL_BIT_DECOMP_INPUT_A_LO_16, COL_BIT_DECOMP_INPUT_A_LO_17,
    COL_BIT_DECOMP_INPUT_A_LO_18, COL_BIT_DECOMP_INPUT_A_LO_19,
    COL_BIT_DECOMP_INPUT_A_LO_20, COL_BIT_DECOMP_INPUT_A_LO_21,
    COL_BIT_DECOMP_INPUT_A_LO_22, COL_BIT_DECOMP_INPUT_A_LO_23,
    COL_BIT_DECOMP_INPUT_A_LO_24, COL_BIT_DECOMP_INPUT_A_LO_25,
    COL_BIT_DECOMP_INPUT_A_LO_26, COL_BIT_DECOMP_INPUT_A_LO_27,
    COL_BIT_DECOMP_INPUT_A_LO_28, COL_BIT_DECOMP_INPUT_A_LO_29,
    COL_BIT_DECOMP_INPUT_A_LO_30, COL_BIT_DECOMP_INPUT_A_LO_31,
];

const COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS: [usize; 32] = [
    COL_BIT_DECOMP_INPUT_A_HI_00, COL_BIT_DECOMP_INPUT_A_HI_01,
    COL_BIT_DECOMP_INPUT_A_HI_02, COL_BIT_DECOMP_INPUT_A_HI_03,
    COL_BIT_DECOMP_INPUT_A_HI_04, COL_BIT_DECOMP_INPUT_A_HI_05,
    COL_BIT_DECOMP_INPUT_A_HI_06, COL_BIT_DECOMP_INPUT_A_HI_07,
    COL_BIT_DECOMP_INPUT_A_HI_08, COL_BIT_DECOMP_INPUT_A_HI_09,
    COL_BIT_DECOMP_INPUT_A_HI_10, COL_BIT_DECOMP_INPUT_A_HI_11,
    COL_BIT_DECOMP_INPUT_A_HI_12, COL_BIT_DECOMP_INPUT_A_HI_13,
    COL_BIT_DECOMP_INPUT_A_HI_14, COL_BIT_DECOMP_INPUT_A_HI_15,
    COL_BIT_DECOMP_INPUT_A_HI_16, COL_BIT_DECOMP_INPUT_A_HI_17,
    COL_BIT_DECOMP_INPUT_A_HI_18, COL_BIT_DECOMP_INPUT_A_HI_19,
    COL_BIT_DECOMP_INPUT_A_HI_20, COL_BIT_DECOMP_INPUT_A_HI_21,
    COL_BIT_DECOMP_INPUT_A_HI_22, COL_BIT_DECOMP_INPUT_A_HI_23,
    COL_BIT_DECOMP_INPUT_A_HI_24, COL_BIT_DECOMP_INPUT_A_HI_25,
    COL_BIT_DECOMP_INPUT_A_HI_26, COL_BIT_DECOMP_INPUT_A_HI_27,
    COL_BIT_DECOMP_INPUT_A_HI_28, COL_BIT_DECOMP_INPUT_A_HI_29,
    COL_BIT_DECOMP_INPUT_A_HI_30, COL_BIT_DECOMP_INPUT_A_HI_31,
];

const COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS: [usize; 32] = [
    COL_BIT_DECOMP_INPUT_B_LO_00, COL_BIT_DECOMP_INPUT_B_LO_01,
    COL_BIT_DECOMP_INPUT_B_LO_02, COL_BIT_DECOMP_INPUT_B_LO_03,
    COL_BIT_DECOMP_INPUT_B_LO_04, COL_BIT_DECOMP_INPUT_B_LO_05,
    COL_BIT_DECOMP_INPUT_B_LO_06, COL_BIT_DECOMP_INPUT_B_LO_07,
    COL_BIT_DECOMP_INPUT_B_LO_08, COL_BIT_DECOMP_INPUT_B_LO_09,
    COL_BIT_DECOMP_INPUT_B_LO_10, COL_BIT_DECOMP_INPUT_B_LO_11,
    COL_BIT_DECOMP_INPUT_B_LO_12, COL_BIT_DECOMP_INPUT_B_LO_13,
    COL_BIT_DECOMP_INPUT_B_LO_14, COL_BIT_DECOMP_INPUT_B_LO_15,
    COL_BIT_DECOMP_INPUT_B_LO_16, COL_BIT_DECOMP_INPUT_B_LO_17,
    COL_BIT_DECOMP_INPUT_B_LO_18, COL_BIT_DECOMP_INPUT_B_LO_19,
    COL_BIT_DECOMP_INPUT_B_LO_20, COL_BIT_DECOMP_INPUT_B_LO_21,
    COL_BIT_DECOMP_INPUT_B_LO_22, COL_BIT_DECOMP_INPUT_B_LO_23,
    COL_BIT_DECOMP_INPUT_B_LO_24, COL_BIT_DECOMP_INPUT_B_LO_25,
    COL_BIT_DECOMP_INPUT_B_LO_26, COL_BIT_DECOMP_INPUT_B_LO_27,
    COL_BIT_DECOMP_INPUT_B_LO_28, COL_BIT_DECOMP_INPUT_B_LO_29,
    COL_BIT_DECOMP_INPUT_B_LO_30, COL_BIT_DECOMP_INPUT_B_LO_31,
];

const COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS: [usize; 32] = [
    COL_BIT_DECOMP_INPUT_B_HI_00, COL_BIT_DECOMP_INPUT_B_HI_01,
    COL_BIT_DECOMP_INPUT_B_HI_02, COL_BIT_DECOMP_INPUT_B_HI_03,
    COL_BIT_DECOMP_INPUT_B_HI_04, COL_BIT_DECOMP_INPUT_B_HI_05,
    COL_BIT_DECOMP_INPUT_B_HI_06, COL_BIT_DECOMP_INPUT_B_HI_07,
    COL_BIT_DECOMP_INPUT_B_HI_08, COL_BIT_DECOMP_INPUT_B_HI_09,
    COL_BIT_DECOMP_INPUT_B_HI_10, COL_BIT_DECOMP_INPUT_B_HI_11,
    COL_BIT_DECOMP_INPUT_B_HI_12, COL_BIT_DECOMP_INPUT_B_HI_13,
    COL_BIT_DECOMP_INPUT_B_HI_14, COL_BIT_DECOMP_INPUT_B_HI_15,
    COL_BIT_DECOMP_INPUT_B_HI_16, COL_BIT_DECOMP_INPUT_B_HI_17,
    COL_BIT_DECOMP_INPUT_B_HI_18, COL_BIT_DECOMP_INPUT_B_HI_19,
    COL_BIT_DECOMP_INPUT_B_HI_20, COL_BIT_DECOMP_INPUT_B_HI_21,
    COL_BIT_DECOMP_INPUT_B_HI_22, COL_BIT_DECOMP_INPUT_B_HI_23,
    COL_BIT_DECOMP_INPUT_B_HI_24, COL_BIT_DECOMP_INPUT_B_HI_25,
    COL_BIT_DECOMP_INPUT_B_HI_26, COL_BIT_DECOMP_INPUT_B_HI_27,
    COL_BIT_DECOMP_INPUT_B_HI_28, COL_BIT_DECOMP_INPUT_B_HI_29,
    COL_BIT_DECOMP_INPUT_B_HI_30, COL_BIT_DECOMP_INPUT_B_HI_31,
];

/// Interpret the first 32 elements of `bits` as bits from low to high
/// of a u32 and return \sum_i bits[i] 2^i as an element of P.
///
/// TODO: This should probably raise an error if there are more than
/// 32 elements in `bits`.
fn binary_to_u32<F, P, I>(
    bits: I
) -> P
where
    F: Field,
    P: PackedField<Scalar = F>,
    I: IntoIterator<Item = P>,
{
    bits.into_iter()
        .enumerate()
        .take(32)
        .map(|(i, b)| b * F::from_canonical_u64(1u64 << i))
        .sum()
}

fn bitwise_mapreduce<F, P>(
    func: fn(P, P) -> P,
    lhs: [P; 32],
    rhs: [P; 32]
) -> P
where
    F: Field,
    P: PackedField<Scalar = F>,
{
    binary_to_u32(
        lhs.into_iter()
            .zip(rhs.into_iter())
            .map(|(b0, b1)| func(b0, b1)))
}

fn binary_to_u32_recursively<F, const D: usize, I>(
    builder: &mut CircuitBuilder<F, D>,
    bits: I
) -> ExtensionTarget<D>
where
    F: RichField + Extendable<D>,
    I: Iterator<Item = ExtensionTarget<D>>
{
    let terms = bits
        .enumerate()
        .map(|(i, b)|
             builder.mul_const_extension(
                 F::from_canonical_u64(1u64 << i), b))
        .collect::<Vec<_>>();
    builder.add_many_extension(&terms)
}

fn bitwise_mapreduce_recursively<F, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    func: fn(&mut CircuitBuilder<F, D>, ExtensionTarget<D>, ExtensionTarget<D>) -> ExtensionTarget<D>,
    lhs: [ExtensionTarget<D>; 32],
    rhs: [ExtensionTarget<D>; 32]
) -> ExtensionTarget<D>
where
    F: RichField + Extendable<D>
{
    // let terms = lhs
    //     .into_iter()
    //     .zip(rhs.into_iter())
    //     .map(|(b0, b1)| func(builder, b0, b1));
    let mut terms = Vec::with_capacity(32);
    for (b0, b1) in lhs.into_iter().zip(rhs.into_iter()) {
        terms.push(func(builder, b0, b1));
    }
    binary_to_u32_recursively(builder, terms.into_iter())
}

fn generate_bitop<F: PrimeField64>(
    bitop: fn(u32, u32) -> u32,
    values: &mut [F; NUM_COLUMNS]
) {
    // Inputs A and B, each as two digits in base 2^32
    let input_a_lo_bits = COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS.map(|r| values[r]);
    let input_a_hi_bits = COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS.map(|r| values[r]);
    let input_b_lo_bits = COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS.map(|r| values[r]);
    let input_b_hi_bits = COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS.map(|r| values[r]);

    let in_a_lo = binary_to_u32(input_a_lo_bits).to_canonical_u64() as u32;
    let in_b_lo = binary_to_u32(input_b_lo_bits).to_canonical_u64() as u32;
    let in_a_hi = binary_to_u32(input_a_hi_bits).to_canonical_u64() as u32;
    let in_b_hi = binary_to_u32(input_b_hi_bits).to_canonical_u64() as u32;

    let out_lo = bitop(in_a_lo, in_b_lo);
    let out_hi = bitop(in_a_hi, in_b_hi);

    // Output in base 2^16.
    values[COL_BITAND_OUTPUT_0] = F::from_canonical_u16(out_lo as u16);
    values[COL_BITAND_OUTPUT_1] = F::from_canonical_u16((out_lo >> 16) as u16);
    values[COL_BITAND_OUTPUT_2] = F::from_canonical_u16(out_hi as u16);
    values[COL_BITAND_OUTPUT_3] = F::from_canonical_u16((out_hi >> 16) as u16);
}

fn eval_bitop<F: Field, P: PackedField<Scalar = F>>(
    bitop: fn(P, P) -> P,
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // Filter
    let is_bitand = lv[IS_BITAND];

    // Inputs
    let input_a_lo_bits = COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS.map(|r| lv[r]);
    let input_a_hi_bits = COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS.map(|r| lv[r]);
    let input_b_lo_bits = COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS.map(|r| lv[r]);
    let input_b_hi_bits = COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS.map(|r| lv[r]);

    // Output
    let base = F::from_canonical_u64(1 << 16);
    let output_lo = lv[COL_BITAND_OUTPUT_0] + lv[COL_BITAND_OUTPUT_1] * base;
    let output_hi = lv[COL_BITAND_OUTPUT_2] + lv[COL_BITAND_OUTPUT_3] * base;

    let output_lo_expected = bitwise_mapreduce(
        bitop, input_a_lo_bits, input_b_lo_bits);
    yield_constr.constraint(is_bitand * (output_lo - output_lo_expected));

    let output_hi_expected = bitwise_mapreduce(
        bitop, input_a_hi_bits, input_b_hi_bits);
    yield_constr.constraint(is_bitand * (output_hi - output_hi_expected));
}

fn eval_bitop_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    bitop: fn(&mut CircuitBuilder<F, D>, ExtensionTarget<D>, ExtensionTarget<D>) -> ExtensionTarget<D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // Filter
    let is_bitand = lv[IS_BITAND];

    // Inputs
    let input_a_lo_bits = COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS.map(|r| lv[r]);
    let input_a_hi_bits = COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS.map(|r| lv[r]);
    let input_b_lo_bits = COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS.map(|r| lv[r]);
    let input_b_hi_bits = COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS.map(|r| lv[r]);

    // Output
    let base = builder.constant_extension(F::Extension::from_canonical_u64(1 << 16));
    let output_lo = builder.mul_add_extension(
        lv[COL_BITAND_OUTPUT_1], base, lv[COL_BITAND_OUTPUT_0]);
    let output_hi = builder.mul_add_extension(
        lv[COL_BITAND_OUTPUT_3], base, lv[COL_BITAND_OUTPUT_2]);

    let output_lo_expected = bitwise_mapreduce_recursively(
        builder, bitop, input_a_lo_bits, input_b_lo_bits);

    let tmp = builder.sub_extension(output_lo, output_lo_expected);
    let out_lo_constr = builder.mul_extension(is_bitand, tmp);
    yield_constr.constraint(builder, out_lo_constr);

    let output_hi_expected = bitwise_mapreduce_recursively(
        builder, bitop, input_a_hi_bits, input_b_hi_bits);
    let tmp = builder.sub_extension(output_hi, output_hi_expected);
    let out_hi_constr = builder.mul_extension(is_bitand, tmp);
    yield_constr.constraint(builder, out_hi_constr);
}

pub(crate) fn generate_bitand<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    generate_bitop(bitand_raw, values);
}

pub(crate) fn eval_bitand<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_bitop(bitand_packed, lv, yield_constr);
}

pub(crate) fn eval_bitand_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_bitop_recursively(builder, bitand_recursively, lv, yield_constr);
}

fn bitand_raw(x: u32, y: u32) -> u32
{
    x & y
}

fn bitand_packed<F, P>(b0: P, b1: P) -> P
where
    F: Field,
    P: PackedField<Scalar = F>
{
    b0 * b1
}

fn bitand_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    b0: ExtensionTarget<D>,
    b1: ExtensionTarget<D>
) -> ExtensionTarget<D>
{
    builder.mul_extension(b0, b1)
}

fn bitior<F, P>(b0: P, b1: P) -> P
where
    F: Field,
    P: PackedField<Scalar = F>
{
    b0 + b1 - b0 * b1
}

fn bitior_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    b0: ExtensionTarget<D>,
    b1: ExtensionTarget<D>
) -> ExtensionTarget<D>
{
    builder.sub_extension(
        builder.add_extension(b0, b1),
        builder.mul_extension(b0, b1))
}

fn bitxor<F, P>(b0: P, b1: P) -> P
where
    F: Field,
    P: PackedField<Scalar = F>
{
    b0 + b1 - b0 * b1 * F::TWO
}

fn bitxor_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    b0: ExtensionTarget<D>,
    b1: ExtensionTarget<D>
) -> ExtensionTarget<D>
{
    builder.sub_extension(
        builder.add_extension(b0, b1),
        builder.mul_const_extension(
            F::TWO,
            builder.mul_extension(b0, b1)))
}

fn bitandnot<F, P>(b0: P, b1: P) -> P
where
    F: Field,
    P: PackedField<Scalar = F>
{
    b0 * (P::ONES - b1)
}

fn bitandnot_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    b0: ExtensionTarget<D>,
    b1: ExtensionTarget<D>
) -> ExtensionTarget<D>
{
    builder.mul_extension(
        b0,
        builder.sub_extension(
            builder.one_extension(),
            b1))
}

fn bitnot<F, P>(b: P) -> P
where
    F: Field,
    P: PackedField<Scalar = F>
{
    P::ONES - b
}

fn bitnot_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    b: ExtensionTarget<D>,
) -> ExtensionTarget<D>
{
    builder.sub_extension(
        builder.one_extension(),
        b)
}
