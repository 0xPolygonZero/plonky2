use plonky2::field::extension_field::Extendable;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns;

const LIMB_SIZE: usize = 16;

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_CPU_COLUMNS]) {
    let eq_filter = lv[columns::IS_EQ].to_canonical_u64();
    let iszero_filter = lv[columns::IS_ISZERO].to_canonical_u64();
    assert!(eq_filter <= 1);
    assert!(iszero_filter <= 1);
    assert!(eq_filter + iszero_filter <= 1);

    if eq_filter != 1 && iszero_filter != 1 {
        return;
    }

    let diffs = if eq_filter == 1 {
        columns::SIMPLE_LOGIC_INPUT0
            .zip(columns::SIMPLE_LOGIC_INPUT1)
            .map(|(in0_col, in1_col)| {
                let in0 = lv[in0_col];
                let in1 = lv[in1_col];
                assert_eq!(in0.to_canonical_u64() >> LIMB_SIZE, 0);
                assert_eq!(in1.to_canonical_u64() >> LIMB_SIZE, 0);
                let diff = in0 - in1;
                diff.square()
            })
            .sum()
    } else if iszero_filter == 1 {
        columns::SIMPLE_LOGIC_INPUT0.map(|i| lv[i]).sum()
    } else {
        panic!()
    };

    lv[columns::SIMPLE_LOGIC_DIFF] = diffs;
    lv[columns::SIMPLE_LOGIC_DIFF_INV] = diffs.try_inverse().unwrap_or(F::ZERO);

    lv[columns::SIMPLE_LOGIC_OUTPUT.start] = F::from_bool(diffs == F::ZERO);
    for i in columns::SIMPLE_LOGIC_OUTPUT.start + 1..columns::SIMPLE_LOGIC_OUTPUT.end {
        lv[i] = F::ZERO;
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &[P; columns::NUM_CPU_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let eq_filter = lv[columns::IS_EQ];
    let iszero_filter = lv[columns::IS_ISZERO];
    let eq_or_iszero_filter = eq_filter + iszero_filter;

    let ls_bit = lv[columns::SIMPLE_LOGIC_OUTPUT.start];

    // Handle EQ and ISZERO. Most limbs of the output are 0, but the least-significant one is
    // either 0 or 1.
    yield_constr.constraint(eq_or_iszero_filter * ls_bit * (ls_bit - P::ONES));

    for bit_col in columns::SIMPLE_LOGIC_OUTPUT.start + 1..columns::SIMPLE_LOGIC_OUTPUT.end {
        let bit = lv[bit_col];
        yield_constr.constraint(eq_or_iszero_filter * bit);
    }

    // Check SIMPLE_LOGIC_DIFF
    let diffs = lv[columns::SIMPLE_LOGIC_DIFF];
    let diffs_inv = lv[columns::SIMPLE_LOGIC_DIFF_INV];
    {
        let input0_sum: P = columns::SIMPLE_LOGIC_INPUT0.map(|i| lv[i]).sum();
        yield_constr.constraint(iszero_filter * (diffs - input0_sum));

        let sum_squared_diffs: P = columns::SIMPLE_LOGIC_INPUT0
            .zip(columns::SIMPLE_LOGIC_INPUT1)
            .map(|(in0_col, in1_col)| {
                let in0 = lv[in0_col];
                let in1 = lv[in1_col];
                let diff = in0 - in1;
                diff.square()
            })
            .sum();
        yield_constr.constraint(eq_filter * (diffs - sum_squared_diffs));
    }

    // diffs != 0 => ls_bit == 0
    yield_constr.constraint(eq_or_iszero_filter * diffs * ls_bit);
    // ls_bit == 0 => diffs != 0 (we provide a diffs_inv)
    yield_constr.constraint(eq_or_iszero_filter * (diffs * diffs_inv + ls_bit - P::ONES));
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_CPU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let eq_filter = lv[columns::IS_EQ];
    let iszero_filter = lv[columns::IS_ISZERO];
    let eq_or_iszero_filter = builder.add_extension(eq_filter, iszero_filter);

    let ls_bit = lv[columns::SIMPLE_LOGIC_OUTPUT.start];

    // Handle EQ and ISZERO. Most limbs of the output are 0, but the least-significant one is
    // either 0 or 1.
    {
        let constr = builder.mul_sub_extension(ls_bit, ls_bit, ls_bit);
        let constr = builder.mul_extension(eq_or_iszero_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    for bit_col in columns::SIMPLE_LOGIC_OUTPUT.start + 1..columns::SIMPLE_LOGIC_OUTPUT.end {
        let bit = lv[bit_col];
        let constr = builder.mul_extension(eq_or_iszero_filter, bit);
        yield_constr.constraint(builder, constr);
    }

    // Check SIMPLE_LOGIC_DIFF
    let diffs = lv[columns::SIMPLE_LOGIC_DIFF];
    let diffs_inv = lv[columns::SIMPLE_LOGIC_DIFF_INV];
    {
        let input0_sum = builder.add_many_extension(columns::SIMPLE_LOGIC_INPUT0.map(|i| lv[i]));
        {
            let constr = builder.sub_extension(diffs, input0_sum);
            let constr = builder.mul_extension(iszero_filter, constr);
            yield_constr.constraint(builder, constr);
        }

        let sum_squared_diffs = columns::SIMPLE_LOGIC_INPUT0
            .zip(columns::SIMPLE_LOGIC_INPUT1)
            .fold(builder.zero_extension(), |acc, (in0_col, in1_col)| {
                let in0 = lv[in0_col];
                let in1 = lv[in1_col];
                let diff = builder.sub_extension(in0, in1);
                builder.mul_add_extension(diff, diff, acc)
            });
        {
            let constr = builder.sub_extension(diffs, sum_squared_diffs);
            let constr = builder.mul_extension(eq_filter, constr);
            yield_constr.constraint(builder, constr);
        }
    }

    {
        // diffs != 0 => ls_bit == 0
        let constr = builder.mul_extension(diffs, ls_bit);
        let constr = builder.mul_extension(eq_or_iszero_filter, constr);
        yield_constr.constraint(builder, constr);
    }
    {
        // ls_bit == 0 => diffs != 0 (we provide a diffs_inv)
        let constr = builder.mul_add_extension(diffs, diffs_inv, ls_bit);
        let constr = builder.mul_sub_extension(eq_or_iszero_filter, constr, eq_or_iszero_filter);
        yield_constr.constraint(builder, constr);
    }
}
