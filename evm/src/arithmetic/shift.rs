//! Support for the EVM SHL and SHR instructions.
//!
//! This crate verifies an EVM shift instruction, which takes two
//! 256-bit inputs S and A, and produces a 256-bit output C satisfying
//!
//!    C = A << S (mod 2^256) for SHL or
//!    C = A >> S (mod 2^256) for SHR.
//!
//! The way this computation is carried is by providing a third input
//!    B = 1 << S (mod 2^256)
//! and then computing:
//!    C = A * B (mod 2^256) for SHL or
//!    C = A / B (mod 2^256) for SHR
//!
//! Inputs A, S, and B, and output C, are given as arrays of 16-bit
//! limbs. For example, if the limbs of A are a[0]...a[15], then
//!
//!    A = \sum_{i=0}^15 a[i] β^i,
//!
//! where β = 2^16 = 2^LIMB_BITS. To verify that A, S, B and C satisfy
//! the equations, we proceed similarly to MUL for SHL and to DIV for SHR.

use ethereum_types::U256;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use super::modular::modular_constr_poly_ext_circuit;
use crate::arithmetic::columns::{self, *};
use crate::arithmetic::modular::{generate_modular_op, modular_constr_poly};
use crate::arithmetic::utils::*;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

/// Generates a shift operation (either SHL or SHR).
/// The inputs are stored in the form `(shift, input, 1 << shift)`.
/// NB: if `shift > 2^32`, then the third register holds 0.
pub fn generate<F: PrimeField64>(
    lv: &mut [F],
    nv: &mut [F],
    is_shl: bool,
    input: U256,
    shift: U256,
    result: U256,
) {
    // TODO: It would probably be clearer/cleaner to read the U256
    // into an [i64;N] and then copy that to the lv table.
    // The first input is the shift we need to apply.
    u256_to_array(&mut lv[INPUT_REGISTER_0], shift);
    // The second register holds the input which needs shifting.
    u256_to_array(&mut lv[INPUT_REGISTER_1], input);
    u256_to_array(&mut lv[OUTPUT_REGISTER], result);
    // If `shift > 2^32`, the shifted displacement is set to 0.
    // Compute 1 << shift and store it in the third input register.
    let shifted_displacement = if shift > U256::from(255u64) {
        U256::zero()
    } else {
        U256::one() << shift
    };

    u256_to_array(&mut lv[INPUT_REGISTER_2], shifted_displacement);

    let input0 = read_value_i64_limbs(lv, INPUT_REGISTER_1); // input
    let input1 = read_value_i64_limbs(lv, INPUT_REGISTER_2); // 1 << shift

    if is_shl {
        // If the operation is SHL, we compute `input * shifted_displacement`.
        const MASK: i64 = (1i64 << LIMB_BITS) - 1i64;

        // Input and output have 16-bit limbs
        let mut output_limbs = [0i64; N_LIMBS];

        // Column-wise pen-and-paper long multiplication on 16-bit limbs.
        // First calculate the coefficients of a(x)*b(x) (in unreduced_prod),
        // then do carry propagation to obtain C = c(β) = a(β)*b(β).
        let mut cy = 0i64;
        let mut unreduced_prod = pol_mul_lo(input0, input1);
        for col in 0..N_LIMBS {
            let t = unreduced_prod[col] + cy;
            cy = t >> LIMB_BITS;
            output_limbs[col] = t & MASK;
        }
        // In principle, the last cy could be dropped because this is
        // multiplication modulo 2^256. However, we need it below for
        // aux_limbs to handle the fact that unreduced_prod will
        // inevitably contain one digit's worth that is > 2^256.

        pol_sub_assign(&mut unreduced_prod, &output_limbs);

        let mut aux_limbs = pol_remove_root_2exp::<LIMB_BITS, _, N_LIMBS>(unreduced_prod);
        aux_limbs[N_LIMBS - 1] = -cy;

        for c in aux_limbs.iter_mut() {
            // we store the unsigned offset value c + 2^20
            *c += AUX_COEFF_ABS_MAX;
        }

        debug_assert!(aux_limbs.iter().all(|&c| c.abs() <= 2 * AUX_COEFF_ABS_MAX));

        lv[MUL_AUX_INPUT_LO].copy_from_slice(&aux_limbs.map(|c| F::from_canonical_u16(c as u16)));
        lv[MUL_AUX_INPUT_HI]
            .copy_from_slice(&aux_limbs.map(|c| F::from_canonical_u16((c >> 16) as u16)));
    } else {
        // If the operation is SHR, we compute: `input / shifted_displacement` if `shifted_displacement == 0`
        // otherwise, the output is 0.
        let input_limbs = read_value_i64_limbs::<N_LIMBS, _>(lv, INPUT_REGISTER_1);
        let pol_input = pol_extend(input_limbs);
        let (out, quo_input) =
            generate_modular_op(lv, nv, columns::IS_SHL, pol_input, INPUT_REGISTER_2);
        debug_assert!(
            &quo_input[N_LIMBS..].iter().all(|&x| x == F::ZERO),
            "expected top half of quo_input to be zero"
        );

        // Initialise whole (double) register to zero; the low half will
        // be overwritten via lv[AUX_INPUT_REGISTER] below.
        for i in MODULAR_QUO_INPUT {
            lv[i] = F::ZERO;
        }

        lv[AUX_INPUT_REGISTER_0].copy_from_slice(&out);
    }
}

