//! Implementation of the shift left instruction (SHL)
//!
//! SHL takes a 256-bit input value x and an 8-bit displacement d and
//! returns x << d (mod 2^256).  The implementation delegates most
//! work to the MUL instruction. The displacement d is provided as the
//! 8-bit value d and the 256-bit value 2^d; then SHL returns MUL(x, 2^d).

use std::ops::Range;

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::columns::*;
use crate::arithmetic::mul::{generate_mul, verify_mul_circuit, verify_mul_packed};
use crate::arithmetic::utils::{read_value, read_value_u64_limbs};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

pub(crate) fn generate_2exp<F: RichField>(
    lv: &mut [F; NUM_ARITH_COLUMNS],
    exp: Range<usize>,
    pow_exp: Range<usize>,
    exp_limb0_quo256: usize,
    exp_mod256_quo16: usize,
    exp_mod16_bits: Range<usize>,
    pow_exp_aux_0: usize,
    pow_exp_aux_1: usize,
    pow_exp_aux_2: usize,
) {
    let exp: [u64; N_LIMBS] = read_value_u64_limbs(lv, exp);
    let exp_mod16 = exp[0] % 16;
    let bits = [
        exp_mod16 & 1,
        (exp_mod16 >> 1) & 1,
        (exp_mod16 >> 2) & 1,
        (exp_mod16 >> 3) & 1,
    ];
    lv[pow_exp_aux_0] = F::from_canonical_u64((bits[0] + 1) * (3 * bits[1] + 1));
    lv[pow_exp_aux_1] = F::from_canonical_u64((15 * bits[2] + 1) * (255 * bits[3] + 1));
    lv[pow_exp_aux_2] = lv[pow_exp_aux_0] * lv[pow_exp_aux_1];
    lv[exp_mod16_bits].copy_from_slice(&bits.map(|b| F::from_canonical_u64(b)));

    let limb0_quo256 = exp[0] / 256;
    lv[exp_limb0_quo256] = F::from_canonical_u64(limb0_quo256);
    let mod256_quo16 = (exp[0] % 256) / 16;
    lv[exp_mod256_quo16] = F::from_canonical_u64(mod256_quo16);

    let mut pow = [0u64; N_LIMBS];
    if limb0_quo256 + exp[1..].iter().sum::<u64>() == 0 {
        // Z = ceil(exp/256) = 0, so 2^exp = 2^E * 2^(Q * 16)
        pow[mod256_quo16 as usize] = 1u64 << exp_mod16;
    }
    lv[pow_exp].copy_from_slice(&pow.map(|c| F::from_canonical_u64(c)));
}

pub fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS], is_op: usize) {
    generate_2exp(
        lv,
        SHIFT_INPUT_EXP,
        SHIFT_INPUT_POW_EXP,
        SHIFT_INPUT_EXP_LIMB0_QUO256,
        SHIFT_INPUT_EXP_MOD256_QUO16,
        SHIFT_INPUT_EXP_MOD16_BITS,
        SHIFT_INPUT_POW_EXP_AUX_0,
        SHIFT_INPUT_POW_EXP_AUX_1,
        SHIFT_INPUT_POW_EXP_AUX_2,
    );
    match is_op {
        IS_SHL => generate_mul(
            lv,
            SHIFT_INPUT_VALUE,
            SHIFT_INPUT_POW_EXP,
            SHIFT_OUTPUT,
            SHIFT_AUX_INPUT,
        ),
        //IS_SHR => generate_div(lv, SHIFT_INPUT_VALUE, SHIFT_INPUT_POW_EXP, SHIFT_OUTPUT, SHIFT_AUX_INPUT, ...),
        IS_SHR => todo!(),
        _ => panic!("unrecognized shift instruction"),
    }
}

