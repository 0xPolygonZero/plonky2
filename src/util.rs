use crate::field::field::Field;
use crate::polynomial::polynomial::PolynomialValues;

pub(crate) fn bits_u64(n: u64) -> usize {
    (64 - n.leading_zeros()) as usize
}

pub(crate) fn ceil_div_usize(a: usize, b: usize) -> usize {
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
    let poly_values = polys.into_iter()
        .map(|p| p.values)
        .collect::<Vec<_>>();
    transpose(&poly_values)
}

pub(crate) fn transpose<T: Clone>(matrix: &[Vec<T>]) -> Vec<Vec<T>> {
    let old_rows = matrix.len();
    let old_cols = matrix[0].len();
    let mut transposed = vec![Vec::with_capacity(old_rows); old_cols];
    for new_r in 0..old_cols {
        for new_c in 0..old_rows {
            transposed[new_r].push(matrix[new_c][new_r].clone());
        }
    }
    transposed
}

/// Permutes `arr` such that each index is mapped to its reverse in binary.
pub(crate) fn reverse_index_bits<T: Clone>(arr: Vec<T>) -> Vec<T> {
    let n = arr.len();
    let n_power = log2_strict(n);

    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        result.push(arr[reverse_bits(i, n_power)].clone());
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

fn reverse_bits(n: usize, num_bits: usize) -> usize {
    let mut result = 0;
    for i in 0..num_bits {
        let i_rev = num_bits - i - 1;
        result |= (n >> i & 1) << i_rev;
    }
    result
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
            vec![10, 30, 20, 40]);
        assert_eq!(
            reverse_index_bits(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
            vec![0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15]);
    }
}
