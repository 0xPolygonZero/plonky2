use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::plonk_common::reduce_with_powers_ext_circuit;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

/// Interpret the N <= 32 elements of `bits` as bits from low to high of a
/// u32 and return \sum_i bits[i] 2^i as an element of P.
pub(crate) fn binary_to_u32<F, P>(bits: [P; 32]) -> P
where
    F: Field,
    P: PackedField<Scalar = F>,
{
    bits.into_iter()
        .enumerate()
        .map(|(i, b)| b * F::from_canonical_u64(1u64 << i))
        .sum()
}

fn generate_bitop_32<F: PrimeField64>(
    values: &mut [F; NUM_COLUMNS],
    bitop: usize,
    input_a_regs: [usize; 32],
    input_b_regs: [usize; 32],
    output_reg: usize,
) {
    let a_bits = input_a_regs.map(|r| values[r]);
    let b_bits = input_b_regs.map(|r| values[r]);

    let a = binary_to_u32(a_bits).to_canonical_u64() as u32;
    let b = binary_to_u32(b_bits).to_canonical_u64() as u32;

    let out = match bitop {
        IS_AND => a & b,
        IS_IOR => a | b,
        IS_XOR => a ^ b,
        IS_ANDNOT => a & !b,
        _ => panic!("unrecognized bitop instruction code"),
    };

    values[output_reg] = F::from_canonical_u32(out);
}

/// Use the `COL_BIT_DECOMP_INPUT_[AB]_{LO,HI}_*` registers to read
/// bits from `values`, apply `bitop` to the reconstructed u32's (both
/// lo and hi, for 64 bits total), and write the result to the
/// `COL_BITOP_OUTPUT_*` registers.
pub(crate) fn generate_bitop<F: PrimeField64>(values: &mut [F; NUM_COLUMNS], bitop: usize) {
    // Generate lo half
    generate_bitop_32(
        values,
        bitop,
        COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS,
        COL_BITOP_OUTPUT_0,
    );
    // Generate hi half
    generate_bitop_32(
        values,
        bitop,
        COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS,
        COL_BITOP_OUTPUT_1,
    );
}

fn eval_bitop_32<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    input_a_regs: [usize; 32],
    input_b_regs: [usize; 32],
    output_reg: usize,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // Filters
    let is_and = lv[IS_AND];
    let is_ior = lv[IS_IOR];
    let is_xor = lv[IS_XOR];
    let is_andnot = lv[IS_ANDNOT];

    // Inputs
    let a_bits = input_a_regs.map(|r| lv[r]);
    let b_bits = input_b_regs.map(|r| lv[r]);

    // Ensure that the inputs are bits
    let inst_constr = is_and + is_ior + is_xor + is_andnot;
    a_bits.map(|v| yield_constr.constraint(inst_constr * (v * v - v)));
    b_bits.map(|v| yield_constr.constraint(inst_constr * (v * v - v)));

    // Output
    let output = lv[output_reg];

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
    // Constraint for lo half
    eval_bitop_32(
        lv,
        COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS,
        COL_BITOP_OUTPUT_0,
        yield_constr,
    );
    // Constraint for hi half
    eval_bitop_32(
        lv,
        COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS,
        COL_BITOP_OUTPUT_1,
        yield_constr,
    );
}

pub(crate) fn constrain_all_to_bits_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    vals: &[ExtensionTarget<D>],
) {
    for v in vals.iter() {
        let t0 = builder.mul_sub_extension(*v, *v, *v);
        let t1 = builder.mul_extension(filter, t0);
        yield_constr.constraint(builder, t1)
    }
}

/// As for `eval_bitop`, but build with `builder`.
fn eval_bitop_32_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    input_a_regs: [usize; 32],
    input_b_regs: [usize; 32],
    output_reg: usize,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // Filters
    let is_and = lv[IS_AND];
    let is_ior = lv[IS_IOR];
    let is_xor = lv[IS_XOR];
    let is_andnot = lv[IS_ANDNOT];

    // Inputs
    let a_bits = input_a_regs.map(|r| lv[r]);
    let b_bits = input_b_regs.map(|r| lv[r]);

    // Ensure that the inputs are bits
    let inst_constr = builder.add_many_extension([is_and, is_ior, is_xor, is_andnot]);
    constrain_all_to_bits_circuit(builder, yield_constr, inst_constr, &a_bits);
    constrain_all_to_bits_circuit(builder, yield_constr, inst_constr, &b_bits);

    // Output
    let output = lv[output_reg];

    let limb_base = builder.constant(F::TWO);
    let a = reduce_with_powers_ext_circuit(builder, &a_bits, limb_base);
    let b = reduce_with_powers_ext_circuit(builder, &b_bits, limb_base);
    let a_and_b_bits = a_bits
        .zip(b_bits)
        .map(|(b0, b1)| builder.mul_extension(b0, b1));
    let a_and_b = reduce_with_powers_ext_circuit(builder, &a_and_b_bits, limb_base);

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

    let constr = builder.add_many_extension([and_constr, ior_constr, xor_constr, andnot_constr]);
    yield_constr.constraint(builder, constr);
}

