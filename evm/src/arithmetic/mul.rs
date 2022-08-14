//! Support for the EVM MUL instruction.
//!
//! This crate verifies an EVM MUL instruction, which takes two
//! 256-bit inputs A and B, and produces a 256-bit output C satisfying
//!
//!    C = A*B (mod 2^256).
//!
//! Inputs A and B, and output C, are given as arrays of 16-bit
//! limbs. For example, if the limbs of A are a[0]...a[15], then
//!
//!    A = \sum_{i=0}^15 a[i] β^i,
//!
//! where β = 2^16. To verify that A, B and C satisfy the equation we
//! proceed as follows. Define a(x) = \sum_{i=0}^15 a[i] x^i (so A = a(β))
//! and similarly for b(x) and c(x). Then A*B = C (mod 2^256) if and only
//! if there exist polynomials q and m such that
//!
//!    a(x)*b(x) - c(x) - m(x)*x^16 - (x - β)*q(x) == 0.
//!
//! Because A, B and C are 256-bit numbers, the degrees of a, b and c
//! are (at most) 15. Thus deg(a*b) <= 30, so deg(m) <= 14 and deg(q)
//! <= 29. However, the fact that we're verifying the equality modulo
//! 2^256 means that we can ignore terms of degree >= 16, since for
//! them evaluating at β gives a factor of β^16 = 2^256 which is 0.
//!
//! Hence, to verify the equality, we don't need m(x) at all, and we
//! only need to know q(x) up to degree 14 (so that (x-β)*q(x) has
//! degree 15). On the other hand, the coefficients of q(x) can be as
//! large as 16*(β-2) or 20 bits.

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::columns::*;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

