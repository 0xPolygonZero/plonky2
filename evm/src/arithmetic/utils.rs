use std::ops::{Add, AddAssign, Mul, Neg, Range, Shr, Sub, SubAssign};

use ethereum_types::U256;
use plonky2::field::extension::Extendable;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use static_assertions::const_assert;

use crate::arithmetic::columns::{LIMB_BITS, N_LIMBS};

/// Return an array of `N` zeros of type T.
pub(crate) fn pol_zero<T, const N: usize>() -> [T; N]
where
    T: Copy + Default,
{
    // TODO: This should really be T::zero() from num::Zero, because
    // default() doesn't guarantee to initialise to zero (though in
    // our case it always does). However I couldn't work out how to do
    // that without touching half of the entire crate because it
    // involves replacing Field::is_zero() with num::Zero::is_zero()
    // which is used everywhere. Hence Default::default() it is.
    [T::default(); N]
}

/// a(x) += b(x), but must have deg(a) >= deg(b).
pub(crate) fn pol_add_assign<T>(a: &mut [T], b: &[T])
where
    T: AddAssign + Copy + Default,
{
    debug_assert!(a.len() >= b.len(), "expected {} >= {}", a.len(), b.len());
    for (a_item, b_item) in a.iter_mut().zip(b) {
        *a_item += *b_item;
    }
}

pub(crate) fn pol_add_assign_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: &mut [ExtensionTarget<D>],
    b: &[ExtensionTarget<D>],
) {
    debug_assert!(a.len() >= b.len(), "expected {} >= {}", a.len(), b.len());
    for (a_item, b_item) in a.iter_mut().zip(b) {
        *a_item = builder.add_extension(*a_item, *b_item);
    }
}

/// Return a(x) + b(x); returned array is bigger than necessary to
/// make the interface consistent with `pol_mul_wide`.
pub(crate) fn pol_add<T>(a: [T; N_LIMBS], b: [T; N_LIMBS]) -> [T; 2 * N_LIMBS - 1]
where
    T: Add<Output = T> + Copy + Default,
{
    let mut sum = pol_zero();
    for i in 0..N_LIMBS {
        sum[i] = a[i] + b[i];
    }
    sum
}

pub(crate) fn pol_add_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: [ExtensionTarget<D>; N_LIMBS],
    b: [ExtensionTarget<D>; N_LIMBS],
) -> [ExtensionTarget<D>; 2 * N_LIMBS - 1] {
    let zero = builder.zero_extension();
    let mut sum = [zero; 2 * N_LIMBS - 1];
    for i in 0..N_LIMBS {
        sum[i] = builder.add_extension(a[i], b[i]);
    }
    sum
}

/// Return a(x) - b(x); returned array is bigger than necessary to
/// make the interface consistent with `pol_mul_wide`.
pub(crate) fn pol_sub<T>(a: [T; N_LIMBS], b: [T; N_LIMBS]) -> [T; 2 * N_LIMBS - 1]
where
    T: Sub<Output = T> + Copy + Default,
{
    let mut diff = pol_zero();
    for i in 0..N_LIMBS {
        diff[i] = a[i] - b[i];
    }
    diff
}

pub(crate) fn pol_sub_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: [ExtensionTarget<D>; N_LIMBS],
    b: [ExtensionTarget<D>; N_LIMBS],
) -> [ExtensionTarget<D>; 2 * N_LIMBS - 1] {
    let zero = builder.zero_extension();
    let mut diff = [zero; 2 * N_LIMBS - 1];
    for i in 0..N_LIMBS {
        diff[i] = builder.sub_extension(a[i], b[i]);
    }
    diff
}

/// a(x) -= b(x), but must have deg(a) >= deg(b).
pub(crate) fn pol_sub_assign<T>(a: &mut [T], b: &[T])
where
    T: SubAssign + Copy,
{
    debug_assert!(a.len() >= b.len(), "expected {} >= {}", a.len(), b.len());
    for (a_item, b_item) in a.iter_mut().zip(b) {
        *a_item -= *b_item;
    }
}

pub(crate) fn pol_sub_assign_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: &mut [ExtensionTarget<D>],
    b: &[ExtensionTarget<D>],
) {
    debug_assert!(a.len() >= b.len(), "expected {} >= {}", a.len(), b.len());
    for (a_item, b_item) in a.iter_mut().zip(b) {
        *a_item = builder.sub_extension(*a_item, *b_item);
    }
}