/// Evaluates the constraints for an SHL opcode.
/// The logic is very similar to the one for MUL. The only difference is that
/// the inputs are in `INPUT_REGISTER_1`  and `INPUT_REGISTER_2` instead of
/// `INPUT_REGISTER_0` and `INPUT_REGISTER_1`.
pub fn eval_packed_shl<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let base = P::Scalar::from_canonical_u64(1 << LIMB_BITS);

    let is_shl = lv[IS_SHL];
    let input0_limbs = read_value::<N_LIMBS, _>(lv, INPUT_REGISTER_1);
    let shifted_limbs = read_value::<N_LIMBS, _>(lv, INPUT_REGISTER_2);
    let output_limbs = read_value::<N_LIMBS, _>(lv, OUTPUT_REGISTER);

    let aux_limbs = {
        // MUL_AUX_INPUT was offset by 2^20 in generation, so we undo
        // that here
        let offset = P::Scalar::from_canonical_u64(AUX_COEFF_ABS_MAX as u64);
        let mut aux_limbs = read_value::<N_LIMBS, _>(lv, MUL_AUX_INPUT_LO);
        let aux_limbs_hi = &lv[MUL_AUX_INPUT_HI];
        for (lo, &hi) in aux_limbs.iter_mut().zip(aux_limbs_hi) {
            *lo += hi * base - offset;
        }
        aux_limbs
    };

    // Constraint poly holds the coefficients of the polynomial that
    // must be identically zero for this multiplication to be
    // verified.
    //
    // These two lines set constr_poly to the polynomial a(x)b(x) - c(x),
    // where a, b and c are the polynomials
    //
    //   a(x) = \sum_i input0_limbs[i] * x^i
    //   b(x) = \sum_i input1_limbs[i] * x^i
    //   c(x) = \sum_i output_limbs[i] * x^i
    //
    // This polynomial should equal (x - β)*s(x) where s is
    //
    //   s(x) = \sum_i aux_limbs[i] * x^i
    //
    let mut constr_poly = pol_mul_lo(input0_limbs, shifted_limbs);
    pol_sub_assign(&mut constr_poly, &output_limbs);

    // This subtracts (x - β) * s(x) from constr_poly.
    pol_sub_assign(&mut constr_poly, &pol_adjoin_root(aux_limbs, base));

    // At this point constr_poly holds the coefficients of the
    // polynomial a(x)b(x) - c(x) - (x - β)*s(x). The
    // multiplication is valid if and only if all of those
    // coefficients are zero.
    for &c in &constr_poly {
        yield_constr.constraint(is_shl * c);
    }
}

/// Evaluates the constraints for an SHR opcode.
/// The logic is very similar to the one for DIV. The only difference is that
/// the inputs are in `INPUT_REGISTER_1`  and `INPUT_REGISTER_2` instead of
/// `INPUT_REGISTER_0` and `INPUT_REGISTER_1`.
fn eval_packed_shr<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    nv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let quo_range = OUTPUT_REGISTER;
    let rem_range = AUX_INPUT_REGISTER_0;
    let filter = lv[IS_SHR];
    debug_assert!(quo_range.len() == N_LIMBS);
    debug_assert!(rem_range.len() == N_LIMBS);

    yield_constr.constraint_last_row(filter);

    let num = &lv[INPUT_REGISTER_1];
    let den = read_value(lv, INPUT_REGISTER_2);
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

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    nv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_shl(lv, yield_constr);
    eval_packed_shr(lv, nv, yield_constr);
}

