use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::add::{eval_ext_circuit_are_equal, eval_packed_generic_are_equal};
use crate::arithmetic::columns::*;
use crate::arithmetic::utils::read_value_u64_limbs;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

pub(crate) fn u256_sub_br(input0: [u64; N_LIMBS], input1: [u64; N_LIMBS]) -> ([u64; N_LIMBS], u64) {
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

pub fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS]) {
    let input0 = read_value_u64_limbs(lv, SUB_INPUT_0);
    let input1 = read_value_u64_limbs(lv, SUB_INPUT_1);

    let (output_limbs, _) = u256_sub_br(input0, input1);

    lv[SUB_OUTPUT].copy_from_slice(&output_limbs.map(|c| F::from_canonical_u64(c)));
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    range_check_error!(SUB_INPUT_0, 16);
    range_check_error!(SUB_INPUT_1, 16);
    range_check_error!(SUB_OUTPUT, 16);

    let is_sub = lv[IS_SUB];
    let input0_limbs = &lv[SUB_INPUT_0];
    let input1_limbs = &lv[SUB_INPUT_1];
    let output_limbs = &lv[SUB_OUTPUT];

    let output_computed = input0_limbs.iter().zip(input1_limbs).map(|(&a, &b)| a - b);

    eval_packed_generic_are_equal(
        yield_constr,
        is_sub,
        output_limbs.iter().copied(),
        output_computed,
        false,
    );
}

#[allow(clippy::needless_collect)]
pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_sub = lv[IS_SUB];
    let input0_limbs = &lv[SUB_INPUT_0];
    let input1_limbs = &lv[SUB_INPUT_1];
    let output_limbs = &lv[SUB_OUTPUT];

    // Since `map` is lazy and the closure passed to it borrows
    // `builder`, we can't then borrow builder again below in the call
    // to `eval_ext_circuit_are_equal`. The solution is to force
    // evaluation with `collect`.
    let output_computed = input0_limbs
        .iter()
        .zip(input1_limbs)
        .map(|(&a, &b)| builder.sub_extension(a, b))
        .collect::<Vec<ExtensionTarget<D>>>();

    eval_ext_circuit_are_equal(
        builder,
        yield_constr,
        is_sub,
        output_limbs.iter().copied(),
        output_computed.into_iter(),
        false,
    );
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

    const N_RND_TESTS: usize = 1000;

    // TODO: Should be able to refactor this test to apply to all operations.
    #[test]
    fn generate_eval_consistency_not_sub() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));

        // if `IS_SUB == 0`, then the constraints should be met even
        // if all values are garbage.
        lv[IS_SUB] = F::ZERO;

        let mut constraint_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_packed_generic(&lv, &mut constraint_consumer);
        for &acc in &constraint_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency_sub() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));

        // set `IS_SUB == 1` and ensure all constraints are satisfied.
        lv[IS_SUB] = F::ONE;

        for _ in 0..N_RND_TESTS {
            // set inputs to random values
            for (ai, bi) in SUB_INPUT_0.zip(SUB_INPUT_1) {
                lv[ai] = F::from_canonical_u16(rng.gen());
                lv[bi] = F::from_canonical_u16(rng.gen());
            }

            generate(&mut lv);

            let mut constraint_consumer = ConstraintConsumer::new(
                vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                GoldilocksField::ONE,
                GoldilocksField::ONE,
                GoldilocksField::ONE,
            );
            eval_packed_generic(&lv, &mut constraint_consumer);
            for &acc in &constraint_consumer.constraint_accs {
                assert_eq!(acc, GoldilocksField::ZERO);
            }
        }
    }
}
