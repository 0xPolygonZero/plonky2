//! Support for the EVM MOD instruction.
//!
//! This crate verifies an EVM MOD instruction, which takes two
//! 256-bit inputs A and M, and produces a 256-bit output C satisfying
//!
//!    C = A (mod M).
//!
//! See the comments in `addmod.rs` for more details.

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::addmod;
use crate::arithmetic::columns::*;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

pub fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS]) {
    let input_limbs = MOD_INPUT.map(|c| lv[c].to_canonical_u64());
    let zero = [0u64; N_LIMBS];
    let modulus_limbs = MOD_MODULUS.map(|c| lv[c].to_canonical_u64());

    addmod::generate_addmod(
        lv,
        input_limbs,
        zero,
        modulus_limbs,
        MOD_OUTPUT,
        MOD_QUO_INPUT,
        MOD_AUX_INPUT,
    );
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    range_check_error!(MOD_INPUT, 16);
    range_check_error!(MOD_MODULUS, 16);
    range_check_error!(MOD_QUO_INPUT, 16);
    range_check_error!(MOD_AUX_INPUT, 16, signed);
    range_check_error!(MOD_OUTPUT, 16);

    let is_mod = lv[IS_MOD];
    let input_limbs = MOD_INPUT.map(|c| lv[c]);
    let modulus_limbs = MOD_MODULUS.map(|c| lv[c]);
    let quot_limbs = MOD_QUO_INPUT.map(|c| lv[c]);
    let aux_limbs = MOD_AUX_INPUT.map(|c| lv[c]);
    let output_limbs = MOD_OUTPUT.map(|c| lv[c]);

    // NB: This should be const, but Rust complains "can't use type
    // parameters from outer function;" which is non-sensical since
    // there is no outer function.
    let zero = [P::ZEROS; N_LIMBS];

    addmod::eval_packed_generic_addmod(
        is_mod,
        input_limbs,
        zero,
        modulus_limbs,
        output_limbs,
        quot_limbs,
        aux_limbs,
        yield_constr,
    );
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_mod = lv[IS_MOD];
    let input_limbs = MOD_INPUT.map(|c| lv[c]);
    let modulus_limbs = MOD_MODULUS.map(|c| lv[c]);
    let quot_limbs = MOD_QUO_INPUT.map(|c| lv[c]);
    let aux_limbs = MOD_AUX_INPUT.map(|c| lv[c]);
    let output_limbs = MOD_OUTPUT.map(|c| lv[c]);

    let zero_e = builder.zero_extension();
    let zero = [zero_e; N_LIMBS];

    addmod::eval_ext_circuit_addmod(
        is_mod,
        input_limbs,
        zero,
        modulus_limbs,
        output_limbs,
        quot_limbs,
        aux_limbs,
        builder,
        yield_constr,
    );
}

#[cfg(test)]
mod tests {
    use itertools::izip;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Field;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::*;
    use crate::arithmetic::columns::NUM_ARITH_COLUMNS;
    use crate::constraint_consumer::ConstraintConsumer;

    const N_RND_TESTS: usize = 1000;

    // TODO: Should be able to refactor this test to apply to all operations.
    #[test]
    fn generate_eval_consistency_not_mod() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // if `IS_MOD == 0`, then the constraints should be met even
        // if all values are garbage.
        lv[IS_MOD] = F::ZERO;

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
    fn generate_eval_consistency_mod() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // set `IS_MOD == 1` and ensure all constraints are satisfied.
        lv[IS_MOD] = F::ONE;
        for i in 0..N_RND_TESTS {
            // set inputs to random values
            for (&ai, &mi) in izip!(MOD_INPUT.iter(), MOD_MODULUS.iter()) {
                lv[ai] = F::from_canonical_u16(rng.gen());
                lv[mi] = F::from_canonical_u16(rng.gen());
            }

            // For the second half of the tests, set the top 16 - start
            // digits to zero, so the modulus is much smaller than the
            // inputs.
            if i > N_RND_TESTS / 2 {
                // 1 <= start < N_LIMBS
                let start = (rng.gen::<usize>() % (N_LIMBS - 1)) + 1;
                for &mi in &MOD_MODULUS[start..N_LIMBS] {
                    lv[mi] = F::ZERO;
                }
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
