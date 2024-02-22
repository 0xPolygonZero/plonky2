use anyhow::Result;

use crate::cpu::kernel::aggregator::{combined_kernel, KERNEL};

#[test]
fn test_kernel_code_hash_consistency() -> Result<()> {
    for _ in 0..10 {
        let kernel2 = combined_kernel();
        assert_eq!(kernel2.code_hash, KERNEL.code_hash);
    }

    Ok(())
}
