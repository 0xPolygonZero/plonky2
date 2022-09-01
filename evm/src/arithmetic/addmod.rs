//! Support for the EVM ADDMOD instruction.
//!
//! This crate verifies an EVM ADDMOD instruction, which takes three
//! 256-bit inputs A, B and M, and produces a 256-bit output C satisfying
//!
//!    C = A + B (mod M).
//!
//! Inputs A, B and M, and output C, are given as arrays of 16-bit
//! limbs. For example, if the limbs of A are a[0]...a[15], then
//!
//!    A = \sum_{i=0}^15 a[i] β^i,
//!
//! where β = 2^16. To verify that A, B, M and C satisfy the equation we
//! proceed as follows. Define a(x) = \sum_{i=0}^15 a[i] x^i (so A = a(β))
//! and similarly for b(x), m(x) and c(x). Then A+B = C (mod M) if and only
//! if there exist polynomials q and s such that
//!
//!    a(x) + b(x) - c(x) - m(x)*s(x) - (β - x)*q(x) == 0.
//!
//! Because A, B, M and C are 256-bit numbers, the degrees of a, b, m
//! and c are (at most) 15. On the other hand, the deg(m) can be 0, in
//! which case deg(s) may need to be 15, so in general we need to
//! accommodate deg(m*s) <= 30. Hence deg(q) can be up to 29.
//!
//! TODO: Write up analysis of degrees of the polynomials and the
//! bounds on their coefficients.

use num::{BigUint, Zero};
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::columns::*;
use crate::arithmetic::compare::{eval_packed_generic_lt, eval_ext_circuit_lt};
use crate::arithmetic::sub::u256_sub_br;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

fn input_to_biguint(limbs: &[u64; N_LIMBS]) -> BigUint {
    // Convert the base-2^16 representation into a base-2^32 representation
    // BigUint only takes slices, not iterators, so we need to `collect` here.
    const BASE: u64 = 1u64 << LIMB_BITS;
    let limbs = (0..N_LIMBS / 2)
        .map(|i| (limbs[2 * i] + BASE * limbs[2 * i + 1]) as u32)
        .collect::<Vec<_>>();
    BigUint::from_slice(&limbs)
}

fn biguint_to_output(num: &BigUint) -> [u64; N_LIMBS] {
    // Convert the base-2^32 representation into a base-2^16 representation
    assert!(num.bits() <= 256);
    let mut output = [0u64; N_LIMBS];
    for (i, limb) in num.iter_u32_digits().enumerate() {
        output[2 * i] = limb as u16 as u64;
        output[2 * i + 1] = (limb >> LIMB_BITS) as u64;
    }
    output
}

