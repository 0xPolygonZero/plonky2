use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::{combined_kernel, KERNEL};
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment;

#[test]
fn test_kernel_code_hash_consistency() -> Result<()> {
    for _ in 0..10 {
        let kernel2 = combined_kernel();
        assert_eq!(kernel2.code_hash, KERNEL.code_hash);
    }

    Ok(())
}
