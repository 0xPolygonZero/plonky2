#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::return_self_not_must_use)]

use std::arch::asm;
use std::hint::unreachable_unchecked;
use std::mem::size_of;
use std::ptr::{swap, swap_nonoverlapping};

mod transpose_util;

use crate::transpose_util::transpose_in_place_square;

pub fn bits_u64(n: u64) -> usize {
    (64 - n.leading_zeros()) as usize
}

pub const fn ceil_div_usize(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

/// Computes `ceil(log_2(n))`.
#[must_use]
pub fn log2_ceil(n: usize) -> usize {
    (usize::BITS - n.saturating_sub(1).leading_zeros()) as usize
}

/// Computes `log_2(n)`, panicking if `n` is not a power of two.
pub fn log2_strict(n: usize) -> usize {
    let res = n.trailing_zeros();
    assert!(n.wrapping_shr(res) == 1, "Not a power of two: {n}");
    // Tell the optimizer about the semantics of `log2_strict`. i.e. it can replace `n` with
    // `1 << res` and vice versa.
    assume(n == 1 << res);
    res as usize
}

/// Returns the largest integer `i` such that `base**i <= n`.
pub const fn log_floor(n: u64, base: u64) -> usize {
    assert!(n > 0);
    assert!(base > 1);
    let mut i = 0;
    let mut cur: u64 = 1;
    loop {
        let (mul, overflow) = cur.overflowing_mul(base);
        if overflow || mul > n {
            return i;
        } else {
            i += 1;
            cur = mul;
        }
    }
}

/// Permutes `arr` such that each index is mapped to its reverse in binary.
pub fn reverse_index_bits<T: Copy>(arr: &[T]) -> Vec<T> {
    let n = arr.len();
    let n_power = log2_strict(n);

    if n_power <= 6 {
        reverse_index_bits_small(arr, n_power)
    } else {
        reverse_index_bits_large(arr, n_power)
    }
}

/* Both functions below are semantically equivalent to:
        for i in 0..n {
            result.push(arr[reverse_bits(i, n_power)]);
        }
   where reverse_bits(i, n_power) computes the n_power-bit reverse. The complications are there
   to guide the compiler to generate optimal assembly.
*/

fn reverse_index_bits_small<T: Copy>(arr: &[T], n_power: usize) -> Vec<T> {
    let n = arr.len();
    let mut result = Vec::with_capacity(n);
    // BIT_REVERSE_6BIT holds 6-bit reverses. This shift makes them n_power-bit reverses.
    let dst_shr_amt = 6 - n_power;
    for i in 0..n {
        let src = (BIT_REVERSE_6BIT[i] as usize) >> dst_shr_amt;
        result.push(arr[src]);
    }
    result
}

fn reverse_index_bits_large<T: Copy>(arr: &[T], n_power: usize) -> Vec<T> {
    let n = arr.len();
    // LLVM does not know that it does not need to reverse src at each iteration (which is expensive
    // on x86). We take advantage of the fact that the low bits of dst change rarely and the high
    // bits of dst are dependent only on the low bits of src.
    let src_lo_shr_amt = 64 - (n_power - 6);
    let src_hi_shl_amt = n_power - 6;
    let mut result = Vec::with_capacity(n);
    for i_chunk in 0..(n >> 6) {
        let src_lo = i_chunk.reverse_bits() >> src_lo_shr_amt;
        for i_lo in 0..(1 << 6) {
            let src_hi = (BIT_REVERSE_6BIT[i_lo] as usize) << src_hi_shl_amt;
            let src = src_hi + src_lo;
            result.push(arr[src]);
        }
    }
    result
}

/// Bit-reverse the order of elements in `arr`.
/// SAFETY: ensure that `arr.len() == 1 << lb_n`.
#[cfg(not(target_arch = "aarch64"))]
unsafe fn reverse_index_bits_in_place_small<T>(arr: &mut [T], lb_n: usize) {
    if lb_n <= 6 {
        // BIT_REVERSE_6BIT holds 6-bit reverses. This shift makes them lb_n-bit reverses.
        let dst_shr_amt = 6 - lb_n;
        for src in 0..arr.len() {
            let dst = (BIT_REVERSE_6BIT[src] as usize) >> dst_shr_amt;
            if src < dst {
                swap(arr.get_unchecked_mut(src), arr.get_unchecked_mut(dst));
            }
        }
    } else {
        // LLVM does not know that it does not need to reverse src at each iteration (which is
        // expensive on x86). We take advantage of the fact that the low bits of dst change rarely and the high
        // bits of dst are dependent only on the low bits of src.
        let dst_lo_shr_amt = 64 - (lb_n - 6);
        let dst_hi_shl_amt = lb_n - 6;
        for src_chunk in 0..(arr.len() >> 6) {
            let src_hi = src_chunk << 6;
            let dst_lo = src_chunk.reverse_bits() >> dst_lo_shr_amt;
            for src_lo in 0..(1 << 6) {
                let dst_hi = (BIT_REVERSE_6BIT[src_lo] as usize) << dst_hi_shl_amt;
                let src = src_hi + src_lo;
                let dst = dst_hi + dst_lo;
                if src < dst {
                    swap(arr.get_unchecked_mut(src), arr.get_unchecked_mut(dst));
                }
            }
        }
    }
}

/// Bit-reverse the order of elements in `arr`.
/// SAFETY: ensure that `arr.len() == 1 << lb_n`.
#[cfg(target_arch = "aarch64")]
unsafe fn reverse_index_bits_in_place_small<T>(arr: &mut [T], lb_n: usize) {
    // Aarch64 can reverse bits in one instruction, so the trivial version works best.
    for src in 0..arr.len() {
        // `wrapping_shr` handles the case when `arr.len() == 1`. In that case `src == 0`, so
        // `src.reverse_bits() == 0`. `usize::wrapping_shr` by 64 is a no-op, but it gives the
        // correct result.
        let dst = src.reverse_bits().wrapping_shr(usize::BITS - lb_n as u32);
        if src < dst {
            swap(arr.get_unchecked_mut(src), arr.get_unchecked_mut(dst));
        }
    }
}

/// Split `arr` chunks and bit-reverse the order of the chunks. There are `1 << lb_num_chunks`
/// chunks, each of length `1 << lb_chunk_size`.
/// SAFETY: ensure that `arr.len() == 1 << lb_num_chunks + lb_chunk_size`.
unsafe fn reverse_index_bits_in_place_chunks<T>(
    arr: &mut [T],
    lb_num_chunks: usize,
    lb_chunk_size: usize,
) {
    for i in 0..1usize << lb_num_chunks {
        // `wrapping_shr` handles the silly case when `lb_num_chunks == 0`.
        let j = i
            .reverse_bits()
            .wrapping_shr(usize::BITS - lb_num_chunks as u32);
        if i < j {
            swap_nonoverlapping(
                arr.get_unchecked_mut(i << lb_chunk_size),
                arr.get_unchecked_mut(j << lb_chunk_size),
                1 << lb_chunk_size,
            );
        }
    }
}

// Ensure that SMALL_ARR_SIZE >= 4 * BIG_T_SIZE.
const BIG_T_SIZE: usize = 1 << 14;
const SMALL_ARR_SIZE: usize = 1 << 16;
pub fn reverse_index_bits_in_place<T>(arr: &mut [T]) {
    let n = arr.len();
    let lb_n = log2_strict(n);
    // If the whole array fits in fast cache, then the trivial algorithm is cache friendly. Also, if
    // `T` is really big, then the trivial algorithm is cache-friendly, no matter the size of the
    // array.
    if size_of::<T>() << lb_n <= SMALL_ARR_SIZE || size_of::<T>() >= BIG_T_SIZE {
        unsafe {
            reverse_index_bits_in_place_small(arr, lb_n);
        }
    } else {
        debug_assert!(n >= 4); // By our choice of `BIG_T_SIZE` and `SMALL_ARR_SIZE`.

        // Algorithm:
        //
        // Treat `arr` as a `sqrt(n)` by `sqrt(n)` row-major matrix. (Assume for now that `lb_n` is
        // even, i.e., `n` is a square number.) To perform bit-order reversal we:
        //  1. Bit-reverse the order of the rows. (They are contiguous in memory, so this is
        //     basically a series of large `memcpy`s.)
        //  2. Transpose the matrix.
        //  3. Bit-reverse the order of the rows.
        // This is equivalent to, for every index `0 <= i < n`:
        //  1. bit-reversing `i[lb_n / 2..lb_n]`,
        //  2. swapping `i[0..lb_n / 2]` and `i[lb_n / 2..lb_n]`,
        //  3. bit-reversing `i[lb_n / 2..lb_n]`.
        //
        // If `lb_n` is odd, i.e., `n` is not a square number, then the above procedure requires
        // slight modification. At steps 1 and 3 we bit-reverse bits `ceil(lb_n / 2)..lb_n`, of the
        // index (shuffling `floor(lb_n / 2)` chunks of length `ceil(lb_n / 2)`). At step 2, we
        // perform _two_ transposes. We treat `arr` as two matrices, one where the middle bit of the
        // index is `0` and another, where the middle bit is `1`; we transpose each individually.

        let lb_num_chunks = lb_n >> 1;
        let lb_chunk_size = lb_n - lb_num_chunks;
        unsafe {
            reverse_index_bits_in_place_chunks(arr, lb_num_chunks, lb_chunk_size);
            transpose_in_place_square(arr, lb_chunk_size, lb_num_chunks, 0);
            if lb_num_chunks != lb_chunk_size {
                // `arr` cannot be interpreted as a square matrix. We instead interpret it as a
                // `1 << lb_num_chunks` by `2` by `1 << lb_num_chunks` tensor, in row-major order.
                // The above transpose acted on `tensor[..., 0, ...]` (all indices with middle bit
                // `0`). We still need to transpose `tensor[..., 1, ...]`. To do so, we advance
                // arr by `1 << lb_num_chunks` effectively, adding that to every index.
                let arr_with_offset = &mut arr[1 << lb_num_chunks..];
                transpose_in_place_square(arr_with_offset, lb_chunk_size, lb_num_chunks, 0);
            }
            reverse_index_bits_in_place_chunks(arr, lb_num_chunks, lb_chunk_size);
        }
    }
}

// Lookup table of 6-bit reverses.
// NB: 2^6=64 bytes is a cacheline. A smaller table wastes cache space.
#[rustfmt::skip]
const BIT_REVERSE_6BIT: &[u8] = &[
    0o00, 0o40, 0o20, 0o60, 0o10, 0o50, 0o30, 0o70,
    0o04, 0o44, 0o24, 0o64, 0o14, 0o54, 0o34, 0o74,
    0o02, 0o42, 0o22, 0o62, 0o12, 0o52, 0o32, 0o72,
    0o06, 0o46, 0o26, 0o66, 0o16, 0o56, 0o36, 0o76,
    0o01, 0o41, 0o21, 0o61, 0o11, 0o51, 0o31, 0o71,
    0o05, 0o45, 0o25, 0o65, 0o15, 0o55, 0o35, 0o75,
    0o03, 0o43, 0o23, 0o63, 0o13, 0o53, 0o33, 0o73,
    0o07, 0o47, 0o27, 0o67, 0o17, 0o57, 0o37, 0o77,
];

#[inline(always)]
pub fn assume(p: bool) {
    debug_assert!(p);
    if !p {
        unsafe {
            unreachable_unchecked();
        }
    }
}

/// Try to force Rust to emit a branch. Example:
///     if x > 2 {
///         y = foo();
///         branch_hint();
///     } else {
///         y = bar();
///     }
/// This function has no semantics. It is a hint only.
#[inline(always)]
pub fn branch_hint() {
    unsafe {
        asm!("", options(nomem, nostack, preserves_flags));
    }
}

#[cfg(test)]
mod tests {
    use crate::{log2_ceil, log2_strict};

    #[test]
    fn test_log2_strict() {
        assert_eq!(log2_strict(1), 0);
        assert_eq!(log2_strict(2), 1);
        assert_eq!(log2_strict(1 << 18), 18);
        assert_eq!(log2_strict(1 << 31), 31);
        assert_eq!(
            log2_strict(1 << (usize::BITS - 1)),
            usize::BITS as usize - 1
        );
    }

    #[test]
    #[should_panic]
    fn test_log2_strict_zero() {
        log2_strict(0);
    }

    #[test]
    #[should_panic]
    fn test_log2_strict_nonpower_2() {
        log2_strict(0x78c341c65ae6d262);
    }

    #[test]
    #[should_panic]
    fn test_log2_strict_usize_max() {
        log2_strict(usize::MAX);
    }

    #[test]
    fn test_log2_ceil() {
        // Powers of 2
        assert_eq!(log2_ceil(0), 0);
        assert_eq!(log2_ceil(1), 0);
        assert_eq!(log2_ceil(2), 1);
        assert_eq!(log2_ceil(1 << 18), 18);
        assert_eq!(log2_ceil(1 << 31), 31);
        assert_eq!(log2_ceil(1 << (usize::BITS - 1)), usize::BITS as usize - 1);

        // Nonpowers; want to round up
        assert_eq!(log2_ceil(3), 2);
        assert_eq!(log2_ceil(0x14fe901b), 29);
        assert_eq!(
            log2_ceil((1 << (usize::BITS - 1)) + 1),
            usize::BITS as usize
        );
        assert_eq!(log2_ceil(usize::MAX - 1), usize::BITS as usize);
        assert_eq!(log2_ceil(usize::MAX), usize::BITS as usize);
    }
}
