// Requires:
// - AVX2
// - BMI2 (for MULX and SHRX)
#[cfg(all(target_feature = "avx2", target_feature = "bmi2"))]
pub(crate) mod poseidon_goldilocks_avx2_bmi2;

// Requires AVX2
#[cfg(target_feature = "avx2")]
pub(crate) mod poseidon_crandall_avx2;