pub(crate) fn verify_2exp_packed<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    exp: Range<usize>,
    pow_exp: Range<usize>,
    exp_limb0_quo256: usize,
    exp_mod256_quo16: usize,
    exp_mod16_bits: Range<usize>,
    pow_exp_aux_0: usize,
    pow_exp_aux_1: usize,
    pow_exp_aux_2: usize,
) {
    // exp ∈ [0, 2^256).
    // pow_exp ?= 2^exp (mod 2^256) ∈ [0, 2^256). For exp >= 256, pow_exp = 0.
    //
    // Write exp = E + Q*16 + Z*256
    //           = (e[0] + e[1]*2 + e[2]*4 + e[3]*8) + Q*16 + Z*256
    //
    // where
    //  - E = exp mod 16, e[i] ∈ {0, 1}
    //  - Q = ceil((exp mod 256) / 16) ∈ [0, 16)
    //  - Z = ceil(exp / 256)
    //
    // So we need to verify that
    //
    //   pow_exp = 2^exp (mod 2^256)
    //           = 2^E * 2^(Q * 16) * 2^(Z * 256)  (mod 2^256)
    //
    // If Z > 0, then pow_exp = 0.
    // Otherwise, pow_exp has the value 2^E at the Qth limb and zero
    // elsewhere.
    //
    // Below we write exp_mod256_quo16 for Q and exp_mod16_bits for
    // E. We also write exp_limb0_quo256 for 256*Z mod 2^16.

    let exp_limb0_quo256 = lv[exp_limb0_quo256];
    let exp_mod256_quo16 = lv[exp_mod256_quo16];
    let exp_mod16_bits: [P; 4] = read_value(lv, exp_mod16_bits);

    // Check that every "bit" of exp_mod16_bits is 0 or 1
    for b in exp_mod16_bits {
        yield_constr.constraint(filter * (b * b - b));
    }

    // Check that exp_mod256_quo16 ∈ [0, 16)
    // FIXME range_check_error!(SHIFT_INPUT_EXP_MOD256_QUO16, 4);

    // Check that
    // exp[0] = exp_mod16 + 16 * exp_mod256_quo16 + exp_limb0_quo256 * 256
    let expected_exp_limb0 = exp_mod16_bits[0]
        + exp_mod16_bits[1] * P::Scalar::TWO
        + exp_mod16_bits[2] * P::Scalar::from_canonical_u64(4)
        + exp_mod16_bits[3] * P::Scalar::from_canonical_u64(8)
        + exp_mod256_quo16 * P::Scalar::from_canonical_u64(16)
        + exp_limb0_quo256 * P::Scalar::from_canonical_u64(256);
    yield_constr.constraint(filter * (lv[exp.start] - expected_exp_limb0));

    // This value is zero if ceil(exp/256) = 0 and nonzero
    // otherwise. Because of the range checks on exp, it is bounded
    // above by 2^8 + 15 * 2^16.
    // NB: I don't understand why I do lv[exp].into_iter().skip(1).sum::<P>();
    let sum_tail = |ps: &[P]| ps.iter().skip(1).fold(P::ZEROS, |acc, &x| acc + x);
    let exp_quo256_is_nonzero = exp_limb0_quo256 + sum_tail(&lv[exp]);

    // If exp_quo256_is_nonzero is zero, then exp < 256 and the
    // nonzero limb index is just exp_mod256_quo16.
    //
    // Otherwise, exp_quo256_is_nonzero is nonzero (signifying that
    // exp >= 256), so multiplying by 16 gives something >= 16 (but
    // won't overflow due to bound described above). Hence nz_limb_idx
    // is >= 16, and so all limbs of pow_exp are forced to be zero by
    // the following for loop.
    let nz_limb_idx = exp_mod256_quo16 + exp_quo256_is_nonzero * P::Scalar::from_canonical_u64(16);

    // Check that the only non-zero limb of pow_exp is the
    // nz_limb_idx-th limb (it doesn't have to be nonzero though).
    let mut limb_sum = P::ZEROS;
    for (i, &limb_i) in (0..).zip(lv[pow_exp].iter()) {
        let i = P::Scalar::from_canonical_u64(i);
        yield_constr.constraint(filter * limb_i * (nz_limb_idx - i));
        limb_sum += limb_i;
    }

    // Obtain the nz_limb_idx-th limb (assuming it's index < 16): As all
    // *other* limbs are zero, we can just use the limb sum.
    let nz_limb = limb_sum;

    // Check that nz_limb = 2^E where E = \sum_{i=0}^3 exp_mod16_bits[i] * 2^i.
    // To do this, observe that
    //
    // 2^E = \prod_i=0^3 (2^(2^i) if exp_mod16_bits[i] = 1 else 1)
    //     = \prod_i=0^3 ((2^(2^i) - 1) * exp_mod16_bits[i] + 1)
    //
    // We verify the degree 4 product using the auxiliary values
    //
    //    pow_exp_aux_0 = \prod_i=0^1 ((2^i - 1) * exp_mod16_bits[i] + 1)
    //    pow_exp_aux_1 = \prod_i=2^3 ((2^i - 1) * exp_mod16_bits[i] + 1)
    //
    // Then
    //
    //    2^E = pow_exp_aux_0 * pow_exp_aux_1 = pow_exp_aux_2

    // c[i-1] = 2^(2^i) - 1
    let c = [
        P::Scalar::from_canonical_u64(3),
        P::Scalar::from_canonical_u64(15),
        P::Scalar::from_canonical_u64(255),
    ];

    let pow_exp_aux_0 = lv[pow_exp_aux_0];
    let constr1 = (exp_mod16_bits[0] + P::ONES) * (exp_mod16_bits[1] * c[0] + P::ONES);
    yield_constr.constraint(filter * (constr1 - pow_exp_aux_0));

    let pow_exp_aux_1 = lv[pow_exp_aux_1];
    let constr2 = (exp_mod16_bits[2] * c[1] + P::ONES) * (exp_mod16_bits[3] * c[2] + P::ONES);
    yield_constr.constraint(filter * (constr2 - pow_exp_aux_1));

    let pow_exp_aux_2 = lv[pow_exp_aux_2];
    let constr3 = pow_exp_aux_0 * pow_exp_aux_1;
    yield_constr.constraint(filter * (constr3 - pow_exp_aux_2));

    yield_constr.constraint(filter * nz_limb * (pow_exp_aux_2 - nz_limb));
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // FIXME: This function needs range checks?!
    let filter = lv[IS_SHL] + lv[IS_SHR];

    verify_2exp_packed(
        lv,
        yield_constr,
        filter,
        SHIFT_INPUT_EXP,
        SHIFT_INPUT_POW_EXP,
        SHIFT_INPUT_EXP_LIMB0_QUO256,
        SHIFT_INPUT_EXP_MOD256_QUO16,
        SHIFT_INPUT_EXP_MOD16_BITS,
        SHIFT_INPUT_POW_EXP_AUX_0,
        SHIFT_INPUT_POW_EXP_AUX_1,
        SHIFT_INPUT_POW_EXP_AUX_2,
    );

    verify_mul_packed(
        lv,
        yield_constr,
        lv[IS_SHL],
        SHIFT_INPUT_VALUE,
        SHIFT_INPUT_POW_EXP,
        SHIFT_OUTPUT,
        SHIFT_AUX_INPUT,
    );
    //verify_div_packed(lv, yield_constr, lv[IS_SHR], SHIFT_INPUT_VALUE, SHIFT_INPUT_POW_EXP, SHIFT_OUTPUT, SHIFT_AUX_INPUT, ...);
}

