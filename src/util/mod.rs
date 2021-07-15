pub mod partial_products;
pub mod scaling;
pub(crate) mod timing;

use crate::field::field::Field;
use crate::polynomial::polynomial::PolynomialValues;

pub(crate) fn bits_u64(n: u64) -> usize {
    (64 - n.leading_zeros()) as usize
}

pub(crate) const fn ceil_div_usize(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

pub(crate) fn pad_to_multiple_usize(a: usize, b: usize) -> usize {
    ceil_div_usize(a, b) * b
}

/// Computes `ceil(log_2(n))`.
pub(crate) fn log2_ceil(n: usize) -> usize {
    n.next_power_of_two().trailing_zeros() as usize
}

/// Computes `log_2(n)`, panicking if `n` is not a power of two.
pub(crate) fn log2_strict(n: usize) -> usize {
    assert!(n.is_power_of_two(), "Not a power of two");
    log2_ceil(n)
}

pub(crate) fn transpose_poly_values<F: Field>(polys: Vec<PolynomialValues<F>>) -> Vec<Vec<F>> {
    let poly_values = polys.into_iter().map(|p| p.values).collect::<Vec<_>>();
    transpose(&poly_values)
}

pub(crate) fn transpose<T: Clone>(matrix: &[Vec<T>]) -> Vec<Vec<T>> {
    if matrix.is_empty() {
        return Vec::new();
    }

    let old_rows = matrix.len();
    let old_cols = matrix[0].len();
    let mut transposed = vec![Vec::with_capacity(old_rows); old_cols];
    for new_r in 0..old_cols {
        for old_row in matrix.iter() {
            transposed[new_r].push(old_row[new_r].clone());
        }
    }
    transposed
}

/// Permutes `arr` such that each index is mapped to its reverse in binary.
pub(crate) fn reverse_index_bits<T: Copy>(arr: Vec<T>) -> Vec<T> {
    let n = arr.len();
    let n_power = log2_strict(n);

    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        result.push(arr[reverse_bits(i, n_power)]);
    }
    result
}

pub(crate) fn reverse_index_bits_in_place<T>(arr: &mut Vec<T>) {
    let n = arr.len();
    let n_power = log2_strict(n);

    for src in 0..n {
        let dst = reverse_bits(src, n_power);
        if src < dst {
            arr.swap(src, dst);
        }
    }
}

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
        assert_eq!(
            reverse_index_bits(vec![10, 20, 30, 40]),
            vec![10, 30, 20, 40]
        );
        assert_eq!(
            reverse_index_bits(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
            vec![0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15]
        );
    }
}
