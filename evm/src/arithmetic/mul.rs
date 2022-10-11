//! Support for the EVM MUL instruction.
//!
//! This crate verifies an EVM MUL instruction, which takes two
//! 256-bit inputs A and B, and produces a 256-bit output C satisfying
//!
//!    C = A*B (mod 2^256),
//!
//! i.e. C is the lower half of the usual long multiplication
//! A*B. Inputs A and B, and output C, are given as arrays of 16-bit
//! limbs. For example, if the limbs of A are a[0]...a[15], then
//!
//!    A = \sum_{i=0}^15 a[i] β^i,
//!
//! where β = 2^16 = 2^LIMB_BITS. To verify that A, B and C satisfy
//! the equation we proceed as follows. Define
//!
//!    a(x) = \sum_{i=0}^15 a[i] x^i
//!
//! (so A = a(β)) and similarly for b(x) and c(x). Then A*B = C (mod
//! 2^256) if and only if there exists q such that the polynomial
//!
//!    a(x) * b(x) - c(x) - x^16 * q(x)
//!
//! is zero when evaluated at x = β, i.e. it is divisible by (x - β);
//! equivalently, there exists a polynomial s (representing the
//! carries from the long multiplication) such that
//!
//!    a(x) * b(x) - c(x) - x^16 * q(x) - (x - β) * s(x) == 0
//!
//! As we only need the lower half of the product, we can omit q(x)
//! since it is multiplied by the modulus β^16 = 2^256. Thus we only
//! need to verify
//!
//!    a(x) * b(x) - c(x) - (x - β) * s(x) == 0
//!
//! In the code below, this "constraint polynomial" is constructed in
//! the variable `constr_poly`. It must be identically zero for the
//! multiplication operation to be verified, or, equivalently, each of
//! its coefficients must be zero. The variable names of the
//! constituent polynomials are (writing N for N_LIMBS=16):
//!
//!   a(x) = \sum_{i=0}^{N-1} input0[i] * x^i
//!   b(x) = \sum_{i=0}^{N-1} input1[i] * x^i
//!   c(x) = \sum_{i=0}^{N-1} output[i] * x^i
//!   s(x) = \sum_i^{2N-3} aux[i] * x^i
//!
//! Because A, B and C are 256-bit numbers, the degrees of a, b and c
//! are (at most) 15. Thus deg(a*b) <= 30 and deg(s) <= 29; however,
//! as we're only verifying the lower half of A*B, we only need to
//! know s(x) up to degree 14 (so that (x - β)*s(x) has degree 15). On
//! the other hand, the coefficients of s(x) can be as large as
//! 16*(β-2) or 20 bits.
//!
//! Note that, unlike for the general modular multiplication (see the
//! file `modular.rs`), we don't need to check that output is reduced,
//! since any value of output is less than β^16 and is hence reduced.

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::columns::*;
use crate::arithmetic::utils::*;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::range_check_error;

pub fn generate<F: RichField>(lv: &mut [F; NUM_ARITH_COLUMNS]) {
    let input0 = read_value_i64_limbs(lv, MUL_INPUT_0);
    let input1 = read_value_i64_limbs(lv, MUL_INPUT_1);

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

    lv[MUL_OUTPUT].copy_from_slice(&output_limbs.map(|c| F::from_canonical_i64(c)));
    pol_sub_assign(&mut unreduced_prod, &output_limbs);

    let mut aux_limbs = pol_remove_root_2exp::<LIMB_BITS, _, N_LIMBS>(unreduced_prod);
    aux_limbs[N_LIMBS - 1] = -cy;

    lv[MUL_AUX_INPUT].copy_from_slice(&aux_limbs.map(|c| F::from_noncanonical_i64(c)));
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
    let input0_limbs = read_value::<N_LIMBS, _>(lv, MUL_INPUT_0);
    let input1_limbs = read_value::<N_LIMBS, _>(lv, MUL_INPUT_1);
    let output_limbs = read_value::<N_LIMBS, _>(lv, MUL_OUTPUT);
    let aux_limbs = read_value::<N_LIMBS, _>(lv, MUL_AUX_INPUT);

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
    let mut constr_poly = pol_mul_lo(input0_limbs, input1_limbs);
    pol_sub_assign(&mut constr_poly, &output_limbs);

    // This subtracts (x - β) * s(x) from constr_poly.
    let base = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    pol_sub_assign(&mut constr_poly, &pol_adjoin_root(aux_limbs, base));

    // At this point constr_poly holds the coefficients of the
    // polynomial a(x)b(x) - c(x) - (x - β)*s(x). The
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
    let input0_limbs = read_value::<N_LIMBS, _>(lv, MUL_INPUT_0);
    let input1_limbs = read_value::<N_LIMBS, _>(lv, MUL_INPUT_1);
    let output_limbs = read_value::<N_LIMBS, _>(lv, MUL_OUTPUT);
    let aux_limbs = read_value::<N_LIMBS, _>(lv, MUL_AUX_INPUT);

    let mut constr_poly = pol_mul_lo_ext_circuit(builder, input0_limbs, input1_limbs);
    pol_sub_assign_ext_circuit(builder, &mut constr_poly, &output_limbs);

    let base = builder.constant_extension(F::Extension::from_canonical_u64(1 << LIMB_BITS));
    let rhs = pol_adjoin_root_ext_circuit(builder, aux_limbs, base);
    pol_sub_assign_ext_circuit(builder, &mut constr_poly, &rhs);

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

    const N_RND_TESTS: usize = 1000;

    // TODO: Should be able to refactor this test to apply to all operations.
    #[test]
    fn generate_eval_consistency_not_mul() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // if `IS_MUL == 0`, then the constraints should be met even
        // if all values are garbage.
        lv[IS_MUL] = F::ZERO;

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
    fn generate_eval_consistency_mul() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::rand_from_rng(&mut rng));

        // set `IS_MUL == 1` and ensure all constraints are satisfied.
        lv[IS_MUL] = F::ONE;

        for _i in 0..N_RND_TESTS {
            // set inputs to random values
            for (ai, bi) in MUL_INPUT_0.zip(MUL_INPUT_1) {
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
