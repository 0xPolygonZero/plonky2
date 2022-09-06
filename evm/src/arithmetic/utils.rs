use log::error;
use num::Zero;
use std::ops::{Add, AddAssign, Mul, Sub};

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

pub(crate) fn polzero<T, const N: usize>() -> [T; N]
where
    T: Copy + Zero,
{
    [T::zero(); N]
}

pub(crate) fn poladd<T, const N: usize>(
    a: [T; N],
    b: [T; N]
) -> [T; N]
where
    T: Add<Output = T> + Copy + Zero,
{
    let mut sum = polzero();
    for i in 0..N {
        sum[i] = a[i] + b[i];
    }
    sum
}

pub(crate) fn polsub<T, const N: usize>(
    a: [T; N],
    b: [T; N]
) -> [T; N]
where
    T: Sub<Output = T> + Copy + Zero,
{
    let mut diff = polzero();
    for i in 0..N {
        diff[i] = a[i] - b[i];
    }
    diff
}

/// Given polynomials a(x) = \sum_{i=0}^{N-1} a[i] x^i and
/// b(x) = \sum_{j=0}^{N-1} b[j] x^j, return their product
/// a(x)b(x) = \sum_{k=0}^{2N-2} c[k] x^k where
/// c[k] = \sum_{i+j=k} a[i]b[j].
///
/// FIXME: finish this comment
/// To avoid overflow, the coefficients must be less than...
pub(crate) fn polmul_wide<T, const N: usize>(
    a: [T; N],
    b: [T; N]
) -> [T; 2 * N - 1]
where
    T: AddAssign + Copy + Mul<Output = T> + Zero,
{
    let mut res = polzero();
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            res[i + j] += ai * bj;
        }
    }
    res
}

pub(crate) fn polmul_lo<T, const N: usize>(
    a: [T; N],
    b: [T; N]
) -> [T; N]
where
    T: AddAssign + Copy + Mul<Output = T> + Zero,
{
    let mut res = polzero();
    for deg in 0..N {
        for i in 0..=deg {
            // Invariant: i + j = deg
            let j = deg - i;
            res[deg] += a[i] * b[j];
        }
    }
    res
}

/*
/// Given two 16N-bit unsigned integers `a` and `b`, return their
/// product modulo 2^(16N).
pub(crate) fn umul_cc<T, const N: usize>(
    a: [T; N],
    b: [T; N],
    mask: T,
    limb_bits: usize
) -> [T; N]
where
    T: AddAssign + Copy + Mul<Output = T> + Zero,
{
    let mut cy = T::zero();
    let mut res = polmul_lo(a, b);

    for i in 0..N {
        let t = res[i] + cy;
        cy = t >> limb_bits;
        res[i] = t & mask;
    }
    res
}
*/
