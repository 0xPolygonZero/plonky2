use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::add::{eval_ext_circuit_are_equal, eval_packed_generic_are_equal};
use crate::arithmetic::columns::*;
use crate::arithmetic::sub::u256_sub_br;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

pub(crate) fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS], op: usize) {
    let input0 = CMP_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1 = CMP_INPUT_1.map(|c| lv[c].to_canonical_u64());

    let (diff, br) = match op {
        // input0 - input1 == diff + br*2^256
        IS_LT => u256_sub_br(input0, input1),
        // input1 - input0 == diff + br*2^256
        IS_GT => u256_sub_br(input1, input0),
        IS_SLT => todo!(),
        IS_SGT => todo!(),
        _ => panic!("op code not a comparison"),
    };

    for (&c, diff_limb) in CMP_AUX_INPUT.iter().zip(diff) {
        lv[c] = F::from_canonical_u64(diff_limb);
    }
    lv[CMP_OUTPUT] = F::from_canonical_u64(br);
}

pub(crate) fn eval_packed_generic_lt<P: PackedField>(
    yield_constr: &mut ConstraintConsumer<P>,
    is_op: P,
    input0: [P; N_LIMBS],
    input1: [P; N_LIMBS],
    aux: [P; N_LIMBS],
    output: P,
) {
    // Verify (input0 < input1) == output by providing aux such that
    // input0 - input1 == aux + output*2^256.
    let lhs_limbs = input0.iter().zip(input1).map(|(&a, b)| a - b);
    let cy = eval_packed_generic_are_equal(yield_constr, is_op, aux.into_iter(), lhs_limbs);
    // We don't need to check that cy is 0 or 1, since output has
    // already been checked to be 0 or 1.
    yield_constr.constraint(is_op * (cy - output));
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    range_check_error!(CMP_INPUT_0, 16);
    range_check_error!(CMP_INPUT_1, 16);
    range_check_error!(CMP_AUX_INPUT, 16);
    range_check_error!([CMP_OUTPUT], 1);

    let input0 = CMP_INPUT_0.map(|c| lv[c]);
    let input1 = CMP_INPUT_1.map(|c| lv[c]);
    let aux = CMP_AUX_INPUT.map(|c| lv[c]);
    let output = lv[CMP_OUTPUT];

    eval_packed_generic_lt(yield_constr, lv[IS_LT], input0, input1, aux, output);
    eval_packed_generic_lt(yield_constr, lv[IS_GT], input1, input0, aux, output);
}

#[allow(clippy::needless_collect)]
pub(crate) fn eval_ext_circuit_lt<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    is_op: ExtensionTarget<D>,
    input0: [ExtensionTarget<D>; N_LIMBS],
    input1: [ExtensionTarget<D>; N_LIMBS],
    aux: [ExtensionTarget<D>; N_LIMBS],
    output: ExtensionTarget<D>,
) {
    // Since `map` is lazy and the closure passed to it borrows
    // `builder`, we can't then borrow builder again below in the call
    // to `eval_ext_circuit_are_equal`. The solution is to force
    // evaluation with `collect`.
    let lhs_limbs = input0
        .iter()
        .zip(input1)
        .map(|(&a, b)| builder.sub_extension(a, b))
        .collect::<Vec<ExtensionTarget<D>>>();

    let cy = eval_ext_circuit_are_equal(
        builder,
        yield_constr,
        is_op,
        aux.into_iter(),
        lhs_limbs.into_iter(),
    );
    let good_output = builder.sub_extension(cy, output);
    let filter = builder.mul_extension(is_op, good_output);
    yield_constr.constraint(builder, filter);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let input0 = CMP_INPUT_0.map(|c| lv[c]);
    let input1 = CMP_INPUT_1.map(|c| lv[c]);
    let aux = CMP_AUX_INPUT.map(|c| lv[c]);
    let output = lv[CMP_OUTPUT];

    eval_ext_circuit_lt(
        builder,
        yield_constr,
        lv[IS_LT],
        input0,
        input1,
        aux,
        output,
    );
    eval_ext_circuit_lt(
        builder,
        yield_constr,
        lv[IS_GT],
        input1,
        input0,
        aux,
        output,
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
    fn generate_eval_consistency_not_compare() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // if `IS_LT == 0`, then the constraints should be met even if
        // all values are garbage. `eval_packed_generic` handles IS_GT
        // at the same time, so we check both at once.
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
    fn generate_eval_consistency_compare() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));
        const N_ITERS: usize = 1000;

        for _ in 0..N_ITERS {
            for (op, other_op) in [(IS_LT, IS_GT), (IS_GT, IS_LT)] {
                // set op == 1 and ensure all constraints are satisfied.
                // we have to explicitly set the other op to zero since both
                // are treated by the call.
                lv[op] = F::ONE;
                lv[other_op] = F::ZERO;

                // set inputs to random values
                for (&ai, bi) in CMP_INPUT_0.iter().zip(CMP_INPUT_1) {
                    lv[ai] = F::from_canonical_u16(rng.gen());
                    lv[bi] = F::from_canonical_u16(rng.gen());
                }

                generate(&mut lv, op);

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
