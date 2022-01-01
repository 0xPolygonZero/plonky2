#[cfg(all(
    target_feature = "avx2",
    not(all(
        target_feature = "avx512bw",
        target_feature = "avx512cd",
        target_feature = "avx512dq",
        target_feature = "avx512f",
        target_feature = "avx512vl"
    ))
))]
pub mod avx2_goldilocks_field;

#[cfg(all(
    target_feature = "avx512bw",
    target_feature = "avx512cd",
    target_feature = "avx512dq",
    target_feature = "avx512f",
    target_feature = "avx512vl"
))]
pub mod avx512_goldilocks_field;
