// TODO: Can this impl usize?
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
