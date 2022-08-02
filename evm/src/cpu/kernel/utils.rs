use ethereum_types::U256;
use plonky2_util::ceil_div_usize;

pub(crate) fn u256_to_trimmed_be_bytes(u256: &U256) -> Vec<u8> {
    let num_bytes = ceil_div_usize(u256.bits(), 8).max(1);
    // `byte` is little-endian, so we manually reverse it.
    (0..num_bytes).rev().map(|i| u256.byte(i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_to_be_bytes() {
        assert_eq!(u256_to_trimmed_be_bytes(&0.into()), vec![0x00]);

        assert_eq!(u256_to_trimmed_be_bytes(&768.into()), vec![0x03, 0x00]);

        assert_eq!(u256_to_trimmed_be_bytes(&0xa1b2.into()), vec![0xa1, 0xb2]);

        assert_eq!(u256_to_trimmed_be_bytes(&0x1b2.into()), vec![0x1, 0xb2]);
    }
}
