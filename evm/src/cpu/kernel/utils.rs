use std::fmt::Debug;

use ethereum_types::U256;
use plonky2_util::ceil_div_usize;

/// Enumerate the length `W` windows of `vec`, and run `maybe_replace` on each one.
///
/// Whenever `maybe_replace` returns `Some(replacement)`, the given replacement will be applied.
pub(crate) fn replace_windows<const W: usize, T, F>(vec: &mut Vec<T>, maybe_replace: F)
where
    T: Clone + Debug,
    F: Fn([T; W]) -> Option<Vec<T>>,
{
    let mut start = 0;
    while start + W <= vec.len() {
        let range = start..start + W;
        let window = vec[range.clone()].to_vec().try_into().unwrap();
        if let Some(replacement) = maybe_replace(window) {
            vec.splice(range, replacement);
            // Go back to the earliest window that changed.
            start = start.saturating_sub(W - 1);
        } else {
            start += 1;
        }
    }
}

pub(crate) fn u256_to_trimmed_be_bytes(u256: &U256) -> Vec<u8> {
    let num_bytes = ceil_div_usize(u256.bits(), 8).max(1);
    // `byte` is little-endian, so we manually reverse it.
    (0..num_bytes).rev().map(|i| u256.byte(i)).collect()
}

pub(crate) fn u256_from_bool(b: bool) -> U256 {
    if b {
        U256::one()
    } else {
        U256::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_windows() {
        // This replacement function adds pairs of integers together.
        let mut vec = vec![1, 2, 3, 4, 5];
        replace_windows(&mut vec, |[x, y]| Some(vec![x + y]));
        assert_eq!(vec, vec![15u32]);

        // This replacement function splits each composite integer into two factors.
        let mut vec = vec![9, 1, 6, 8, 15, 7, 9];
        replace_windows(&mut vec, |[n]| {
            (2..n).find(|d| n % d == 0).map(|d| vec![d, n / d])
        });
        assert_eq!(vec, vec![3, 3, 1, 2, 3, 2, 2, 2, 3, 5, 7, 3, 3]);
    }

    #[test]
    fn literal_to_be_bytes() {
        assert_eq!(u256_to_trimmed_be_bytes(&0.into()), vec![0x00]);

        assert_eq!(u256_to_trimmed_be_bytes(&768.into()), vec![0x03, 0x00]);

        assert_eq!(u256_to_trimmed_be_bytes(&0xa1b2.into()), vec![0xa1, 0xb2]);

        assert_eq!(u256_to_trimmed_be_bytes(&0x1b2.into()), vec![0x1, 0xb2]);
    }
}
