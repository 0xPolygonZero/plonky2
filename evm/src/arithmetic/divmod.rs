//! Support for EVM instructions DIV and MOD.
//!
//! The logic for verifying them is detailed in the `modular` submodule.

use std::ops::Range;

use ethereum_types::U256;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::PrimeField64;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::arithmetic::columns::*;
use crate::arithmetic::modular::{
    generate_modular_op, modular_constr_poly, modular_constr_poly_ext_circuit,
};
use crate::arithmetic::utils::*;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

/// Generates the output and auxiliary values for modular operations,
/// assuming the input, modular and output limbs are already set.
pub(crate) fn generate_divmod<F: PrimeField64>(
    lv: &mut [F],
    nv: &mut [F],
    filter: usize,
    input_limbs_range: Range<usize>,
    modulus_range: Range<usize>,
) {
    let input_limbs = read_value_i64_limbs::<N_LIMBS, _>(lv, input_limbs_range);
    let pol_input = pol_extend(input_limbs);
    let (out, quo_input) = generate_modular_op(lv, nv, filter, pol_input, modulus_range);

    debug_assert!(
        &quo_input[N_LIMBS..].iter().all(|&x| x == F::ZERO),
        "expected top half of quo_input to be zero"
    );

    // Initialise whole (double) register to zero; the low half will
    // be overwritten via lv[AUX_INPUT_REGISTER] below.
    for i in MODULAR_QUO_INPUT {
        lv[i] = F::ZERO;
    }

    match filter {
        IS_DIV | IS_SHR => {
            debug_assert!(
                lv[OUTPUT_REGISTER]
                    .iter()
                    .zip(&quo_input[..N_LIMBS])
                    .all(|(x, y)| x == y),
                "computed output doesn't match expected"
            );
            lv[AUX_INPUT_REGISTER_0].copy_from_slice(&out);
        }
        IS_MOD => {
            debug_assert!(
                lv[OUTPUT_REGISTER].iter().zip(&out).all(|(x, y)| x == y),
                "computed output doesn't match expected"
            );
            lv[AUX_INPUT_REGISTER_0].copy_from_slice(&quo_input[..N_LIMBS]);
        }
        _ => panic!("expected filter to be IS_DIV, IS_SHR or IS_MOD but it was {filter}"),
    };
}
/// Generate the output and auxiliary values for modular operations.
pub(crate) fn generate<F: PrimeField64>(
    lv: &mut [F],
    nv: &mut [F],
    filter: usize,
    input0: U256,
    input1: U256,
    result: U256,
) {
    debug_assert!(lv.len() == NUM_ARITH_COLUMNS);

    u256_to_array(&mut lv[INPUT_REGISTER_0], input0);
    u256_to_array(&mut lv[INPUT_REGISTER_1], input1);
    u256_to_array(&mut lv[OUTPUT_REGISTER], result);

    generate_divmod(lv, nv, filter, INPUT_REGISTER_0, INPUT_REGISTER_1);
}

/// Verify that num = quo * den + rem and 0 <= rem < den.
pub(crate) fn eval_packed_divmod_helper<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    nv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    num_range: Range<usize>,
    den_range: Range<usize>,
    quo_range: Range<usize>,
    rem_range: Range<usize>,
) {
    debug_assert!(quo_range.len() == N_LIMBS);
    debug_assert!(rem_range.len() == N_LIMBS);

    yield_constr.constraint_last_row(filter);

    let num = &lv[num_range];
    let den = read_value(lv, den_range);
    let quo = {
        let mut quo = [P::ZEROS; 2 * N_LIMBS];
        quo[..N_LIMBS].copy_from_slice(&lv[quo_range]);
        quo
    };
    let rem = read_value(lv, rem_range);

    let mut constr_poly = modular_constr_poly(lv, nv, yield_constr, filter, rem, den, quo);

    let input = num;
    pol_sub_assign(&mut constr_poly, input);

    for &c in constr_poly.iter() {
        yield_constr.constraint_transition(filter * c);
    }
}

pub(crate) fn eval_packed<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    nv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_divmod_helper(
        lv,
        nv,
        yield_constr,
        lv[IS_DIV],
        INPUT_REGISTER_0,
        INPUT_REGISTER_1,
        OUTPUT_REGISTER,
        AUX_INPUT_REGISTER_0,
    );
    eval_packed_divmod_helper(
        lv,
        nv,
        yield_constr,
        lv[IS_MOD],
        INPUT_REGISTER_0,
        INPUT_REGISTER_1,
        AUX_INPUT_REGISTER_0,
        OUTPUT_REGISTER,
    );
}

pub(crate) fn eval_ext_circuit_divmod_helper<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    nv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    num_range: Range<usize>,
    den_range: Range<usize>,
    quo_range: Range<usize>,
    rem_range: Range<usize>,
) {
    yield_constr.constraint_last_row(builder, filter);

    let num = &lv[num_range];
    let den = read_value(lv, den_range);
    let quo = {
        let zero = builder.zero_extension();
        let mut quo = [zero; 2 * N_LIMBS];
        quo[..N_LIMBS].copy_from_slice(&lv[quo_range]);
        quo
    };
    let rem = read_value(lv, rem_range);

    let mut constr_poly =
        modular_constr_poly_ext_circuit(lv, nv, builder, yield_constr, filter, rem, den, quo);

    let input = num;
    pol_sub_assign_ext_circuit(builder, &mut constr_poly, input);

    for &c in constr_poly.iter() {
        let t = builder.mul_extension(filter, c);
        yield_constr.constraint_transition(builder, t);
    }
}

pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    nv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_divmod_helper(
        builder,
        lv,
        nv,
        yield_constr,
        lv[IS_DIV],
        INPUT_REGISTER_0,
        INPUT_REGISTER_1,
        OUTPUT_REGISTER,
        AUX_INPUT_REGISTER_0,
    );
    eval_ext_circuit_divmod_helper(
        builder,
        lv,
        nv,
        yield_constr,
        lv[IS_MOD],
        INPUT_REGISTER_0,
        INPUT_REGISTER_1,
        AUX_INPUT_REGISTER_0,
        OUTPUT_REGISTER,
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
    const MODULAR_OPS: [usize; 2] = [IS_MOD, IS_DIV];

    // TODO: Should be able to refactor this test to apply to all operations.
    #[test]
    fn generate_eval_consistency_not_modular() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));
        let nv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));

        // if `IS_MOD == 0`, then the constraints should be met even
        // if all values are garbage (and similarly for the other operations).
        for op in MODULAR_OPS {
            lv[op] = F::ZERO;
        }
        // Since SHR uses the logic for DIV, `IS_SHR` should also be set to 0 here.
        lv[IS_SHR] = F::ZERO;

        let mut constraint_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_packed(&lv, &nv, &mut constraint_consumer);
        for &acc in &constraint_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);

        for op_filter in MODULAR_OPS {
            for i in 0..N_RND_TESTS {
                // set inputs to random values
                let mut lv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));
                let mut nv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));

                // Reset operation columns, then select one
                for op in MODULAR_OPS {
                    lv[op] = F::ZERO;
                }
                // Since SHR uses the logic for DIV, `IS_SHR` should also be set to 0 here.
                lv[IS_SHR] = F::ZERO;
                lv[op_filter] = F::ONE;

                let input0 = U256::from(rng.gen::<[u8; 32]>());
                let input1 = {
                    let mut modulus_limbs = [0u8; 32];
                    // For the second half of the tests, set the top
                    // 16-start digits of the "modulus" to zero so it is
                    // much smaller than the inputs.
                    if i > N_RND_TESTS / 2 {
                        // 1 <= start < N_LIMBS
                        let start = (rng.gen::<usize>() % (modulus_limbs.len() - 1)) + 1;
                        for mi in modulus_limbs.iter_mut().skip(start) {
                            *mi = 0u8;
                        }
                    }
                    U256::from(modulus_limbs)
                };

                let result = if input1 == U256::zero() {
                    U256::zero()
                } else if op_filter == IS_DIV {
                    input0 / input1
                } else {
                    input0 % input1
                };
                generate(&mut lv, &mut nv, op_filter, input0, input1, result);

                let mut constraint_consumer = ConstraintConsumer::new(
                    vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                    GoldilocksField::ONE,
                    GoldilocksField::ZERO,
                    GoldilocksField::ZERO,
                );
                eval_packed(&lv, &nv, &mut constraint_consumer);
                for &acc in &constraint_consumer.constraint_accs {
                    assert_eq!(acc, GoldilocksField::ZERO);
                }
            }
        }
    }

    #[test]
    fn zero_modulus() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);

        for op_filter in MODULAR_OPS {
            for _i in 0..N_RND_TESTS {
                // set inputs to random values and the modulus to zero;
                // the output is defined to be zero when modulus is zero.
                let mut lv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));
                let mut nv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));

                // Reset operation columns, then select one
                for op in MODULAR_OPS {
                    lv[op] = F::ZERO;
                }
                // Since SHR uses the logic for DIV, `IS_SHR` should also be set to 0 here.
                lv[IS_SHR] = F::ZERO;
                lv[op_filter] = F::ONE;

                let input0 = U256::from(rng.gen::<[u8; 32]>());
                let input1 = U256::zero();

                generate(&mut lv, &mut nv, op_filter, input0, input1, U256::zero());

                // check that the correct output was generated
                assert!(lv[OUTPUT_REGISTER].iter().all(|&c| c == F::ZERO));

                let mut constraint_consumer = ConstraintConsumer::new(
                    vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                    GoldilocksField::ONE,
                    GoldilocksField::ZERO,
                    GoldilocksField::ZERO,
                );
                eval_packed(&lv, &nv, &mut constraint_consumer);
                assert!(constraint_consumer
                    .constraint_accs
                    .iter()
                    .all(|&acc| acc == F::ZERO));

                // Corrupt one output limb by setting it to a non-zero value
                let random_oi = OUTPUT_REGISTER.start + rng.gen::<usize>() % N_LIMBS;
                lv[random_oi] = F::from_canonical_u16(rng.gen_range(1..u16::MAX));

                eval_packed(&lv, &nv, &mut constraint_consumer);

                // Check that at least one of the constraints was non-zero
                assert!(constraint_consumer
                    .constraint_accs
                    .iter()
                    .any(|&acc| acc != F::ZERO));
            }
        }
    }
}
