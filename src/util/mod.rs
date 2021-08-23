use crate::field::field_types::Field;
use crate::polynomial::polynomial::PolynomialValues;

pub(crate) mod context_tree;
pub(crate) mod marking;
pub(crate) mod partial_products;
pub mod reducing;
pub(crate) mod timing;

pub(crate) fn bits_u64(n: u64) -> usize {
    (64 - n.leading_zeros()) as usize
}

pub(crate) const fn ceil_div_usize(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

/// Computes `ceil(log_2(n))`.
pub(crate) fn log2_ceil(n: usize) -> usize {
    n.next_power_of_two().trailing_zeros() as usize
}

/// Computes `log_2(n)`, panicking if `n` is not a power of two.
pub(crate) fn log2_strict(n: usize) -> usize {
    assert!(n.is_power_of_two(), "Not a power of two: {}", n);
    log2_ceil(n)
}

pub(crate) fn transpose_poly_values<F: Field>(polys: Vec<PolynomialValues<F>>) -> Vec<Vec<F>> {
    let poly_values = polys.into_iter().map(|p| p.values).collect::<Vec<_>>();
    transpose(&poly_values)
}

pub fn transpose<F: Field>(matrix: &[Vec<F>]) -> Vec<Vec<F>> {
    let l = matrix.len();
    let w = matrix[0].len();

    let mut transposed = vec![vec![]; w];
    for i in 0..w {
        transposed[i].reserve_exact(l);
        unsafe {
            // After .reserve_exact(l), transposed[i] will have capacity at least l. Hence, set_len
            // will not cause the buffer to overrun.
            transposed[i].set_len(l);
        }
    }

    // Optimization: ensure the larger loop is outside.
    if w >= l {
        for i in 0..w {
            for j in 0..l {
                transposed[i][j] = matrix[j][i];
            }
        }
    } else {
        for j in 0..l {
            for i in 0..w {
                transposed[i][j] = matrix[j][i];
            }
        }
    }

    transposed
}

/// Permutes `arr` such that each index is mapped to its reverse in binary.
pub(crate) fn reverse_index_bits<T: Copy>(arr: &[T]) -> Vec<T> {
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

pub(crate) fn reverse_index_bits_in_place<T>(arr: &mut Vec<T>) {
    let n = arr.len();
    let n_power = log2_strict(n);

    if n_power <= 6 {
        reverse_index_bits_in_place_small(arr, n_power);
    } else {
        reverse_index_bits_in_place_large(arr, n_power);
    }
}

/* Both functions below are semantically equivalent to:
        for src in 0..n {
            let dst = reverse_bits(src, n_power);
            if src < dst {
                arr.swap(src, dst);
            }
        }
   where reverse_bits(src, n_power) computes the n_power-bit reverse.
*/

fn reverse_index_bits_in_place_small<T>(arr: &mut Vec<T>, n_power: usize) {
    let n = arr.len();
    // BIT_REVERSE_6BIT holds 6-bit reverses. This shift makes them n_power-bit reverses.
    let dst_shr_amt = 6 - n_power;
    for src in 0..n {
        let dst = (BIT_REVERSE_6BIT[src] as usize) >> dst_shr_amt;
        if src < dst {
            arr.swap(src, dst);
        }
    }
}

fn reverse_index_bits_in_place_large<T>(arr: &mut Vec<T>, n_power: usize) {
    let n = arr.len();
    // LLVM does not know that it does not need to reverse src at each iteration (which is expensive
    // on x86). We take advantage of the fact that the low bits of dst change rarely and the high
    // bits of dst are dependent only on the low bits of src.
    let dst_lo_shr_amt = 64 - (n_power - 6);
    let dst_hi_shl_amt = n_power - 6;
    for src_chunk in 0..(n >> 6) {
        let src_hi = src_chunk << 6;
        let dst_lo = src_chunk.reverse_bits() >> dst_lo_shr_amt;
        for src_lo in 0..(1 << 6) {
            let dst_hi = (BIT_REVERSE_6BIT[src_lo] as usize) << dst_hi_shl_amt;

            let src = src_hi + src_lo;
            let dst = dst_hi + dst_lo;
            if src < dst {
                arr.swap(src, dst);
            }
        }
    }
}

// Lookup table of 6-bit reverses.
// NB: 2^6=64 bytes is a cacheline. A smaller table wastes cache space.
#[rustfmt::skip]
static BIT_REVERSE_6BIT: &[u8] = &[
    0o00, 0o40, 0o20, 0o60, 0o10, 0o50, 0o30, 0o70,
    0o04, 0o44, 0o24, 0o64, 0o14, 0o54, 0o34, 0o74,
    0o02, 0o42, 0o22, 0o62, 0o12, 0o52, 0o32, 0o72,
    0o06, 0o46, 0o26, 0o66, 0o16, 0o56, 0o36, 0o76,
    0o01, 0o41, 0o21, 0o61, 0o11, 0o51, 0o31, 0o71,
    0o05, 0o45, 0o25, 0o65, 0o15, 0o55, 0o35, 0o75,
    0o03, 0o43, 0o23, 0o63, 0o13, 0o53, 0o33, 0o73,
    0o07, 0o47, 0o27, 0o67, 0o17, 0o57, 0o37, 0o77,
];

pub(crate) fn reverse_bits(n: usize, num_bits: usize) -> usize {
    // NB: The only reason we need overflowing_shr() here as opposed
    // to plain '>>' is to accommodate the case n == num_bits == 0,
    // which would become `0 >> 64`. Rust thinks that any shift of 64
    // bits causes overflow, even when the argument is zero.
    n.reverse_bits()
        .overflowing_shr(usize::BITS - num_bits as u32)
        .0
}

#[cfg(test)]
mod tests {
    use crate::util::{reverse_bits, reverse_index_bits};

    #[test]
    fn test_reverse_bits() {
        assert_eq!(reverse_bits(0b0000000000, 10), 0b0000000000);
        assert_eq!(reverse_bits(0b0000000001, 10), 0b1000000000);
        assert_eq!(reverse_bits(0b1000000000, 10), 0b0000000001);
        assert_eq!(reverse_bits(0b00000, 5), 0b00000);
        assert_eq!(reverse_bits(0b01011, 5), 0b11010);
    }

    #[test]
    fn test_reverse_index_bits() {
        assert_eq!(reverse_index_bits(&[10, 20, 30, 40]), vec![10, 30, 20, 40]);
        assert_eq!(
            reverse_index_bits(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
            vec![0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15]
        );
    }
}
