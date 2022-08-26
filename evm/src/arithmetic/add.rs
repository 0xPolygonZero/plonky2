use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::columns::*;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

pub(crate) fn u256_add_cc(input0: [u64; N_LIMBS], input1: [u64; N_LIMBS]) -> ([u64; N_LIMBS], u64) {
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

/// Given two sequences `larger` and `smaller` of equal length (not
/// checked), verifies that \sum_i larger[i] 2^(LIMB_BITS * i) ==
/// \sum_i smaller[i] 2^(LIMB_BITS * i), taking care of carry propagation.
///
/// The sequences must have been produced by `{add,sub}::eval_packed_generic()`.
pub(crate) fn eval_packed_generic_are_equal<P, I, J>(
    yield_constr: &mut ConstraintConsumer<P>,
    is_op: P,
    larger: I,
    smaller: J,
) -> P
where
    P: PackedField,
    I: Iterator<Item = P>,
    J: Iterator<Item = P>,
{
    let overflow = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    let overflow_inv = overflow.inverse();
    let mut cy = P::ZEROS;
    for (a, b) in larger.zip(smaller) {
        // t should be either 0 or 2^LIMB_BITS
        let t = cy + a - b;
        yield_constr.constraint(is_op * t * (overflow - t));
        // cy <-- 0 or 1
        // NB: this is multiplication by a constant, so doesn't
        // increase the degree of the constraint.
        cy = t * overflow_inv;
    }
    cy
}

pub(crate) fn eval_ext_circuit_are_equal<F, const D: usize, I, J>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    is_op: ExtensionTarget<D>,
    larger: I,
    smaller: J,
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
    cy
}

pub fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS]) {
    let input0_limbs = ADD_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs = ADD_INPUT_1.map(|c| lv[c].to_canonical_u64());

    // Input and output have 16-bit limbs
    let (output_limbs, _) = u256_add_cc(input0_limbs, input1_limbs);

    for (&c, output_limb) in ADD_OUTPUT.iter().zip(output_limbs) {
        lv[c] = F::from_canonical_u64(output_limb);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    range_check_error!(ADD_INPUT_0, 16);
    range_check_error!(ADD_INPUT_1, 16);
    range_check_error!(ADD_OUTPUT, 16);

    let is_add = lv[IS_ADD];
    let input0_limbs = ADD_INPUT_0.iter().map(|&c| lv[c]);
    let input1_limbs = ADD_INPUT_1.iter().map(|&c| lv[c]);
    let output_limbs = ADD_OUTPUT.iter().map(|&c| lv[c]);

    // This computed output is not yet reduced; i.e. some limbs may be
    // more than 16 bits.
    let output_computed = input0_limbs.zip(input1_limbs).map(|(a, b)| a + b);

    eval_packed_generic_are_equal(yield_constr, is_add, output_computed, output_limbs);
}

#[allow(clippy::needless_collect)]
pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_add = lv[IS_ADD];
    let input0_limbs = ADD_INPUT_0.iter().map(|&c| lv[c]);
    let input1_limbs = ADD_INPUT_1.iter().map(|&c| lv[c]);
    let output_limbs = ADD_OUTPUT.iter().map(|&c| lv[c]);

    // Since `map` is lazy and the closure passed to it borrows
    // `builder`, we can't then borrow builder again below in the call
    // to `eval_ext_circuit_are_equal`. The solution is to force
    // evaluation with `collect`.
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

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Field;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::*;
    use crate::arithmetic::columns::NUM_ARITH_COLUMNS;
    use crate::constraint_consumer::ConstraintConsumer;

    // TODO: Should be able to refactor this test to apply to all operations.
    #[test]
    fn generate_eval_consistency_not_add() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // if `IS_ADD == 0`, then the constraints should be met even
        // if all values are garbage.
        lv[IS_ADD] = F::ZERO;

        let mut constrant_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_packed_generic(&lv, &mut constrant_consumer);
        for &acc in &constrant_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency_add() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // set `IS_ADD == 1` and ensure all constraints are satisfied.
        lv[IS_ADD] = F::ONE;
        // set inputs to random values
        for (&ai, bi) in ADD_INPUT_0.iter().zip(ADD_INPUT_1) {
            lv[ai] = F::from_canonical_u16(rng.gen());
            lv[bi] = F::from_canonical_u16(rng.gen());
        }

        generate(&mut lv);

        let mut constrant_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_packed_generic(&lv, &mut constrant_consumer);
        for &acc in &constrant_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }
}