pub fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS]) {
    let input0_limbs = MUL_INPUT_0.map(|c| lv[c].to_canonical_u64());
    let input1_limbs = MUL_INPUT_1.map(|c| lv[c].to_canonical_u64());

    const MASK: u64 = (1u64 << LIMB_BITS) - 1u64;

    // Input and output have 16-bit limbs
    let mut aux_in_limbs = [0u64; N_LIMBS];
    let mut output_limbs = [0u64; N_LIMBS];

    let mut unreduced_prod = [0u64; N_LIMBS];

    // Column-wise pen-and-paper long multiplication on 16-bit limbs.
    // We have heaps of space at the top of each limb, so by
    // calculating column-wise (instead of the usual row-wise) we
    // avoid a bunch of carry propagation handling (at the expense of
    // slightly worse cache coherency), and it makes it easy to
    // calculate the coefficients of a(x)*b(x) (in unreduced_prod).
    let mut cy = 0u64;
    for col in 0..N_LIMBS {
        for i in 0..=col {
            // Invariant: i + j = col
            let j = col - i;
            let ai_x_bj = input0_limbs[i] * input1_limbs[j];
            unreduced_prod[col] += ai_x_bj;
        }
        let t = unreduced_prod[col] + cy;
        cy = t >> LIMB_BITS;
        output_limbs[col] = t & MASK;
    }
    // In principle, the last cy could be dropped because this is
    // multiplication modulo 2^256. However, we need it below for
    // aux_in_limbs to handle the fact that unreduced_prod will
    // inevitably contain a one digit's worth that is > 2^256.

    for (&c, output_limb) in MUL_OUTPUT.iter().zip(output_limbs) {
        lv[c] = F::from_canonical_u64(output_limb);
    }
    for deg in 0..N_LIMBS {
        // deg'th element <- a*b - c
        unreduced_prod[deg] -= output_limbs[deg];
    }

    // unreduced_prod is the coefficients of the polynomial a(x)*b(x) - c(x).
    // This must be zero when evaluated at x = B = 2^LIMB_BITS, hence it's
    // divisible by (B - x). If we write unreduced_prod as
    //
    //   a(x)*b(x) - c(x) = \sum_{i=0}^n p_i x^i
    //                    = (B - x) \sum_{i=0}^{n-1} q_i x^i
    //
    // then by comparing coefficients it is easy to see that
    //
    //   q_0 = p_0 / B  and  q_i = (p_i + q_{i-1}) / B
    //
    // for 0 < i < n-1 (and the divisions are exact).
    aux_in_limbs[0] = unreduced_prod[0] >> LIMB_BITS;
    for deg in 1..N_LIMBS - 1 {
        aux_in_limbs[deg] = (unreduced_prod[deg] + aux_in_limbs[deg - 1]) >> LIMB_BITS;
    }
    aux_in_limbs[N_LIMBS - 1] = cy;

    for deg in 0..N_LIMBS {
        let c = MUL_AUX_INPUT[deg];
        lv[c] = F::from_canonical_u64(aux_in_limbs[deg]);
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    range_check_error!(MUL_INPUT_0, 16);
    range_check_error!(MUL_INPUT_1, 16);
    range_check_error!(MUL_OUTPUT, 16);
    range_check_error!(MUL_AUX_INPUT, 20);

    let is_mul = lv[IS_MUL];
    let input0_limbs = MUL_INPUT_0.map(|c| lv[c]);
    let input1_limbs = MUL_INPUT_1.map(|c| lv[c]);
    let output_limbs = MUL_OUTPUT.map(|c| lv[c]);
    let aux_limbs = MUL_AUX_INPUT.map(|c| lv[c]);

    // Constraint poly holds the coefficients of the polynomial that
    // must be identically zero for this multiplication to be
    // verified. It is initialised to the /negative/ of the claimed
    // output.
    let mut constr_poly = [P::ZEROS; N_LIMBS];

    assert_eq!(constr_poly.len(), N_LIMBS);

    // After this loop constr_poly holds the coefficients of the
    // polynomial A(x)B(x) - C(x), where A, B and C are the polynomials
    //
    //   A(x) = \sum_i input0_limbs[i] * 2^LIMB_BITS
    //   B(x) = \sum_i input1_limbs[i] * 2^LIMB_BITS
    //   C(x) = \sum_i output_limbs[i] * 2^LIMB_BITS
    //
    // This polynomial should equal (2^LIMB_BITS - x) * Q(x) where Q is
    //
    //   Q(x) = \sum_i aux_limbs[i] * 2^LIMB_BITS
    //
    for col in 0..N_LIMBS {
        // Invariant: i + j = col
        for i in 0..=col {
            let j = col - i;
            constr_poly[col] += input0_limbs[i] * input1_limbs[j];
        }
        constr_poly[col] -= output_limbs[col];
    }

    // This subtracts (2^LIMB_BITS - x) * Q(x) from constr_poly.
    let base = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    constr_poly[0] -= base * aux_limbs[0];
    for deg in 1..N_LIMBS {
        constr_poly[deg] -= (base * aux_limbs[deg]) - aux_limbs[deg - 1];
    }

    // At this point constr_poly holds the coefficients of the
    // polynomial A(x)B(x) - C(x) - (x - 2^LIMB_BITS)*Q(x). The
    // multiplication is valid if and only if all of those
    // coefficients are zero.
    for &c in &constr_poly {
        yield_constr.constraint(is_mul * c);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_mul = lv[IS_MUL];
    let input0_limbs = MUL_INPUT_0.map(|c| lv[c]);
    let input1_limbs = MUL_INPUT_1.map(|c| lv[c]);
    let output_limbs = MUL_OUTPUT.map(|c| lv[c]);
    let aux_in_limbs = MUL_AUX_INPUT.map(|c| lv[c]);

    let zero = builder.zero_extension();
    let mut constr_poly = [zero; N_LIMBS]; // pointless init

    // Invariant: i + j = deg
    for col in 0..N_LIMBS {
        let mut acc = zero;
        for i in 0..=col {
            let j = col - i;
            acc = builder.mul_add_extension(input0_limbs[i], input1_limbs[j], acc);
        }
        constr_poly[col] = builder.sub_extension(acc, output_limbs[col]);
    }

    let base = F::from_canonical_u64(1 << LIMB_BITS);
    let t = builder.mul_const_extension(base, aux_in_limbs[0]);
    constr_poly[0] = builder.sub_extension(constr_poly[0], t);
    for deg in 1..N_LIMBS {
        let t0 = builder.mul_const_extension(base, aux_in_limbs[deg]);
        let t1 = builder.sub_extension(t0, aux_in_limbs[deg - 1]);
        constr_poly[deg] = builder.sub_extension(constr_poly[deg], t1);
    }

    for &c in &constr_poly {
        let filter = builder.mul_extension(is_mul, c);
        yield_constr.constraint(builder, filter);
    }
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
    fn generate_eval_consistency_not_mul() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // if `IS_MUL == 0`, then the constraints should be met even
        // if all values are garbage.
        lv[IS_MUL] = F::ZERO;

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
    fn generate_eval_consistency_mul() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // set `IS_MUL == 1` and ensure all constraints are satisfied.
        lv[IS_MUL] = F::ONE;
        // set inputs to random values
        for (&ai, bi) in MUL_INPUT_0.iter().zip(MUL_INPUT_1) {
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