pub(crate) fn generate_addmod<F: RichField>(
    lv: &mut [F; NUM_ARITH_COLUMNS],
    input0_limbs: [u64; N_LIMBS],
    input1_limbs: [u64; N_LIMBS],
    modulus_limbs: [u64; N_LIMBS],
    output_cols: [usize; N_LIMBS],
    quot_cols: [usize; N_LIMBS],
    aux_cols: [usize; N_LIMBS],
    aux_out_reduced_cols: [usize; N_LIMBS],
    aux_constr_poly_cols: [usize; N_LIMBS],
) {
    let modulus = input_to_biguint(&modulus_limbs);

    // The spec defines the result of remainder modulo zero to be zero.
    if modulus.is_zero() {
        for i in 0..N_LIMBS {
            lv[output_cols[i]] = F::ZERO;

            // It doesn't matter what's in quot_cols when modulus is
            // zero, since the product with modulus will be zero.
            // Similarly, we don't use aux_out_reduced_cols when
            // modulus is zero. We set these both to zero "for neatness".
            lv[quot_cols[i]] = F::ZERO;
            lv[aux_out_reduced_cols[i]] = F::ZERO;

            // It also doesn't matter what's in aux_cols when modulus is
            // zero, except that it does have to be consistent with what we
            // put in aux_constr_poly.
            //
            // Easiest to set aux_cols values to zero and set
            // aux_constr_poly values to coefficients of a(x) + b(x).
            lv[aux_cols[i]] = F::ZERO;
            lv[aux_constr_poly_cols[i]] = F::from_canonical_u64(input0_limbs[i] + input1_limbs[i]);
        }
        return;
    }

    let input0 = input_to_biguint(&input0_limbs);
    let input1 = input_to_biguint(&input1_limbs);

    let sum = input0 + input1;
    let res = &sum % &modulus;
    let output_limbs = biguint_to_output(&res);
    let lambda = (sum - &res) / &modulus; // exact division
    let quot_limbs = biguint_to_output(&lambda);
    let (aux_out_reduced_limbs, br) = u256_sub_br(output_limbs, modulus_limbs);
    assert!(br == 1, "expected output < modulus");

    // TODO: Most of the code below should be refactored with the
    // original in 'mul.rs'.

    // unreduced_sum is the coefficients of the polynomial
    //
    //   a(x) + b(x) - c(x) - s(x)*m(x).
    //
    // All the inputs have coefficients < 2^16, so the conversions to
    // i64s are safe.
    let mut unreduced_sum = [0i64; N_LIMBS];
    for deg in 0..N_LIMBS {
        unreduced_sum[deg] = (input0_limbs[deg] + input1_limbs[deg]) as i64;
        unreduced_sum[deg] -= output_limbs[deg] as i64;

        for i in 0..=deg {
            // Invariant: i + j = deg
            let j = deg - i;
            let ai_x_bj = (quot_limbs[i] * modulus_limbs[j]) as i64;
            unreduced_sum[deg] -= ai_x_bj;
        }
    }

    // unreduced_sum must be zero when evaluated at x = β :=
    // 2^LIMB_BITS, hence it's divisible by (β - x). If we write it as
    //
    //   a(x) + b(x) - c(x) - s(x)*m(x) = \sum_{i=0}^n p_i x^i
    //                                  = (β - x) \sum_{i=0}^{n-1} q_i x^i
    //
    // then by comparing coefficients it is easy to see that
    //
    //   q_0 = p_0 / β  and  q_i = (p_i + q_{i-1}) / β
    //
    // for 0 < i < n-1 (and the divisions are exact).
    let mut aux_limbs = [0i64; N_LIMBS];
    aux_limbs[0] = unreduced_sum[0] >> LIMB_BITS;
    for deg in 1..N_LIMBS - 1 {
        aux_limbs[deg] = (unreduced_sum[deg] + aux_limbs[deg - 1]) >> LIMB_BITS;
    }
    aux_limbs[N_LIMBS - 1] = 0i64;

    for deg in 0..N_LIMBS {
        let c = aux_cols[deg];
        lv[c] = F::from_canonical_i64(aux_limbs[deg]);

        let c = quot_cols[deg];
        lv[c] = F::from_canonical_u64(quot_limbs[deg]);

        let c = aux_out_reduced_cols[deg];
        lv[c] = F::from_canonical_u64(aux_out_reduced_limbs[deg]);

        let c = aux_constr_poly_cols[deg];
        // The absval of this sum can't exceed the base field order,
        // and in fact must be much less.
        lv[c] = F::from_canonical_i64(unreduced_sum[deg]);

        let c = output_cols[deg];
        lv[c] = F::from_canonical_u64(output_limbs[deg]);
    }
}

pub fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS]) {
    let input0_limbs = ADDMOD_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs = ADDMOD_INPUT_1.map(|c| lv[c].to_canonical_u64());
    let modulus_limbs = ADDMOD_MODULUS.map(|c| lv[c].to_canonical_u64());

    generate_addmod(
        lv,
        input0_limbs,
        input1_limbs,
        modulus_limbs,
        ADDMOD_OUTPUT,
        ADDMOD_QUO_INPUT,
        ADDMOD_AUX_INPUT,
        ADDMOD_AUX_OUTPUT_REDUCED,
        ADDMOD_AUX_CONSTR_POLY,
    );
}

