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

use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
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
    let mut aux_in_limbs = [0u64; N_LIMBS - 1];
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
        for i in 0..col {
            // Invariant: i + j = col
            let j = col - i;
            let ai_x_bj = input0_limbs[i] * input1_limbs[j];
            unreduced_prod[col] += ai_x_bj;
        }
        let t = unreduced_prod[col] + cy;
        cy = t >> LIMB_BITS;
        output_limbs[col] = t & MASK;
    }
    // last cy is dropped because this is multiplication modulo 2^256.

    for (&c, &output_limb) in MUL_OUTPUT.iter().zip(output_limbs.iter()) {
        lv[c] = F::from_canonical_u64(output_limb);
    }
    for deg in 0..N_LIMBS {
        // deg'th element <- a*b - c
        unreduced_prod[deg] -= output_limbs[deg];
    }

    // unreduced_prod is the coefficients of the polynomial a(x)*b(x) - c(x).
    // This must be zero when evaluated at x = B = 2^LIMB_BITS, hence it's
    // divisible by (x - B). If we write unreduced_prod as
    //
    //   a(x)*b(x) - c(x) = \sum_{i=0}^n p_i x^i
    //                    = (x - B) \sum_{i=0}^{n-1} q_i x^i
    //
    // then by comparing coefficients it is easy to see that
    //
    //   q_{n-1} = p_n  and  q_{i-1} = p_i + q_i*B for 0 < i < n-1.
    //
    aux_in_limbs[N_LIMBS - 2] = unreduced_prod[N_LIMBS - 1];
    for deg in (1..N_LIMBS - 1).rev() {
        aux_in_limbs[deg - 1] = unreduced_prod[deg] + (aux_in_limbs[deg] << LIMB_BITS);
    }

    for deg in 0..N_LIMBS - 1 {
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
    let aux_limbs = MUL_AUX_INPUT.map(|c| lv[c]);

    // Constraint poly holds the coefficients of the polynomial that
    // must be identically zero for this multiplication to be
    // verified. It is initialised to the /negative/ of the claimed
    // output.
    let mut constr_poly = MUL_OUTPUT.map(|c| -lv[c]);

    debug_assert_eq!(constr_poly.len(), N_LIMBS);

    // After this loop constr_poly holds the coefficients of the
    // polynomial A(x)B(x) - C(x), where A, B and C are the polynomials
    //
    //   A(x) = \sum_i input0_limbs[i] * 2^LIMB_BITS
    //   B(x) = \sum_i input1_limbs[i] * 2^LIMB_BITS
    //   C(x) = \sum_i output_limbs[i] * 2^LIMB_BITS
    //
    // This polynomial should equal (x - 2^LIMB_BITS) * Q(x) where Q is
    //
    //   Q(x) = \sum_i aux_limbs[i] * 2^LIMB_BITS
    //
    for deg in 0..N_LIMBS {
        // Invariant: i + j = deg
        for i in 0..deg {
            let j = deg - i;
            constr_poly[deg] += input0_limbs[i] * input1_limbs[j];
        }
    }

    // This subtracts (x - 2^LIMB_BITS) * Q(x) from constr_poly.
    let base = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    constr_poly[0] += base * aux_limbs[0];
    for deg in 1..N_LIMBS - 1 {
        constr_poly[deg] += (base * aux_limbs[deg]) - aux_limbs[deg - 1];
    }
    constr_poly[N_LIMBS - 1] -= aux_limbs[N_LIMBS - 2];

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
    for deg in 0..N_LIMBS {
        let mut acc = zero;
        for i in 0..deg {
            let j = deg - i;
            acc = builder.mul_add_extension(input0_limbs[i], input1_limbs[j], acc);
        }
        constr_poly[deg] = builder.sub_extension(acc, output_limbs[deg]);
    }

    let base = F::from_canonical_u64(1 << LIMB_BITS);
    constr_poly[0] = builder.mul_const_add_extension(base, aux_in_limbs[0], constr_poly[0]);
    for deg in 1..N_LIMBS - 1 {
        constr_poly[deg] =
            builder.mul_const_add_extension(base, aux_in_limbs[deg], constr_poly[deg]);
        constr_poly[deg] = builder.sub_extension(constr_poly[deg], aux_in_limbs[deg - 1]);
    }
    constr_poly[N_LIMBS] = builder.sub_extension(constr_poly[N_LIMBS], aux_in_limbs[N_LIMBS - 1]);

    for &c in &constr_poly {
        let filter = builder.mul_extension(is_mul, c);
        yield_constr.constraint(builder, filter);
    }
}