pub fn eval_ext_circuit_shl<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_shl = lv[IS_SHL];
    let input0_limbs = read_value::<N_LIMBS, _>(lv, INPUT_REGISTER_1);
    let shifted_limbs = read_value::<N_LIMBS, _>(lv, INPUT_REGISTER_2);

    let output_limbs = read_value::<N_LIMBS, _>(lv, OUTPUT_REGISTER);

    let aux_limbs = {
        let base = builder.constant_extension(F::Extension::from_canonical_u64(1 << LIMB_BITS));
        let offset =
            builder.constant_extension(F::Extension::from_canonical_u64(AUX_COEFF_ABS_MAX as u64));
        let mut aux_limbs = read_value::<N_LIMBS, _>(lv, MUL_AUX_INPUT_LO);
        let aux_limbs_hi = &lv[MUL_AUX_INPUT_HI];
        for (lo, &hi) in aux_limbs.iter_mut().zip(aux_limbs_hi) {
            //*lo = lo + hi * base - offset;
            let t = builder.mul_sub_extension(hi, base, offset);
            *lo = builder.add_extension(*lo, t);
        }
        aux_limbs
    };

    let mut constr_poly = pol_mul_lo_ext_circuit(builder, input0_limbs, shifted_limbs);
    pol_sub_assign_ext_circuit(builder, &mut constr_poly, &output_limbs);

    let base = builder.constant_extension(F::Extension::from_canonical_u64(1 << LIMB_BITS));
    let rhs = pol_adjoin_root_ext_circuit(builder, aux_limbs, base);
    pol_sub_assign_ext_circuit(builder, &mut constr_poly, &rhs);

    for &c in &constr_poly {
        let filter = builder.mul_extension(is_shl, c);
        yield_constr.constraint(builder, filter);
    }
}

pub fn eval_ext_circuit_shr<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    nv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = lv[IS_SHR];
    yield_constr.constraint_last_row(builder, filter);

    let quo_range = OUTPUT_REGISTER;
    let rem_range = AUX_INPUT_REGISTER_0;
    let num = &lv[INPUT_REGISTER_1];
    let den = read_value(lv, INPUT_REGISTER_2);
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

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    nv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_shl(builder, lv, yield_constr);
    eval_ext_circuit_shr(builder, lv, nv, yield_constr);
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
    fn generate_eval_consistency_not_shl() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));
        let nv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));

        // if `IS_SHL == 0` and ÌS_SHR == 0`, then the constraints should be met even
        // if all values are garbage.
        lv[IS_SHL] = F::ZERO;
        lv[IS_SHR] = F::ZERO;

        let mut constraint_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_packed_generic(&lv, &nv, &mut constraint_consumer);
        for &acc in &constraint_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency_shl() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));
        let mut nv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));

        // set `IS_MUL == 1` and ensure all constraints are satisfied.
        lv[IS_SHL] = F::ONE;
        lv[IS_SHR] = F::ZERO;

        for _i in 0..N_RND_TESTS {
            let shift = U256::from(rng.gen::<usize>());
            let shifted = if shift > U256::from(255) {
                U256::zero()
            } else {
                U256::one() << shift
            };
            u256_to_array(&mut lv[INPUT_REGISTER_0], shift);
            u256_to_array(&mut lv[INPUT_REGISTER_2], shifted);
            let mut full_input = U256::from(0);
            // set inputs to random values
            for ai in INPUT_REGISTER_1 {
                lv[ai] = F::from_canonical_u16(rng.gen());
                full_input =
                    U256::from(lv[ai].to_canonical_u64()) + full_input * U256::from(1 << 16);
            }

            let output = full_input * shifted;

            generate(&mut lv, &mut nv, true, full_input, shift, output);

            let mut constraint_consumer = ConstraintConsumer::new(
                vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                GoldilocksField::ONE,
                GoldilocksField::ONE,
                GoldilocksField::ONE,
            );
            eval_packed_generic(&lv, &nv, &mut constraint_consumer);
            for &acc in &constraint_consumer.constraint_accs {
                assert_eq!(acc, GoldilocksField::ZERO);
            }
        }
    }
}