/// Given polynomials a(x) and b(x), return a(x)*b(x).
///
/// NB: The caller is responsible for ensuring that no undesired
/// overflow occurs during the calculation of the coefficients of the
/// product.
pub(crate) fn pol_mul_wide<T>(a: [T; N_LIMBS], b: [T; N_LIMBS]) -> [T; 2 * N_LIMBS - 1]
where
    T: AddAssign + Copy + Mul<Output = T> + Default,
{
    let mut res = [T::default(); 2 * N_LIMBS - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            res[i + j] += ai * bj;
        }
    }
    res
}

pub(crate) fn pol_mul_wide_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: [ExtensionTarget<D>; N_LIMBS],
    b: [ExtensionTarget<D>; N_LIMBS],
) -> [ExtensionTarget<D>; 2 * N_LIMBS - 1] {
    let zero = builder.zero_extension();
    let mut res = [zero; 2 * N_LIMBS - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            res[i + j] = builder.mul_add_extension(ai, bj, res[i + j]);
        }
    }
    res
}

/// As for `pol_mul_wide` but the first argument has 2N elements and
/// hence the result has 3N-1.
pub(crate) fn pol_mul_wide2<T>(a: [T; 2 * N_LIMBS], b: [T; N_LIMBS]) -> [T; 3 * N_LIMBS - 1]
where
    T: AddAssign + Copy + Mul<Output = T> + Default,
{
    let mut res = [T::default(); 3 * N_LIMBS - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            res[i + j] += ai * bj;
        }
    }
    res
}

pub(crate) fn pol_mul_wide2_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: [ExtensionTarget<D>; 2 * N_LIMBS],
    b: [ExtensionTarget<D>; N_LIMBS],
) -> [ExtensionTarget<D>; 3 * N_LIMBS - 1] {
    let zero = builder.zero_extension();
    let mut res = [zero; 3 * N_LIMBS - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            res[i + j] = builder.mul_add_extension(ai, bj, res[i + j]);
        }
    }
    res
}

/// Given a(x) and b(x), return a(x)*b(x) mod 2^256.
pub(crate) fn pol_mul_lo<T, const N: usize>(a: [T; N], b: [T; N]) -> [T; N]
where
    T: AddAssign + Copy + Default + Mul<Output = T>,
{
    let mut res = pol_zero();
    for deg in 0..N {
        // Invariant: i + j = deg
        for i in 0..=deg {
            let j = deg - i;
            res[deg] += a[i] * b[j];
        }
    }
    res
}

pub(crate) fn pol_mul_lo_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: [ExtensionTarget<D>; N_LIMBS],
    b: [ExtensionTarget<D>; N_LIMBS],
) -> [ExtensionTarget<D>; N_LIMBS] {
    let zero = builder.zero_extension();
    let mut res = [zero; N_LIMBS];
    for deg in 0..N_LIMBS {
        for i in 0..=deg {
            let j = deg - i;
            res[deg] = builder.mul_add_extension(a[i], b[j], res[deg]);
        }
    }
    res
}

/// Adjoin M - N zeros to a, returning [a[0], a[1], ..., a[N-1], 0, 0, ..., 0].
pub(crate) fn pol_extend<T, const N: usize, const M: usize>(a: [T; N]) -> [T; M]
where
    T: Copy + Default,
{
    assert_eq!(M, 2 * N - 1);

    let mut zero_extend = pol_zero();
    zero_extend[..N].copy_from_slice(&a);
    zero_extend
}

/// Given polynomial a(x) = \sum_{i=0}^{N-2} a[i] x^i and an element
/// `root`, return b = (x - root) * a(x).
pub(crate) fn pol_adjoin_root<T, U, const N: usize>(a: [T; N], root: U) -> [T; N]
where
    T: Add<Output = T> + Copy + Default + Mul<Output = T> + Sub<Output = T>,
    U: Copy + Mul<T, Output = T> + Neg<Output = U>,
{
    // \sum_i res[i] x^i = (x - root) \sum_i a[i] x^i. Comparing
    // coefficients, res[0] = -root*a[0] and
    // res[i] = a[i-1] - root * a[i]

    let mut res = [T::default(); N];
    res[0] = -root * a[0];
    for deg in 1..N {
        res[deg] = a[deg - 1] - (root * a[deg]);
    }
    res
}

