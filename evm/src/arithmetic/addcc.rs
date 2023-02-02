//! Support for EVM LT and GT instructions
//!
//! This crate verifies EVM LT and GT instructions (i.e. for unsigned
//! inputs). The difference between LT and GT is of course just a
//! matter of the order of the inputs. The verification is essentially
//! identical to the SUB instruction: For both SUB and LT we have values
//!
//!   - `input0`
//!   - `input1`
//!   - `difference` (mod 2^256)
//!   - `borrow` (= 0 or 1)
//!
//! satisfying `input0 - input1 = difference + borrow * 2^256`. Where
//! SUB verifies `difference` and ignores `borrow`, LT verifies
//! `borrow` (and uses `difference` as an auxiliary input).

use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::columns::*;
use crate::arithmetic::utils::read_value_u64_limbs;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

fn u256_add_cc(input0: [u64; N_LIMBS], input1: [u64; N_LIMBS]) -> ([u64; N_LIMBS], u64) {
    // Input and output have 16-bit limbs
    let mut output = [0u64; N_LIMBS];

    const MASK: u64 = (1u64 << LIMB_BITS) - 1u64;
    let mut cy = 0u64;
    for (i, a, b) in izip!(0.., input0, input1) {
        let s = a + b + cy;
        cy = s >> LIMB_BITS;
        assert!(cy <= 1u64, "input limbs were larger than 16 bits");
        output[i] = s & MASK;
    }
    (output, cy)
}

fn u256_sub_br(input0: [u64; N_LIMBS], input1: [u64; N_LIMBS]) -> ([u64; N_LIMBS], u64) {
    const LIMB_BOUNDARY: u64 = 1 << LIMB_BITS;
    const MASK: u64 = LIMB_BOUNDARY - 1u64;

    let mut output = [0u64; N_LIMBS];
    let mut br = 0u64;
    for (i, a, b) in izip!(0.., input0, input1) {
        let d = LIMB_BOUNDARY + a - b - br;
        // if a < b, then d < 2^16 so br = 1
        // if a >= b, then d >= 2^16 so br = 0
        br = 1u64 - (d >> LIMB_BITS);
        assert!(br <= 1u64, "input limbs were larger than 16 bits");
        output[i] = d & MASK;
    }

    (output, br)
}

/// Generate row for SUB, GT and LT operations.
///
/// A row consists of four values, GENERAL_REGISTER_[012] and
/// GENERAL_REGISTER_BIT. The interpretation of these values for each
/// operation is as follows:
///
/// ADD: INPUT_0 + INPUT_1, output in INPUT_2, ignore INPUT_BIT
/// SUB: INPUT_0 - INPUT_2, output in INPUT_1, ignore INPUT_BIT
///  GT: INPUT_0 > INPUT_2, output in INPUT_BIT, auxiliary output in INPUT_1
///  LT: INPUT_2 < INPUT_0, output in INPUT_BIT, auxiliary output in INPUT_1
pub(crate) fn generate<F: RichField>(lv: &mut [F], filter: usize) {
    match filter {
        IS_ADD => {
            let x = read_value_u64_limbs(lv, GENERAL_REGISTER_0);
            let y = read_value_u64_limbs(lv, GENERAL_REGISTER_1);

            // x + y == z + cy*2^256
            let (z, cy) = u256_add_cc(x, y);

            lv[GENERAL_REGISTER_2].copy_from_slice(&z.map(F::from_canonical_u64));
            lv[GENERAL_REGISTER_BIT] = F::from_canonical_u64(cy);
        }
        IS_SUB | IS_GT | IS_LT => {
            let x = read_value_u64_limbs(lv, GENERAL_REGISTER_0);
            let z = read_value_u64_limbs(lv, GENERAL_REGISTER_2);

            // y == z - x + cy*2^256
            let (y, cy) = u256_sub_br(z, x);

            lv[GENERAL_REGISTER_1].copy_from_slice(&y.map(F::from_canonical_u64));
            lv[GENERAL_REGISTER_BIT] = F::from_canonical_u64(cy);
        }
        _ => panic!("unexpected operation filter"),
    };
}

