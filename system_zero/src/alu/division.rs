use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

/// Division instruction of a u32 divisor N into a u32 dividend D,
/// with u32 quotient Q and u32 remainder R. If D is not zero, then
/// the values will satisfy N = Q*D + R with 0 <= R < D.  If D is
/// zero, then the remainder is set to the special value u32::MAX =
/// 2^32 - 1 (which is not a valid remainder for any nonzero D) and
/// the quotient is set to zero.  In particular, no overflow is
/// possible.

pub(crate) fn generate_division<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    let dividend = values[COL_DIV_INPUT_DIVIDEND].to_canonical_u64() as u32;
    let divisor = values[COL_DIV_INPUT_DIVISOR].to_canonical_u64() as u32;

    // `COL_DIV_INVDIVISOR` is `divisor^-1` if `divisor != 0` and `0` otherwise.
    // `COL_DIV_NONZERO_DIVISOR` is `1` if `divisor != 0` and `0` otherwise.

    // `COL_DIV_RANGE_CHECKED_TMP` is set to `divisor - rem - 1` if `divisor != 0` and `0`
    // otherwise. This is used to ensure that `rem < divisor` when `divisor != 0`.

    if divisor == 0 {
        // Outputs
        values[COL_DIV_OUTPUT_QUOT_0] = F::ZERO;
        values[COL_DIV_OUTPUT_QUOT_1] = F::ZERO;
        values[COL_DIV_OUTPUT_REM_0] = F::from_canonical_u16(u16::MAX);
        values[COL_DIV_OUTPUT_REM_1] = F::from_canonical_u16(u16::MAX);

        // Temporaries
        values[COL_DIV_RANGE_CHECKED_TMP_0] = F::ZERO;
        values[COL_DIV_RANGE_CHECKED_TMP_1] = F::ZERO;
        values[COL_DIV_INVDIVISOR] = F::ZERO;
        values[COL_DIV_NONZERO_DIVISOR] = F::ZERO;
    } else {
        let quo = dividend / divisor;
        let rem = dividend % divisor;

        let div_rem_diff_m1 = divisor - rem - 1;

        // Outputs
        values[COL_DIV_OUTPUT_QUOT_0] = F::from_canonical_u16(quo as u16);
        values[COL_DIV_OUTPUT_QUOT_1] = F::from_canonical_u16((quo >> 16) as u16);
        values[COL_DIV_OUTPUT_REM_0] = F::from_canonical_u16(rem as u16);
        values[COL_DIV_OUTPUT_REM_1] = F::from_canonical_u16((rem >> 16) as u16);

        // Temporaries
        values[COL_DIV_RANGE_CHECKED_TMP_0] = F::from_canonical_u16(div_rem_diff_m1 as u16);
        values[COL_DIV_RANGE_CHECKED_TMP_1] = F::from_canonical_u16((div_rem_diff_m1 >> 16) as u16);
        values[COL_DIV_INVDIVISOR] = F::from_canonical_u32(divisor).inverse();
        values[COL_DIV_NONZERO_DIVISOR] = F::ONE;
    }
}

pub(crate) fn eval_division<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let base = F::from_canonical_u64(1 << 16);
    let u32_max = P::from(F::from_canonical_u32(u32::MAX));

    // Filter
    let is_div = lv[IS_DIV];

    // Inputs
    let dividend = lv[COL_DIV_INPUT_DIVIDEND];
    let divisor = lv[COL_DIV_INPUT_DIVISOR];

    // Outputs
    let quotient = lv[COL_DIV_OUTPUT_QUOT_0] + lv[COL_DIV_OUTPUT_QUOT_1] * base;
    let remainder = lv[COL_DIV_OUTPUT_REM_0] + lv[COL_DIV_OUTPUT_REM_1] * base;

    // Temporaries
    let divinv = lv[COL_DIV_INVDIVISOR];
    let div_divinv = lv[COL_DIV_NONZERO_DIVISOR];
    let div_rem_diff_m1 = lv[COL_DIV_RANGE_CHECKED_TMP_0] + lv[COL_DIV_RANGE_CHECKED_TMP_1] * base;

    // Constraints
    yield_constr.constraint(is_div * (divisor * divinv - div_divinv));
    yield_constr.constraint(is_div * (div_divinv - F::ONE) * (remainder - quotient - u32_max));
    yield_constr.constraint(is_div * divisor * (div_divinv - F::ONE));
    yield_constr.constraint(is_div * (quotient + remainder * divinv - divinv * dividend));
    yield_constr.constraint(is_div * divisor * (divisor - remainder - F::ONE - div_rem_diff_m1));
}

