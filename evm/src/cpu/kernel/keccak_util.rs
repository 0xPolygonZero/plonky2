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
pub(crate) fn keccakf_u32s(_state: &mut [u32; 50]) {
    // TODO: Implement
}