#[allow(clippy::needless_range_loop)]
pub(crate) fn eval_packed_generic_addmod<P: PackedField>(
    is_op: P,
    input0_limbs: [P; N_LIMBS],
    input1_limbs: [P; N_LIMBS],
    modulus_limbs: [P; N_LIMBS],
    output_limbs: [P; N_LIMBS],
    quot_limbs: [P; N_LIMBS],
    aux_limbs: [P; N_LIMBS],
    aux_output_reduced_limbs: [P; N_LIMBS],
    aux_constr_poly: [P; N_LIMBS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // The modulus limbs have been range-checked to be in [0, 2^16),
    // so the modulus is zero iff the sum of the limbs is zero.
    let modulus_limb_sum: P = modulus_limbs.into_iter().sum();
    // Idem. for the output
    let output_limb_sum: P = output_limbs.into_iter().sum();
    // This constraint ensures that the ouput is zero if the modulus
    // was zero (as required by the spec).
    let zero_mod = modulus_limb_sum + output_limb_sum;

    // FIXME: If modulus is zero, but output is non-zero, can the
    // prover find a value for the other inputs to produce a zero
    // constr_poly?

    let filter = is_op * zero_mod;

    // Start by confirming that output < modulus, i.e. that output is
    // reduced. Degree of `eval_packed_generic_lt` is deg(filter) + 1 = 3.
    let is_less_than = P::ONES;
    eval_packed_generic_lt(yield_constr, filter, output_limbs,
                           modulus_limbs, aux_output_reduced_limbs, is_less_than);

    // Constraint poly holds the coefficients of the polynomial that
    // must be identically zero for this modular addition to be
    // verified.
    let mut constr_poly = [P::ZEROS; N_LIMBS];

    // Set constr_poly[deg] to be the degree deg coefficient of the
    // polynomial a(x) + b(x) - c(x) - s(x) * m(x) where
    //
    //   a(x) = \sum_i input0_limbs[i] * 2^LIMB_BITS
    //   b(x) = \sum_i input1_limbs[i] * 2^LIMB_BITS
    //   c(x) = \sum_i output_limbs[i] * 2^LIMB_BITS
    //   s(x) = \sum_i quot_limbs[i] * 2^LIMB_BITS
    //   m(x) = \sum_i modulus_limbs[i] * 2^LIMB_BITS
    //
    // This polynomial should equal (2^LIMB_BITS - x) * q(x) where q is
    //
    //   q(x) = \sum_i aux_limbs[i] * 2^LIMB_BITS
    //
    // TODO: Same code as in generate above; refactor.
    for deg in 0..N_LIMBS {
        constr_poly[deg] = input0_limbs[deg] + input1_limbs[deg] - output_limbs[deg];

        // Invariant: i + j = deg
        for i in 0..=deg {
            let j = deg - i;
            constr_poly[deg] -= quot_limbs[i] * modulus_limbs[j];
        }
    }

    for (&c, d) in constr_poly.iter().zip(aux_constr_poly) {
        // Verify that the constr_poly and aux_constr_poly are equal;
        // we can then use the aux_constr_poly values in the following
        // constraint to reduce its degree
        yield_constr.constraint(is_op * (c - d));
    }

    // TODO: This is just copypasta from 'mul.rs'; really need to refactor.

    // This subtracts (2^LIMB_BITS - x) * q(x) from constr_poly.
    let mut final_poly = [P::ZEROS; N_LIMBS];
    let base = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    final_poly[0] = aux_constr_poly[0] - base * aux_limbs[0];
    for deg in 1..N_LIMBS {
        final_poly[deg] = aux_constr_poly[deg] - ((base * aux_limbs[deg]) - aux_limbs[deg - 1]);
    }

    // At this point constr_poly holds the coefficients of the
    // polynomial a(x) + b(x) - c(x) - s(x)*m(x) - (2^LIMB_BITS - x)*q(x).
    // The modular addition is valid if and only if all of those
    // coefficients are zero.
    for &c in &final_poly {
        // is_op, zero_mod and d all have degree 1, hence this has
        // total degree 3.
        yield_constr.constraint(is_op * zero_mod * c);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    range_check_error!(ADDMOD_INPUT_0, 16);
    range_check_error!(ADDMOD_INPUT_1, 16);
    range_check_error!(ADDMOD_MODULUS, 16);
    range_check_error!(ADDMOD_QUO_INPUT, 16);
    range_check_error!(ADDMOD_AUX_INPUT, 16, signed);
    range_check_error!(ADDMOD_AUX_OUTPUT_REDUCED, 16);
    range_check_error!(ADDMOD_AUX_CONSTR_POLY, 16);
    range_check_error!(ADDMOD_OUTPUT, 16);

    let is_addmod = lv[IS_ADDMOD];
    let input0_limbs = ADDMOD_INPUT_0.map(|c| lv[c]);
    let input1_limbs = ADDMOD_INPUT_1.map(|c| lv[c]);
    let modulus_limbs = ADDMOD_MODULUS.map(|c| lv[c]);
    let quot_limbs = ADDMOD_QUO_INPUT.map(|c| lv[c]);
    let aux_limbs = ADDMOD_AUX_INPUT.map(|c| lv[c]);
    let aux_output_reduced_limbs = ADDMOD_AUX_OUTPUT_REDUCED.map(|c| lv[c]);
    let aux_constr_poly_limbs = ADDMOD_AUX_CONSTR_POLY.map(|c| lv[c]);
    let output_limbs = ADDMOD_OUTPUT.map(|c| lv[c]);

    eval_packed_generic_addmod(
        is_addmod,
        input0_limbs,
        input1_limbs,
        modulus_limbs,
        output_limbs,
        quot_limbs,
        aux_limbs,
        aux_output_reduced_limbs,
        aux_constr_poly_limbs,
        yield_constr,
    );
}

pub(crate) fn eval_ext_circuit_addmod<F: RichField + Extendable<D>, const D: usize>(
    is_op: ExtensionTarget<D>,
    input0_limbs: [ExtensionTarget<D>; N_LIMBS],
    input1_limbs: [ExtensionTarget<D>; N_LIMBS],
    modulus_limbs: [ExtensionTarget<D>; N_LIMBS],
    output_limbs: [ExtensionTarget<D>; N_LIMBS],
    quot_limbs: [ExtensionTarget<D>; N_LIMBS],
    aux_limbs: [ExtensionTarget<D>; N_LIMBS],
    aux_output_reduced_limbs: [ExtensionTarget<D>; N_LIMBS],
    aux_constr_poly: [ExtensionTarget<D>; N_LIMBS],
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let modulus_limb_sum = builder.add_many_extension(modulus_limbs);
    let output_limb_sum = builder.add_many_extension(output_limbs);
    let zero_mod = builder.add_extension(modulus_limb_sum, output_limb_sum);

    let filter = builder.mul_extension(is_op, zero_mod);

    let is_less_than = builder.one_extension();
    eval_ext_circuit_lt(builder, yield_constr, filter, output_limbs,
                        modulus_limbs, aux_output_reduced_limbs, is_less_than);

    let zero = builder.zero_extension();
    let mut constr_poly = [zero; N_LIMBS];

    for deg in 0..N_LIMBS {
        let t = builder.add_extension(input0_limbs[deg], input1_limbs[deg]);
        constr_poly[deg] = builder.sub_extension(t, output_limbs[deg]);

        for i in 0..=deg {
            let j = deg - i;
            let t = builder.mul_extension(quot_limbs[i], modulus_limbs[j]);
            constr_poly[deg] = builder.sub_extension(constr_poly[deg], t);
        }
    }

    for (&c, d) in constr_poly.iter().zip(aux_constr_poly) {
        let t = builder.sub_extension(c, d);
        let t = builder.mul_extension(is_op, t);
        yield_constr.constraint(builder, t);
    }

    let mut final_poly = [zero; N_LIMBS];
    let base = F::from_canonical_u64(1 << LIMB_BITS);
    let t = builder.mul_const_extension(base, aux_limbs[0]);
    final_poly[0] = builder.sub_extension(constr_poly[0], t);
    for deg in 1..N_LIMBS {
        let t0 = builder.mul_const_extension(base, aux_limbs[deg]);
        let t1 = builder.sub_extension(t0, aux_limbs[deg - 1]);
        final_poly[deg] = builder.sub_extension(constr_poly[deg], t1);
    }

    for &c in &final_poly {
        let t = builder.mul_extension(is_op, c);
        yield_constr.constraint(builder, t);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_addmod = lv[IS_ADDMOD];
    let input0_limbs = ADDMOD_INPUT_0.map(|c| lv[c]);
    let input1_limbs = ADDMOD_INPUT_1.map(|c| lv[c]);
    let modulus_limbs = ADDMOD_MODULUS.map(|c| lv[c]);
    let quot_limbs = ADDMOD_QUO_INPUT.map(|c| lv[c]);
    let aux_limbs = ADDMOD_AUX_INPUT.map(|c| lv[c]);
    let aux_output_reduced_limbs = ADDMOD_AUX_OUTPUT_REDUCED.map(|c| lv[c]);
    let aux_constr_poly_limbs = ADDMOD_AUX_CONSTR_POLY.map(|c| lv[c]);
    let output_limbs = ADDMOD_OUTPUT.map(|c| lv[c]);

    eval_ext_circuit_addmod(
        is_addmod,
        input0_limbs,
        input1_limbs,
        modulus_limbs,
        output_limbs,
        quot_limbs,
        aux_limbs,
        aux_output_reduced_limbs,
        aux_constr_poly_limbs,
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
    fn generate_eval_consistency_not_addmod() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // if `IS_ADDMOD == 0`, then the constraints should be met even
        // if all values are garbage.
        lv[IS_ADDMOD] = F::ZERO;

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
    fn generate_eval_consistency_addmod() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // set `IS_ADDMOD == 1` and ensure all constraints are satisfied.
        lv[IS_ADDMOD] = F::ONE;
        for i in 0..N_RND_TESTS {
            // set inputs to random values
            for (&ai, &bi, &mi) in izip!(
                ADDMOD_INPUT_0.iter(),
                ADDMOD_INPUT_1.iter(),
                ADDMOD_MODULUS.iter()
            ) {
                lv[ai] = F::from_canonical_u16(rng.gen());
                lv[bi] = F::from_canonical_u16(rng.gen());
                lv[mi] = F::from_canonical_u16(rng.gen());
            }

            // For the second half of the tests, set the top 16 - start
            // digits to zero, so the modulus is much smaller than the
            // inputs.
            if i > N_RND_TESTS / 2 {
                // 1 <= start < N_LIMBS
                let start = (rng.gen::<usize>() % (N_LIMBS - 1)) + 1;
                for &mi in &ADDMOD_MODULUS[start..N_LIMBS] {
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

    #[test]
    fn addmod_zero_modulus() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // set `IS_ADDMOD == 1` and ensure all constraints are satisfied.
        lv[IS_ADDMOD] = F::ONE;

        for _i in 0..N_RND_TESTS {
            // set inputs to random values and the modulus to zero;
            // the output is defined to be zero when modulus is zero.
            for (&ai, &bi, &mi) in izip!(
                ADDMOD_INPUT_0.iter(),
                ADDMOD_INPUT_1.iter(),
                ADDMOD_MODULUS.iter()
            ) {
                lv[ai] = F::from_canonical_u16(rng.gen());
                lv[bi] = F::from_canonical_u16(rng.gen());
                lv[mi] = F::ZERO;
            }

            generate(&mut lv);

            // check that the correct output was generated
            assert!(ADDMOD_OUTPUT.iter().all(|&oi| lv[oi] == F::ZERO));

            let mut constraint_consumer = ConstraintConsumer::new(
                vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                GoldilocksField::ONE,
                GoldilocksField::ONE,
                GoldilocksField::ONE,
            );
            eval_packed_generic(&lv, &mut constraint_consumer);
            assert!(constraint_consumer
                .constraint_accs
                .iter()
                .all(|&acc| acc == F::ZERO));

            // Corrupt one output limb by setting it to a non-zero value
            let random_oi = ADDMOD_OUTPUT[rng.gen::<usize>() % N_LIMBS];
            lv[random_oi] = F::from_canonical_u16(rng.gen_range(1..u16::MAX));

            // TODO: Do I need a new constraint consumer?
            eval_packed_generic(&lv, &mut constraint_consumer);

            // Check that at least one of the constraints was non-zero
            assert!(constraint_consumer
                .constraint_accs
                .iter()
                .any(|&acc| acc != F::ZERO));
        }
    }
}