/// Given a polynomial f represented by its coefficients `pol_coeffs`
/// and a number n represented by its digits `digits` in base
/// B=2^LIMB_BITS, with `pol_coeffs` and `digits` being of equal
/// length as sequences, this method constrains that f(B) == n, but
/// allowing the possibility of overflow by one bit (which is
/// returned).
fn eval_packed_generic_pol_eval_equal<P, I, J>(
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    pol_coeffs: I,
    digits: J,
    is_two_row_op: bool,
) -> P
where
    P: PackedField,
    I: Iterator<Item = P>,
    J: Iterator<Item = P>,
{
    let overflow = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    let overflow_inv = overflow.inverse();
    let mut cy = P::ZEROS;
    for (a, b) in pol_coeffs.zip(digits) {
        // t should be either 0 or 2^LIMB_BITS
        let t = cy + a - b;
        if is_two_row_op {
            yield_constr.constraint_transition(filter * t * (overflow - t));
        } else {
            yield_constr.constraint(filter * t * (overflow - t));
        }
        // cy <-- 0 or 1
        // NB: this is multiplication by a constant, so doesn't
        // increase the degree of the constraint.
        cy = t * overflow_inv;
    }
    cy
}

fn eval_ext_circuit_pol_eval_equal<F, const D: usize, I, J>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    pol_coeffs: I,
    digits: J,
    is_two_row_op: bool,
) -> ExtensionTarget<D>
where
    F: RichField + Extendable<D>,
    I: Iterator<Item = ExtensionTarget<D>>,
    J: Iterator<Item = ExtensionTarget<D>>,
{
    // 2^LIMB_BITS in the base field
    let overflow_base = F::from_canonical_u64(1 << LIMB_BITS);
    // 2^LIMB_BITS in the extension field as an ExtensionTarget
    let overflow = builder.constant_extension(F::Extension::from(overflow_base));
    // 2^-LIMB_BITS in the base field.
    let overflow_inv = F::inverse_2exp(LIMB_BITS);

    let mut cy = builder.zero_extension();
    for (a, b) in pol_coeffs.zip(digits) {
        // t0 = cy + a
        let t0 = builder.add_extension(cy, a);
        // t  = t0 - b
        let t = builder.sub_extension(t0, b);
        // t1 = overflow - t
        let t1 = builder.sub_extension(overflow, t);
        // t2 = t * t1
        let t2 = builder.mul_extension(t, t1);

        let filtered_limb_constraint = builder.mul_extension(filter, t2);
        if is_two_row_op {
            yield_constr.constraint_transition(builder, filtered_limb_constraint);
        } else {
            yield_constr.constraint(builder, filtered_limb_constraint);
        }

        cy = builder.mul_const_extension(overflow_inv, t);
    }
    cy
}

fn eval_packed_generic_check_is_one_bit<P: PackedField>(
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    x: P,
) {
    yield_constr.constraint(filter * x * (x - P::ONES));
}

fn eval_ext_circuit_check_is_one_bit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    x: ExtensionTarget<D>,
) {
    let constr = builder.mul_sub_extension(x, x, x);
    let filtered_constr = builder.mul_extension(filter, constr);
    yield_constr.constraint(builder, filtered_constr);
}

