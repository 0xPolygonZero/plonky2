use anyhow::Result;

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run;
use crate::cpu::kernel::tests::u256ify;

#[test]
fn test_ec_ops() -> Result<()> {
    // Make sure we can parse and assemble the entire kernel.
    let kernel = combined_kernel();
    let ecrecover = kernel.global_labels["ecrecover"];
    let hash = "0x0";
    let v = "0x27";
    let r = "0x1";
    let s = "0x1";

    let initial_stack = u256ify([s, r, v, hash])?;
    let stack = run(&kernel.code, ecrecover, initial_stack);
    dbg!(stack);

    Ok(())
}
