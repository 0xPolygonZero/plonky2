use num::{Integer, Zero};

/// Try to invert an element in a prime field with the given modulus.
#[allow(clippy::many_single_char_names)] // The names are from the paper.
pub(crate) fn try_inverse_u64(x: u64, p: u64) -> Option<u64> {
    if x.is_zero() {
        return None;
    }

    // Based on Algorithm 16 of "Efficient Software-Implementation of Finite Fields with
    // Applications to Cryptography".

    let mut u = x;
    let mut v = p;
    let mut b = 1u64;
    let mut c = 0u64;

    while u != 1 && v != 1 {
        let u_tz = u.trailing_zeros();
        u >>= u_tz;
        for _ in 0..u_tz {
            if b.is_even() {
                b /= 2;
            } else {
                // b = (b + p)/2, avoiding overflow
                b = (b / 2) + (p / 2) + 1;
            }
        }

        let v_tz = v.trailing_zeros();
        v >>= v_tz;
        for _ in 0..v_tz {
            if c.is_even() {
                c /= 2;
            } else {
                // c = (c + p)/2, avoiding overflow
                c = (c / 2) + (p / 2) + 1;
            }
        }

        if u >= v {
            u -= v;
            // b -= c
            let (mut diff, under) = b.overflowing_sub(c);
            if under {
                diff = diff.wrapping_add(p);
            }
            b = diff;
        } else {
            v -= u;
            // c -= b
            let (mut diff, under) = c.overflowing_sub(b);
            if under {
                diff = diff.wrapping_add(p);
            }
            c = diff;
        }
    }

    Some(if u == 1 { b } else { c })
}