/// Constrains x + y == z + cy*2^256 assuming filter != 0.
pub(crate) fn eval_packed_generic_add_cc<P: PackedField>(
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    x: &[P],
    y: &[P],
    z: &[P],
    cy: P,
    is_two_row_op: bool,
) {
    debug_assert!(x.len() == N_LIMBS && y.len() == N_LIMBS && z.len() == N_LIMBS);

    let pol_sum = x.iter().zip(y).map(|(&xi, &yi)| xi + yi);
    let expected_cy = eval_packed_generic_pol_eval_equal(
        yield_constr,
        filter,
        pol_sum,
        z.iter().copied(),
        is_two_row_op,
    );
    // We don't need to check that expected_cy is 0 or 1, since cy has
    // already been checked to be 0 or 1.
    if is_two_row_op {
        yield_constr.constraint_transition(filter * (cy - expected_cy));
    } else {
        yield_constr.constraint(filter * (cy - expected_cy));
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_add = lv[IS_ADD];
    let is_sub = lv[IS_SUB];
    let is_lt = lv[IS_LT];
    let is_gt = lv[IS_GT];

    let x = &lv[GENERAL_REGISTER_0];
    let y = &lv[GENERAL_REGISTER_1];
    let z = &lv[GENERAL_REGISTER_2];
    let cy = lv[GENERAL_REGISTER_BIT];

    let op_filter = is_add + is_sub + is_lt + is_gt;
    eval_packed_generic_check_is_one_bit(yield_constr, op_filter, cy);

    // x + y = z + cy*2^256
    eval_packed_generic_add_cc(yield_constr, op_filter, x, y, z, cy, false);
}

#[allow(clippy::needless_collect)]
pub(crate) fn eval_ext_circuit_add_cc<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    x: &[ExtensionTarget<D>],
    y: &[ExtensionTarget<D>],
    z: &[ExtensionTarget<D>],
    cy: ExtensionTarget<D>,
    is_two_row_op: bool,
) {
    debug_assert!(x.len() == N_LIMBS && y.len() == N_LIMBS && z.len() == N_LIMBS);

    // Since `map` is lazy and the closure passed to it borrows
    // `builder`, we can't then borrow builder again below in the call
    // to `eval_ext_circuit_pol_eval_equal`. The solution is to force
    // evaluation with `collect`.
    let pol_sum = x
        .iter()
        .zip(y)
        .map(|(&xi, &yi)| builder.add_extension(xi, yi))
        .collect::<Vec<ExtensionTarget<D>>>();

    let expected_cy = eval_ext_circuit_pol_eval_equal(
        builder,
        yield_constr,
        filter,
        pol_sum.into_iter(),
        z.iter().copied(),
        is_two_row_op,
    );
    let good_cy = builder.sub_extension(cy, expected_cy);
    let filter = builder.mul_extension(filter, good_cy);
    if is_two_row_op {
        yield_constr.constraint_transition(builder, filter);
    } else {
        yield_constr.constraint(builder, filter);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_add = lv[IS_ADD];
    let is_sub = lv[IS_SUB];
    let is_lt = lv[IS_LT];
    let is_gt = lv[IS_GT];

    let x = &lv[GENERAL_REGISTER_0];
    let y = &lv[GENERAL_REGISTER_1];
    let z = &lv[GENERAL_REGISTER_2];
    let cy = lv[GENERAL_REGISTER_BIT];

    let op_filter = builder.add_many_extension([is_add, is_sub, is_lt, is_gt]);
    eval_ext_circuit_check_is_one_bit(builder, yield_constr, op_filter, cy);

    eval_ext_circuit_add_cc(builder, yield_constr, op_filter, x, y, z, cy, false);
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::{Field, Sample};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::*;
    use crate::arithmetic::columns::NUM_ARITH_COLUMNS;
    use crate::constraint_consumer::ConstraintConsumer;

    // TODO: Should be able to refactor this test to apply to all operations.
    #[test]
    fn generate_eval_consistency_not_addcc() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));

        // if the operation filters are all zero, then the constraints
        // should be met even if all values are
        // garbage.
        lv[IS_ADD] = F::ZERO;
        lv[IS_SUB] = F::ZERO;
        lv[IS_LT] = F::ZERO;
        lv[IS_GT] = F::ZERO;

        let mut constrant_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            F::ONE,
            F::ONE,
            F::ONE,
        );
        eval_packed_generic(&lv, &mut constrant_consumer);
        for &acc in &constrant_consumer.constraint_accs {
            assert_eq!(acc, F::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency_addcc() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        const N_ITERS: usize = 1000;

        for _ in 0..N_ITERS {
            for op_filter in [IS_ADD, IS_SUB, IS_LT, IS_GT] {
                // set entire row to random 16-bit values
                let mut lv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));

                // set operation filter and ensure all constraints are
                // satisfied.  we have to explicitly set the other
                // operation filters to zero since all are treated by
                // the call.
                lv[IS_ADD] = F::ZERO;
                lv[IS_SUB] = F::ZERO;
                lv[IS_LT] = F::ZERO;
                lv[IS_GT] = F::ZERO;
                lv[op_filter] = F::ONE;

                generate(&mut lv, op_filter);

                let mut constrant_consumer = ConstraintConsumer::new(
                    vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                    F::ONE,
                    F::ONE,
                    F::ONE,
                );
                eval_packed_generic(&lv, &mut constrant_consumer);
                for &acc in &constrant_consumer.constraint_accs {
                    assert_eq!(acc, F::ZERO);
                }
            }
        }
    }
}
