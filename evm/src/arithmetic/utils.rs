use log::error;

/// Emit an error message regarding unchecked range assumptions.
/// Assumes the values in `cols` are `[cols[0], cols[0] + 1, ...,
/// cols[0] + cols.len() - 1]`.
pub(crate) fn _range_check_error<const RC_BITS: u32>(file: &str, line: u32, cols: &[usize]) {
    error!(
        "{}:{}: arithmetic unit skipped {}-bit range-checks on columns {}--{}: not yet implemented",
        line,
        file,
        RC_BITS,
        cols[0],
        cols[0] + cols.len() - 1
    );
}

#[macro_export]
macro_rules! range_check_error {
    ($cols:ident, $rc_bits:expr) => {
        $crate::arithmetic::utils::_range_check_error::<$rc_bits>(file!(), line!(), &$cols);
    };
    ([$cols:ident], $rc_bits:expr) => {
        $crate::arithmetic::utils::_range_check_error::<$rc_bits>(file!(), line!(), &[$cols]);
    };
}
