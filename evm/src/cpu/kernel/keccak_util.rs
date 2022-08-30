use tiny_keccak::keccakf;

/// A Keccak-f based hash.
///
/// This hash does not use standard Keccak padding, since we don't care about extra zeros at the
/// end of the code.
pub(crate) fn hash_kernel(_code: &[u8]) -> [u32; 8] {
    let state = [0u32; 50];
    // TODO: absorb code
    state[0..8].try_into().unwrap()
}

/// Like tiny-keccak's `keccakf`, but deals with `u32` limbs instead of `u64` limbs.
pub(crate) fn keccakf_u32s(state_u32s: &mut [u32; 50]) {
    let mut state_u64s: [u64; 25] = std::array::from_fn(|i| {
        let lo = state_u32s[i * 2] as u64;
        let hi = state_u32s[i * 2 + 1] as u64;
        lo | (hi << 32)
    });
    keccakf(&mut state_u64s);
    *state_u32s = std::array::from_fn(|i| {
        let u64_limb = state_u64s[i / 2];
        let is_hi = i % 2;
        (u64_limb >> (is_hi * 32)) as u32
    });
}

#[cfg(test)]
mod tests {
    use tiny_keccak::keccakf;

    use crate::cpu::kernel::keccak_util::keccakf_u32s;

    #[test]
    #[rustfmt::skip]
    fn test_consistency() {
        // We will hash the same data using keccakf and keccakf_u32s.
        // The inputs were randomly generated in Python.
        let mut state_u64s: [u64; 25] = [0x5dc43ed05dc64048, 0x7bb9e18cdc853880, 0xc1fde300665b008f, 0xeeab85e089d5e431, 0xf7d61298e9ef27ea, 0xc2c5149d1a492455, 0x37a2f4eca0c2d2f2, 0xa35e50c015b3e85c, 0xd2daeced29446ebe, 0x245845f1bac1b98e, 0x3b3aa8783f30a9bf, 0x209ca9a81956d241, 0x8b8ea714da382165, 0x6063e67e202c6d29, 0xf4bac2ded136b907, 0xb17301b461eae65, 0xa91ff0e134ed747c, 0xcc080b28d0c20f1d, 0xf0f79cbec4fb551c, 0x25e04cb0aa930cad, 0x803113d1b541a202, 0xfaf1e4e7cd23b7ec, 0x36a03bbf2469d3b0, 0x25217341908cdfc0, 0xe9cd83f88fdcd500];
        let mut state_u32s: [u32; 50] = [0x5dc64048, 0x5dc43ed0, 0xdc853880, 0x7bb9e18c, 0x665b008f, 0xc1fde300, 0x89d5e431, 0xeeab85e0, 0xe9ef27ea, 0xf7d61298, 0x1a492455, 0xc2c5149d, 0xa0c2d2f2, 0x37a2f4ec, 0x15b3e85c, 0xa35e50c0, 0x29446ebe, 0xd2daeced, 0xbac1b98e, 0x245845f1, 0x3f30a9bf, 0x3b3aa878, 0x1956d241, 0x209ca9a8, 0xda382165, 0x8b8ea714, 0x202c6d29, 0x6063e67e, 0xd136b907, 0xf4bac2de, 0x461eae65, 0xb17301b, 0x34ed747c, 0xa91ff0e1, 0xd0c20f1d, 0xcc080b28, 0xc4fb551c, 0xf0f79cbe, 0xaa930cad, 0x25e04cb0, 0xb541a202, 0x803113d1, 0xcd23b7ec, 0xfaf1e4e7, 0x2469d3b0, 0x36a03bbf, 0x908cdfc0, 0x25217341, 0x8fdcd500, 0xe9cd83f8];

        // The first output was generated using tiny-keccak; the second was derived from it.
        let out_u64s: [u64; 25] = [0x8a541df597e79a72, 0x5c26b8c84faaebb3, 0xc0e8f4e67ca50497, 0x95d98a688de12dec, 0x1c837163975ffaed, 0x9481ec7ef948900e, 0x6a072c65d050a9a1, 0x3b2817da6d615bee, 0x7ffb3c4f8b94bf21, 0x85d6c418cced4a11, 0x18edbe0442884135, 0x2bf265ef3204b7fd, 0xc1e12ce30630d105, 0x8c554dbc61844574, 0x5504db652ce9e42c, 0x2217f3294d0dabe5, 0x7df8eebbcf5b74df, 0x3a56ebb61956f501, 0x7840219dc6f37cc, 0x23194159c967947, 0x9da289bf616ba14d, 0x5a90aaeeca9e9e5b, 0x885dcdc4a549b4e3, 0x46cb188c20947df7, 0x1ef285948ee3d8ab];
        let out_u32s: [u32; 50] = [0x97e79a72, 0x8a541df5, 0x4faaebb3, 0x5c26b8c8, 0x7ca50497, 0xc0e8f4e6, 0x8de12dec, 0x95d98a68, 0x975ffaed, 0x1c837163, 0xf948900e, 0x9481ec7e, 0xd050a9a1, 0x6a072c65, 0x6d615bee, 0x3b2817da, 0x8b94bf21, 0x7ffb3c4f, 0xcced4a11, 0x85d6c418, 0x42884135, 0x18edbe04, 0x3204b7fd, 0x2bf265ef, 0x630d105, 0xc1e12ce3, 0x61844574, 0x8c554dbc, 0x2ce9e42c, 0x5504db65, 0x4d0dabe5, 0x2217f329, 0xcf5b74df, 0x7df8eebb, 0x1956f501, 0x3a56ebb6, 0xdc6f37cc, 0x7840219, 0x9c967947, 0x2319415, 0x616ba14d, 0x9da289bf, 0xca9e9e5b, 0x5a90aaee, 0xa549b4e3, 0x885dcdc4, 0x20947df7, 0x46cb188c, 0x8ee3d8ab, 0x1ef28594];

        keccakf(&mut state_u64s);
        keccakf_u32s(&mut state_u32s);

        assert_eq!(state_u64s, out_u64s);
        assert_eq!(state_u32s, out_u32s);
    }
}