pub(crate) fn pol_adjoin_root_ext_circuit<
    F: RichField + Extendable<D>,
    const D: usize,
    const N: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    a: [ExtensionTarget<D>; N],
    root: ExtensionTarget<D>,
) -> [ExtensionTarget<D>; N] {
    let zero = builder.zero_extension();
    let mut res = [zero; N];
    // res[0] = NEG_ONE * root * a[0] + ZERO * zero
    res[0] = builder.mul_extension_with_const(F::NEG_ONE, root, a[0]);
    for deg in 1..N {
        // res[deg] = NEG_ONE * root * a[deg] + ONE * a[deg - 1]
        res[deg] = builder.arithmetic_extension(F::NEG_ONE, F::ONE, root, a[deg], a[deg - 1]);
    }
    res
}

/// Given polynomial a(x) = \sum_{i=0}^{N-1} a[i] x^i and a root of `a`
/// of the form 2^EXP, return q(x) satisfying a(x) = (x - root) * q(x).
///
/// NB: We do not verify that a(2^EXP) = 0; if this doesn't hold the
/// result is basically junk.
///
/// NB: The result could be returned in N-1 elements, but we return
/// N and set the last element to zero since the calling code
/// happens to require a result zero-extended to N elements.
pub(crate) fn pol_remove_root_2exp<const EXP: usize, T, const N: usize>(a: [T; N]) -> [T; N]
where
    T: Copy + Default + Neg<Output = T> + Shr<usize, Output = T> + Sub<Output = T>,
{
    // By assumption β := 2^EXP is a root of `a`, i.e. (x - β) divides
    // `a`; if we write
    //
    //    a(x) = \sum_{i=0}^{N-1} a[i] x^i
    //         = (x - β) \sum_{i=0}^{N-2} q[i] x^i
    //
    // then by comparing coefficients it is easy to see that
    //
    //   q[0] = -a[0] / β  and  q[i] = (q[i-1] - a[i]) / β
    //
    // for 0 < i <= N-1 (and the divisions are exact).

    let mut q = [T::default(); N];
    q[0] = -(a[0] >> EXP);

    // NB: Last element of q is deliberately left equal to zero.
    for deg in 1..N - 1 {
        q[deg] = (q[deg - 1] - a[deg]) >> EXP;
    }
    q
}

/// Read the range `value_idxs` of values from `lv` into an array of
/// length `N`. Panics if the length of the range is not `N`.
pub(crate) fn read_value<const N: usize, T: Copy>(lv: &[T], value_idxs: Range<usize>) -> [T; N] {
    lv[value_idxs].try_into().unwrap()
}

/// Read the range `value_idxs` of values from `lv` into an array of
/// length `N`, interpreting the values as `i64`s. Panics if the
/// length of the range is not `N`.
pub(crate) fn read_value_i64_limbs<const N: usize, F: PrimeField64>(
    lv: &[F],
    value_idxs: Range<usize>,
) -> [i64; N] {
    let limbs: [_; N] = lv[value_idxs].try_into().unwrap();
    limbs.map(|c| c.to_canonical_u64() as i64)
}

#[inline]
/// Turn a 64-bit integer into 4 16-bit limbs and convert them to field elements.
fn u64_to_array<F: Field>(out: &mut [F], x: u64) {
    const_assert!(LIMB_BITS == 16);
    debug_assert!(out.len() == 4);

    out[0] = F::from_canonical_u16(x as u16);
    out[1] = F::from_canonical_u16((x >> 16) as u16);
    out[2] = F::from_canonical_u16((x >> 32) as u16);
    out[3] = F::from_canonical_u16((x >> 48) as u16);
}

/// Turn a 256-bit integer into 16 16-bit limbs and convert them to field elements.
// TODO: Refactor/replace u256_limbs in evm/src/util.rs
pub(crate) fn u256_to_array<F: Field>(out: &mut [F], x: U256) {
    const_assert!(N_LIMBS == 16);
    debug_assert!(out.len() == N_LIMBS);

    u64_to_array(&mut out[0..4], x.0[0]);
    u64_to_array(&mut out[4..8], x.0[1]);
    u64_to_array(&mut out[8..12], x.0[2]);
    u64_to_array(&mut out[12..16], x.0[3]);
}
