use std::ops::{Add, AddAssign, Mul, SubAssign};

use log::error;

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

pub(crate) fn pol_add<T, const M: usize, const N: usize>(a: [T; N], b: [T; N]) -> [T; M]
where
    T: Add<Output = T> + Copy + Default,
{
    // TODO: This should be static_assert-able
    assert!(M >= N);
    let mut sum = pol_zero();
    for i in 0..N {
        sum[i] = a[i] + b[i];
    }
    sum
}

pub(crate) fn pol_sub_assign<T, const M: usize>(a: &mut [T; M], b: &[T])
where
    T: SubAssign + Copy,
{
    assert!(b.len() <= M, "expected {} <= {}", b.len(), M);
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
///
/// NB: The parameter M is inferred at the call site, but it should be
/// *enforced* to be 2*N - 1. Unfortunately Rust's generics won't
/// allow me to just put 2*N-1 in place of M below; worse, the
/// static_assert package can't check that M == 2*N - 1 at compile
/// time either, for reasons the compiler was not able to clearly
/// explain.
pub(crate) fn pol_mul_wide<T, const M: usize, const N: usize, const P: usize>(
    a: [T; M],
    b: [T; N],
) -> [T; P]
where
    T: AddAssign + Copy + Mul<Output = T> + Default,
{
    assert!(P == M + N - 1);
    let mut res = [T::default(); P];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            res[i + j] += ai * bj;
        }
    }
    res
}

pub(crate) fn pol_mul_lo<T, const N: usize>(a: [T; N], b: [T; N]) -> [T; N]
where
    T: AddAssign + Copy + Mul<Output = T> + Default,
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

/// N.B. See comment above at pol_mul_wide for discussion of parameter M.
pub(crate) fn pol_extend<T, const N: usize, const M: usize>(a: [T; N]) -> [T; M]
where
    T: Copy + Default,
{
    assert!(M == 2 * N - 1);

    // Return [a[0], a[1], ..., a[N-1], 0, 0, ..., 0]
    let mut zero_extend = pol_zero();
    zero_extend[..N].copy_from_slice(&a);
    zero_extend
}