pub(crate) fn verify_2exp_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    exp: Range<usize>,
    pow_exp: Range<usize>,
    exp_limb0_quo256: usize,
    exp_mod256_quo16: usize,
    exp_mod16_bits: Range<usize>,
    pow_exp_aux_0: usize,
    pow_exp_aux_1: usize,
    pow_exp_aux_2: usize,
) {
    let exp_limb0_quo256 = lv[exp_limb0_quo256];
    let exp_mod256_quo16 = lv[exp_mod256_quo16];
    let exp_mod16_bits: [_; 4] = read_value(lv, exp_mod16_bits);
    for b in exp_mod16_bits {
        let t = builder.mul_sub_extension(b, b, b);
        let t = builder.mul_extension(filter, t);
        yield_constr.constraint(builder, t);
    }

    let mut t = exp_mod16_bits[0];
    t = builder.mul_const_add_extension(F::TWO, exp_mod16_bits[1], t);
    t = builder.mul_const_add_extension(F::from_canonical_u64(4), exp_mod16_bits[2], t);
    t = builder.mul_const_add_extension(F::from_canonical_u64(8), exp_mod16_bits[3], t);
    t = builder.mul_const_add_extension(F::from_canonical_u64(16), exp_mod256_quo16, t);

    t = builder.mul_const_add_extension(F::from_canonical_u64(256), exp_limb0_quo256, t);
    let expected_exp_limb0 = t;
    let c = builder.sub_extension(lv[exp.start], expected_exp_limb0);
    let c = builder.mul_extension(filter, c);
    yield_constr.constraint(builder, c);

    let exp_tail: [_; 15] = lv[exp].try_into().unwrap();
    let sum_tail = builder.add_many_extension(exp_tail);
    let exp_quo256_is_nonzero = builder.add_extension(exp_limb0_quo256, sum_tail);
    let nz_limb_idx = builder.mul_const_add_extension(
        F::from_canonical_u64(16),
        exp_quo256_is_nonzero,
        exp_mod256_quo16,
    );

    let mut limb_sum = builder.zero_extension();
    for (i, &limb_i) in (0..).zip(lv[pow_exp].iter()) {
        let i = F::from_canonical_u64(i);
        // t = limb_i * (nz_limb_idx - i)
        let t = builder.arithmetic_extension(F::ONE, -i, limb_i, nz_limb_idx, limb_i);
        let c = builder.mul_extension(filter, t);
        yield_constr.constraint(builder, c);
        limb_sum = builder.add_extension(limb_sum, limb_i);
    }

    let nz_limb = limb_sum;
    let c = [
        F::from_canonical_u64(3),
        F::from_canonical_u64(15),
        F::from_canonical_u64(255),
    ];

    let pow_exp_aux_0 = lv[pow_exp_aux_0];
    let t = builder.add_const_extension(exp_mod16_bits[0], F::ONE);
    let constr1 = builder.arithmetic_extension(c[0], F::ONE, t, exp_mod16_bits[1], t);
    let t = builder.sub_extension(constr1, pow_exp_aux_0);
    let t = builder.mul_extension(filter, t);
    yield_constr.constraint(builder, t);

    let pow_exp_aux_1 = lv[pow_exp_aux_1];
    let one = builder.one_extension();
    let t = builder.mul_const_add_extension(c[1], exp_mod16_bits[2], one);
    let constr2 = builder.arithmetic_extension(c[2], F::ONE, t, exp_mod16_bits[3], t);
    let t = builder.sub_extension(constr2, pow_exp_aux_1);
    let t = builder.mul_extension(filter, t);
    yield_constr.constraint(builder, t);

    let pow_exp_aux_2 = lv[pow_exp_aux_2];
    let t = builder.mul_sub_extension(pow_exp_aux_0, pow_exp_aux_1, pow_exp_aux_2);
    let t = builder.mul_extension(filter, t);
    yield_constr.constraint(builder, t);

    let t = builder.sub_extension(pow_exp_aux_2, nz_limb);
    let t = builder.mul_extension(nz_limb, t);
    let t = builder.mul_extension(filter, t);
    yield_constr.constraint(builder, t);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = builder.add_extension(lv[IS_SHL], lv[IS_SHR]);

    verify_2exp_circuit(
        builder,
        lv,
        yield_constr,
        filter,
        SHIFT_INPUT_EXP,
        SHIFT_INPUT_POW_EXP,
        SHIFT_INPUT_EXP_LIMB0_QUO256,
        SHIFT_INPUT_EXP_MOD256_QUO16,
        SHIFT_INPUT_EXP_MOD16_BITS,
        SHIFT_INPUT_POW_EXP_AUX_0,
        SHIFT_INPUT_POW_EXP_AUX_1,
        SHIFT_INPUT_POW_EXP_AUX_2,
    );

    verify_mul_circuit(
        builder,
        lv,
        yield_constr,
        lv[IS_SHL],
        SHIFT_INPUT_VALUE,
        SHIFT_INPUT_POW_EXP,
        SHIFT_OUTPUT,
        SHIFT_AUX_INPUT,
    );
    //verify_div_circuit(builder, lv, yield_constr, lv[IS_SHR], SHIFT_INPUT_VALUE, SHIFT_INPUT_POW_EXP, SHIFT_OUTPUT, SHIFT_AUX_INPUT);
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
    fn generate_eval_consistency_not_shift() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // if `IS_SHL == 0`, then the constraints should be met even if
        // all values are garbage. `eval_packed_generic` handles IS_SHR
        // at the same time, so we check both at once.
        lv[IS_SHL] = F::ZERO;
        lv[IS_SHR] = F::ZERO;

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
    fn generate_eval_consistency_shift() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));
        const N_ITERS: usize = 1000;

        for iter in 0..N_ITERS {
            //for (op, other_op) in [(IS_SHL, IS_SHR), (IS_SHR, IS_SHL)] {
            let (op, other_op) = (IS_SHL, IS_SHR);
            {
                // set op == 1 and ensure all constraints are satisfied.
                // we have to explicitly set the other op to zero since both
                // are treated by the call.
                lv[op] = F::ONE;
                lv[other_op] = F::ZERO;

                // set inputs to random values
                for (i, (ai, bi)) in (0..).zip(SHIFT_INPUT_VALUE.zip(SHIFT_INPUT_EXP)) {
                    // input is random
                    lv[ai] = F::from_canonical_u16(rng.gen());

                    // exponent is completely random for 20% of the iterations,
                    // and is random but less than 256 for the other 80%.
                    lv[bi] = if iter > N_ITERS / 5 {
                        if i == 0 {
                            F::from_canonical_u16(rng.gen::<u16>() % 256)
                        } else {
                            F::ZERO
                        }
                    } else {
                        F::from_canonical_u16(rng.gen())
                    };
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