pub(crate) fn eval_division_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let base = builder.constant_extension(F::Extension::from_canonical_u64(1 << 16));
    let u32_max = builder.constant_extension(F::Extension::from_canonical_u32(u32::MAX));
    let one = builder.constant_extension(F::Extension::ONE);

    // Filter
    let is_div = lv[IS_DIV];

    // Inputs
    let dividend = lv[COL_DIV_INPUT_DIVIDEND];
    let divisor = lv[COL_DIV_INPUT_DIVISOR];

    // Outputs
    let quotient =
        builder.mul_add_extension(lv[COL_DIV_OUTPUT_QUOT_1], base, lv[COL_DIV_OUTPUT_QUOT_0]);
    let remainder =
        builder.mul_add_extension(lv[COL_DIV_OUTPUT_REM_1], base, lv[COL_DIV_OUTPUT_REM_0]);

    // Temporaries
    let divinv = lv[COL_DIV_INVDIVISOR];
    let div_divinv = lv[COL_DIV_NONZERO_DIVISOR];
    let div_rem_diff_m1 = builder.mul_add_extension(
        lv[COL_DIV_RANGE_CHECKED_TMP_1],
        base,
        lv[COL_DIV_RANGE_CHECKED_TMP_0],
    );

    // Constraints
    let constr6 = builder.mul_sub_extension(divisor, divinv, div_divinv);
    let constr7 = {
        let t = builder.sub_extension(div_divinv, one);
        let u = builder.sub_extension(remainder, quotient);
        let v = builder.sub_extension(u, u32_max);
        builder.mul_extension(t, v)
    };
    let constr8 = {
        let t = builder.sub_extension(div_divinv, one);
        builder.mul_extension(divisor, t)
    };
    let constr9 = {
        let t = builder.sub_extension(remainder, dividend);
        builder.mul_add_extension(t, divinv, quotient)
    };
    let constr10 = {
        let t = builder.sub_extension(divisor, remainder);
        let u = builder.add_extension(one, div_rem_diff_m1);
        let v = builder.sub_extension(t, u);
        builder.mul_extension(divisor, v)
    };

    let constr6 = builder.mul_extension(is_div, constr6);
    let constr7 = builder.mul_extension(is_div, constr7);
    let constr8 = builder.mul_extension(is_div, constr8);
    let constr9 = builder.mul_extension(is_div, constr9);
    let constr10 = builder.mul_extension(is_div, constr10);

    yield_constr.constraint(builder, constr6);
    yield_constr.constraint(builder, constr7);
    yield_constr.constraint(builder, constr8);
    yield_constr.constraint(builder, constr9);
    yield_constr.constraint(builder, constr10);
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
    fn generate_eval_consistency_not_div() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut values = [F::default(); NUM_COLUMNS].map(|_| F::sample(&mut rng));

        // if `IS_DIV == 0`, then the constraints should be met even if all values are garbage.
        values[IS_DIV] = F::ZERO;

        let mut constrant_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_division(&values, &mut constrant_consumer);
        for &acc in &constrant_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency_div() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut values = [F::default(); NUM_COLUMNS].map(|_| F::sample(&mut rng));

        // set `IS_DIV == 1` and ensure all constraints are satisfied.
        values[IS_DIV] = F::ONE;
        // set `DIVIDEND` and `DIVISOR` to `u32`s
        values[COL_DIV_INPUT_DIVIDEND] = F::from_canonical_u32(rng.gen::<u32>());
        values[COL_DIV_INPUT_DIVISOR] = F::from_canonical_u32(rng.gen::<u32>());

        generate_division(&mut values);

        let mut constrant_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_division(&values, &mut constrant_consumer);
        for &acc in &constrant_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    // TODO: test eval_division_recursively.
}
