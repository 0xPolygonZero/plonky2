use std::ops::{Add, AddAssign, Mul, Neg, Shr, Sub, SubAssign};
use log::error;

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

pub(crate) fn pol_zero<T, const N: usize>() -> [T; N]
where
    T: Copy + Default,
{
    // NB: This should really be T::zero() from num::Zero, because
    // default() doesn't guarantee to initialise to zero (though in
    // our case it always does). However I couldn't work out how to do
    // that without touching half of the entire crate because it
    // involves replacing Field::is_zero() with num::Zero::is_zero()
    // which is used everywhere. Hence Default::default() it is.
    [T::default(); N]
}

pub(crate) fn pol_add_assign<T>(a: &mut [T], b: &[T])
where
    T: AddAssign + Copy + Default,
{
    debug_assert!(a.len() >= b.len(), "expected {} >= {}", a.len(), b.len());
    for i in 0..b.len() {
        a[i] += b[i];
    }
}

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

pub(crate) fn pol_sub_assign<T>(a: &mut [T], b: &[T])
where
    T: SubAssign + Copy,
{
    debug_assert!(a.len() >= b.len(), "expected {} >= {}", a.len(), b.len());
    for i in 0..b.len() {
        a[i] -= b[i];
    }
}

/// Given polynomials a(x) = \sum_{i=0}^{N-1} a[i] x^i and
/// b(x) = \sum_{j=0}^{N-1} b[j] x^j, return their product
/// a(x)b(x) = \sum_{k=0}^{2N-2} c[k] x^k where
/// c[k] = \sum_{i+j=k} a[i]b[j].
///
/// NB: The caller is responsible for ensuring that no undesired
/// overflow occurs during the calculation of the coefficients of the
/// product. In expected applications, N = 16 and the a[i] and b[j] are
/// in [0, 2^16).
pub(crate) fn pol_mul_wide<T>(
    a: [T; N_LIMBS],
    b: [T; N_LIMBS],
) -> [T; 2 * N_LIMBS - 1]
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

pub(crate) fn pol_mul_wide2<T>(
    a: [T; 2 * N_LIMBS],
    b: [T; N_LIMBS],
) -> [T; 3 * N_LIMBS - 1]
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
///
/// N.B. See comment above at pol_mul_wide for discussion of parameter M.
pub(crate) fn pol_extend<T, const N: usize, const M: usize>(a: [T; N]) -> [T; M]
where
    T: Copy + Default,
{
    assert!(M == 2 * N - 1);

    let mut zero_extend = pol_zero();
    zero_extend[..N].copy_from_slice(&a);
    zero_extend
}

/// Given polynomial a(X) = \sum_{i=0}^{M-1} a[i] X^i and an element
/// `root`, return b = (X - root) * a(X)
///
/// NB: Assumes that a[2 * N_LIMBS - 1] = 0.
pub(crate) fn pol_adjoin_root<T, U>(a: [T; 2 * N_LIMBS], root: U) -> [T; 2 * N_LIMBS]
where
    T: Add<Output = T> + Copy + Default + Mul<Output = T> + Neg<Output = T>,
    U: Copy + Mul<T, Output = T> + Neg<Output = U>,
{
    let mut res = [T::default(); 2 * N_LIMBS];
    res[0] = -root * a[0];
    for deg in 1..(2 * N_LIMBS - 1) {
        res[deg] = -(root * a[deg]) + a[deg - 1];
    }
    // NB: We assumes that a[2 * N_LIMBS - 1] = 0, so the last
    // iteration has no "* root" term.
    res[2 * N_LIMBS - 1] = a[2 * N_LIMBS - 2];
    res
}

/// Given polynomial a(X) = \sum_{i=0}^{M-1} a[i] X^i and a root of `a`
/// of the form 2^Exp, return q(X) satisfying a(X) = (X - root) * q(X).
///
/// NB: We do not verify that a(2^Exp) = 0.
///
/// NB: The result could be returned in 2*N-1 elements, but we return
/// 2*N and set the last element to zero since the calling code requires
/// a result zero-extended to 2*N elements anyway.
pub(crate) fn pol_remove_root_2exp<const EXP: usize, T>(a: [T; 2 * N_LIMBS]) -> [T; 2 * N_LIMBS]
where
    T: Copy + Default + Neg<Output = T> + Shr<usize, Output = T> + Sub<Output = T>,
{
    let mut res = [T::default(); 2 * N_LIMBS];
    res[0] = -(a[0] >> EXP);

    // NB: Last element of res is deliberately left equal to zero.
    for deg in 1..2 * N_LIMBS - 1 {
        res[deg] = (res[deg - 1] - a[deg]) >> EXP;
    }

    res
}
