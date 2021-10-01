// Requires:
// - 64-bit integer registers
// - AVX2
// - BMI2 (for MULX and SHRX)
#[cfg(all(
    target_arch = "x86_64",
    target_feature = "avx2",
    target_feature = "bmi2"
))]
pub(crate) mod poseidon_avx2;
