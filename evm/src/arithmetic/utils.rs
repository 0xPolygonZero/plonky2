use std::ops::{Add, AddAssign, Mul, Neg, Shr, Sub, SubAssign};

use log::error;
use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::arithmetic::columns::N_LIMBS;

/// Emit an error message regarding unchecked range assumptions.
/// Assumes the values in `cols` are `[cols[0], cols[0] + 1, ...,
/// cols[0] + cols.len() - 1]`.
pub(crate) fn _range_check_error<const RC_BITS: u32>(
    file: &str,
    line: u32,
    cols: &[usize],
    signedness: &str,
) {
    error!(
        "{}:{}: arithmetic unit skipped {}-bit {} range-checks on columns {}--{}: not yet implemented",
        line,
        file,
        RC_BITS,
        signedness,
        cols[0],
        cols[0] + cols.len() - 1
    );
}

#[macro_export]
macro_rules! range_check_error {
    ($cols:ident, $rc_bits:expr) => {
        $crate::arithmetic::utils::_range_check_error::<$rc_bits>(
            file!(),
            line!(),
            &$cols,
            "unsigned",
        );
    };
    ($cols:ident, $rc_bits:expr, signed) => {
        $crate::arithmetic::utils::_range_check_error::<$rc_bits>(
            file!(),
            line!(),
            &$cols,
            "signed",
        );
    };
    ([$cols:ident], $rc_bits:expr) => {
        $crate::arithmetic::utils::_range_check_error::<$rc_bits>(
            file!(),
            line!(),
            &[$cols],
            "unsigned",
        );
    };
}

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

pub(crate) fn pol_mul_wide_ext_circuit<
    F: RichField + Extendable<D>,
    const D: usize,
    const M: usize,
    const N: usize,
    const P: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    a: [ExtensionTarget<D>; M],
    b: [ExtensionTarget<D>; N],
) -> [ExtensionTarget<D>; P] {
    let zero = builder.zero_extension();
    let mut res = [zero; P];
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

pub(crate) fn pol_extend_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: [ExtensionTarget<D>; N_LIMBS],
) -> [ExtensionTarget<D>; 2 * N_LIMBS - 1] {
    let zero = builder.zero_extension();
    let mut zero_extend = [zero; 2 * N_LIMBS - 1];

    zero_extend[..N_LIMBS].copy_from_slice(&a);
    zero_extend
}

/// Given polynomial a(x) = \sum_{i=0}^{2N-2} a[i] x^i and an element
/// `root`, return b = (x - root) * a(x).
///
/// NB: Ignores element a[2 * N_LIMBS - 1], treating it as if it's 0.
pub(crate) fn pol_adjoin_root<T, U>(a: [T; 2 * N_LIMBS], root: U) -> [T; 2 * N_LIMBS]
where
    T: Add<Output = T> + Copy + Default + Mul<Output = T> + Sub<Output = T>,
    U: Copy + Mul<T, Output = T> + Neg<Output = U>,
{
    // \sum_i res[i] x^i = (x - root) \sum_i a[i] x^i. Comparing
    // coefficients, res[0] = -root*a[0] and
    // res[i] = a[i-1] - root * a[i]

    let mut res = [T::default(); 2 * N_LIMBS];
    res[0] = -root * a[0];
    for deg in 1..(2 * N_LIMBS - 1) {
        res[deg] = a[deg - 1] - (root * a[deg]);
    }
    // NB: We assume that a[2 * N_LIMBS - 1] = 0, so the last
    // iteration has no "* root" term.
    res[2 * N_LIMBS - 1] = a[2 * N_LIMBS - 2];
    res
}

pub(crate) fn pol_adjoin_root_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    a: [ExtensionTarget<D>; 2 * N_LIMBS],
    root: ExtensionTarget<D>,
) -> [ExtensionTarget<D>; 2 * N_LIMBS] {
    let zero = builder.zero_extension();
    let mut res = [zero; 2 * N_LIMBS];
    let t = builder.mul_extension(root, a[0]);
    res[0] = builder.sub_extension(zero, t);
    for deg in 1..(2 * N_LIMBS - 1) {
        let t = builder.mul_extension(root, a[deg]);
        res[deg] = builder.sub_extension(a[deg - 1], t);
    }
    // NB: We assumes that a[2 * N_LIMBS - 1] = 0, so the last
    // iteration has no "* root" term.
    res[2 * N_LIMBS - 1] = a[2 * N_LIMBS - 2];
    res
}

/// Given polynomial a(x) = \sum_{i=0}^{2N-1} a[i] x^i and a root of `a`
/// of the form 2^EXP, return q(x) satisfying a(x) = (x - root) * q(x).
///
/// NB: We do not verify that a(2^EXP) = 0; if this doesn't hold the
/// result is basically junk.
///
/// NB: The result could be returned in 2*N-1 elements, but we return
/// 2*N and set the last element to zero since the calling code
/// happens to require a result zero-extended to 2*N elements.
pub(crate) fn pol_remove_root_2exp<const EXP: usize, T>(a: [T; 2 * N_LIMBS]) -> [T; 2 * N_LIMBS]
where
    T: Copy + Default + Neg<Output = T> + Shr<usize, Output = T> + Sub<Output = T>,
{
    // By assumption β := 2^EXP is a root of `a`, i.e. (x - β) divides
    // `a`; if we write
    //
    //    a(x) = \sum_{i=0}^{2N-1} a[i] x^i
    //         = (x - β) \sum_{i=0}^{2N-2} q[i] x^i
    //
    // then by comparing coefficients it is easy to see that
    //
    //   q[0] = -a[0] / β  and  q[i] = (q[i-1] - a[i]) / β
    //
    // for 0 < i <= 2N-1 (and the divisions are exact).

    let mut q = [T::default(); 2 * N_LIMBS];
    q[0] = -(a[0] >> EXP);

    // NB: Last element of q is deliberately left equal to zero.
    for deg in 1..2 * N_LIMBS - 1 {
        q[deg] = (q[deg - 1] - a[deg]) >> EXP;
    }
    q
}