/// As for `eval_bitop` but with a builder.
pub(crate) fn eval_bitop_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // Recursive constraint for lo half
    eval_bitop_32_circuit(
        builder,
        lv,
        COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS,
        COL_BITOP_OUTPUT_0,
        yield_constr,
    );
    // Recursive constraint for hi half
    eval_bitop_32_circuit(
        builder,
        lv,
        COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS,
        COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS,
        COL_BITOP_OUTPUT_1,
        yield_constr,
    );
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Sample;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;
    use starky::constraint_consumer::ConstraintConsumer;

    use super::*;
    use crate::registers::NUM_COLUMNS;

    #[test]
    fn generate_eval_consistency_not_bitop() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut values = [F::default(); NUM_COLUMNS].map(|_| F::sample(&mut rng));

        // if `IS_bitop == 0`, then the constraints should be met even
        // if all values are garbage.
        for bitop in [IS_AND, IS_IOR, IS_XOR, IS_ANDNOT] {
            values[bitop] = F::ZERO;
        }

        let mut constrant_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_bitop(&values, &mut constrant_consumer);
        for &acc in &constrant_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency_bitop() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut values = [F::default(); NUM_COLUMNS].map(|_| F::sample(&mut rng));

        const BITOPS: [usize; 4] = [IS_AND, IS_IOR, IS_XOR, IS_ANDNOT];
        for bitop in BITOPS {
            // Reset all the instruction registers
            for op in BITOPS {
                values[op] = F::ZERO;
            }
            // set `IS_bitop == 1` and ensure all constraints are satisfied.
            values[bitop] = F::ONE;

            // Set inputs to random binary values
            let all_bin_regs = COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS
                .into_iter()
                .chain(COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS)
                .chain(COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS)
                .chain(COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS);

            for reg in all_bin_regs {
                values[reg] = F::from_canonical_u32(rng.gen::<u32>() & 1);
            }

            generate_bitop(&mut values, bitop);

            let mut constrant_consumer = ConstraintConsumer::new(
                vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                GoldilocksField::ONE,
                GoldilocksField::ONE,
                GoldilocksField::ONE,
            );
            eval_bitop(&values, &mut constrant_consumer);
            for &acc in &constrant_consumer.constraint_accs {
                assert_eq!(acc, GoldilocksField::ZERO);
            }
        }
    }

    #[test]
    fn generate_eval_consistency_bit_inputs() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut values = [F::default(); NUM_COLUMNS].map(|_| F::sample(&mut rng));

        const BITOPS: [usize; 4] = [IS_AND, IS_IOR, IS_XOR, IS_ANDNOT];
        for bitop in BITOPS {
            // Reset all the instruction registers
            for op in BITOPS {
                values[op] = F::ZERO;
            }
            // set `IS_bitop == 1` and ensure all constraints are satisfied.
            values[bitop] = F::ONE;

            // Set inputs to random binary values
            let all_bin_regs = COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS
                .into_iter()
                .chain(COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS)
                .chain(COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS)
                .chain(COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS);

            for reg in all_bin_regs {
                values[reg] = F::from_canonical_u32(rng.gen::<u32>() & 1);
            }
            // Make first "bit" non-binary.
            values[COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS[0]] = F::TWO;

            generate_bitop(&mut values, bitop);

            let mut constrant_consumer = ConstraintConsumer::new(
                vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                GoldilocksField::ONE,
                GoldilocksField::ONE,
                GoldilocksField::ONE,
            );
            eval_bitop(&values, &mut constrant_consumer);
            for &acc in &constrant_consumer.constraint_accs {
                assert_ne!(acc, GoldilocksField::ZERO);
            }
        }
    }

    // TODO: test eval_division_recursively.
}
