#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::return_self_not_must_use)]

use std::arch::asm;
use std::hint::unreachable_unchecked;
use std::mem::{size_of, swap};

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
    assert!(n.wrapping_shr(res) == 1, "Not a power of two: {}", n);
    res as usize
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

const LB_TLB_SIZE: usize = 3;
const LB_CACHE_SIZE: usize = 11;

#[inline(always)]
unsafe fn swap_unchecked<T>(arr: &mut [T], i: usize, j: usize) {
    // Cast to pointers to remove lifetime information.
    let i_ptr: *mut T = arr.get_unchecked_mut(i);
    let j_ptr: *mut T = arr.get_unchecked_mut(j);
    swap(&mut *i_ptr, &mut *j_ptr);
}

unsafe fn transpose_in_place_square_small<T>(
    arr: &mut [T],
    lb_stride: usize,
    lb_size: usize,
    x: usize,
) {
    for i in x..x + (1 << lb_size) {
        for offset in 0..1 << lb_size {
            let j = x + ((i + offset) & ((1 << lb_size) - 1));
            swap_unchecked(arr, i + (j << lb_stride), (i << lb_stride) + j);
        }
    }
}

unsafe fn transpose_swap_square_small<T>(
    arr: &mut [T],
    lb_stride: usize,
    lb_size: usize,
    x: usize,
    y: usize,
) {
    for i in x..x + (1 << lb_size) {
        for offset in 0..1 << lb_size {
            let j = y + ((i + offset) & ((1 << lb_size) - 1));
            swap_unchecked(arr, i + (j << lb_stride), (i << lb_stride) + j);
        }
    }
}

unsafe fn transpose_in_place_square<T>(arr: &mut [T], lb_stride: usize, lb_size: usize, x: usize) {
    if lb_size <= LB_TLB_SIZE {
        transpose_in_place_square_small(arr, lb_stride, lb_size, x);
    } else {
        transpose_in_place_square(arr, lb_stride, lb_size - 1, x);
        transpose_swap_square(arr, lb_stride, lb_size - 1, x, x + (1 << (lb_size >> 1)));
        transpose_in_place_square(arr, lb_stride, lb_size - 1, x + (1 << (lb_size >> 1)));
    }
}

unsafe fn transpose_swap_square<T>(
    arr: &mut [T],
    lb_stride: usize,
    lb_size: usize,
    x: usize,
    y: usize,
) {
    if lb_size <= LB_TLB_SIZE {
        transpose_swap_square_small(arr, lb_stride, lb_size, x, y);
    } else {
        transpose_swap_square(arr, lb_stride, lb_size - 1, x, y);
        transpose_swap_square(arr, lb_stride, lb_size - 1, x + (1 << (lb_size >> 1)), y);
        transpose_swap_square(
            arr,
            lb_stride,
            lb_size - 1,
            x + (1 << (lb_size >> 1)),
            y + (1 << (lb_size >> 1)),
        );
        transpose_swap_square(arr, lb_stride, lb_size - 1, x, y + (1 << (lb_size >> 1)));
    }
}

fn reverse_index_bits_in_place_small<T>(arr: &mut [T], n_power: usize) {
    for src in 0..arr.len() {
        let dst = src.reverse_bits() >> (64 - n_power);
        if src < dst {
            unsafe {
                swap_unchecked(arr, src, dst);
            }
        }
    }
}

fn reverse_index_bits_swap_small<T>(arr0: &mut [T], arr1: &mut [T], n_power: usize) {
    let n = arr0.len();
    debug_assert_eq!(n, arr1.len());
    for src in 0..n {
        let dst = src.reverse_bits() >> (64 - n_power);
        swap(unsafe { arr0.get_unchecked_mut(src) }, unsafe {
            arr1.get_unchecked_mut(dst)
        });
    }
}

fn reverse_index_bits_swap<T>(arr0: &mut [T], arr1: &mut [T], n_power: usize) {
    let n = arr0.len();
    debug_assert_eq!(n, arr1.len());
    if n * size_of::<T>() <= 1 << LB_CACHE_SIZE {
        reverse_index_bits_swap_small(arr0, arr1, n_power);
    } else {
        assert_eq!(n_power & 1, 0);
        let half_n_power = n_power >> 1;

        for i in 0..1usize << half_n_power {
            let j = i.reverse_bits() >> (64 - half_n_power);
            reverse_index_bits_swap(
                unsafe { arr0.get_unchecked_mut(i << half_n_power..(i + 1) << half_n_power) },
                unsafe { arr1.get_unchecked_mut(j << half_n_power..(j + 1) << half_n_power) },
                half_n_power,
            );
        }

        unsafe {
            transpose_in_place_square(arr0, half_n_power, half_n_power, 0);
            transpose_in_place_square(arr1, half_n_power, half_n_power, 0);
        }
    }
}

fn reverse_index_bits_in_place_inner<T>(arr: &mut [T], n_power: usize) {
    let n = arr.len();
    if n * size_of::<T>() <= 1 << LB_CACHE_SIZE {
        reverse_index_bits_in_place_small(arr, n_power);
    } else {
        assert_eq!(n_power & 1, 0);
        let half_n_power = n_power >> 1;

        for i in 0..1usize << half_n_power {
            let j = i.reverse_bits() >> (64 - half_n_power);
            if i < j {
                let arr0_ptr: *mut [T] =
                    unsafe { arr.get_unchecked_mut(i << half_n_power..(j + 1) << half_n_power) };
                let arr1_ptr: *mut [T] =
                    unsafe { arr.get_unchecked_mut(j << half_n_power..(i + 1) << half_n_power) };
                reverse_index_bits_swap(
                    unsafe { &mut *arr0_ptr },
                    unsafe { &mut *arr1_ptr },
                    half_n_power,
                );
            } else if i == j {
                reverse_index_bits_in_place_inner(
                    unsafe { arr.get_unchecked_mut(i << half_n_power..(i + 1) << half_n_power) },
                    half_n_power,
                );
            }
        }

        unsafe {
            transpose_in_place_square(arr, half_n_power, half_n_power, 0);
        }
    }
}

pub fn reverse_index_bits_in_place<T>(arr: &mut [T]) {
    let n = arr.len();
    let n_power = log2_strict(n);
    reverse_index_bits_in_place_inner(arr, n_power);
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
