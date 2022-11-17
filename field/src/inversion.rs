use crate::types::PrimeField64;

/// This is a 'safe' iteration for the modular inversion algorithm. It
/// is safe in the sense that it will produce the right answer even
/// when f + g >= 2^64.
#[inline(always)]
fn safe_iteration(f: &mut u64, g: &mut u64, c: &mut i128, d: &mut i128, k: &mut u32) {
    if f < g {
        core::mem::swap(f, g);
        core::mem::swap(c, d);
    }
    if *f & 3 == *g & 3 {
        // f - g = 0 (mod 4)
        *f -= *g;
        *c -= *d;

        // kk >= 2 because f is now 0 (mod 4).
        let kk = f.trailing_zeros();
        *f >>= kk;
        *d <<= kk;
        *k += kk;
    } else {
        // f + g = 0 (mod 4)
        *f = (*f >> 2) + (*g >> 2) + 1u64;
        *c += *d;
        let kk = f.trailing_zeros();
        *f >>= kk;
        *d <<= kk + 2;
        *k += kk + 2;
    }
}

/// This is an 'unsafe' iteration for the modular inversion
/// algorithm. It is unsafe in the sense that it might produce the
/// wrong answer if f + g >= 2^64.
#[inline(always)]
unsafe fn unsafe_iteration(f: &mut u64, g: &mut u64, c: &mut i128, d: &mut i128, k: &mut u32) {
    if *f < *g {
        core::mem::swap(f, g);
        core::mem::swap(c, d);
    }
    if *f & 3 == *g & 3 {
        // f - g = 0 (mod 4)
        *f -= *g;
        *c -= *d;
    } else {
        // f + g = 0 (mod 4)
        *f += *g;
        *c += *d;
    }

    // kk >= 2 because f is now 0 (mod 4).
    let kk = f.trailing_zeros();
    *f >>= kk;
    *d <<= kk;
    *k += kk;
}

/// Try to invert an element in a prime field.
///
/// The algorithm below is the "plus-minus-inversion" method
/// with an "almost Montgomery inverse" flair. See Handbook of
/// Elliptic and Hyperelliptic Cryptography, Algorithms 11.6
/// and 11.12.
#[allow(clippy::many_single_char_names)]
pub(crate) fn try_inverse_u64<F: PrimeField64>(x: &F) -> Option<F> {
    let mut f = x.to_noncanonical_u64();
    let mut g = F::ORDER;
    // NB: These two are very rarely such that their absolute
    // value exceeds (p-1)/2; we are paying the price of i128 for
    // the whole calculation, just for the times they do
    // though. Measurements suggest a further 10% time saving if c
    // and d could be replaced with i64's.
    let mut c = 1i128;
    let mut d = 0i128;

    if f == 0 {
        return None;
    }

    // f and g must always be odd.
    let mut k = f.trailing_zeros();
    f >>= k;
    if f == 1 {
        return Some(F::inverse_2exp(k as usize));
    }

    // The first two iterations are unrolled. This is to handle
    // the case where f and g are both large and f+g can
    // overflow. log2(max{f,g}) goes down by at least one each
    // iteration though, so after two iterations we can be sure
    // that f+g won't overflow.

    // Iteration 1:
    safe_iteration(&mut f, &mut g, &mut c, &mut d, &mut k);

    if f == 1 {
        // c must be -1 or 1 here.
        if c == -1 {
            return Some(-F::inverse_2exp(k as usize));
        }
        debug_assert!(c == 1, "bug in try_inverse_u64");
        return Some(F::inverse_2exp(k as usize));
    }

    // Iteration 2:
    safe_iteration(&mut f, &mut g, &mut c, &mut d, &mut k);

    // Remaining iterations:
    while f != 1 {
        unsafe {
            unsafe_iteration(&mut f, &mut g, &mut c, &mut d, &mut k);
        }
    }

    // The following two loops adjust c so it's in the canonical range
    // [0, F::ORDER).

    // The maximum number of iterations observed here is 2; should
    // prove this.
    while c < 0 {
        c += F::ORDER as i128;
    }

    // The maximum number of iterations observed here is 1; should
    // prove this.
    while c >= F::ORDER as i128 {
        c -= F::ORDER as i128;
    }

    // Precomputing the binary inverses rather than using inverse_2exp
    // saves ~5ns on my machine.
    let res = F::from_canonical_u64(c as u64) * F::inverse_2exp(k as usize);
    debug_assert!(*x * res == F::ONE, "bug in try_inverse_u64");
    Some(res)
}
