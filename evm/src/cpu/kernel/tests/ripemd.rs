use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run;

#[test]
fn test_ripemd() -> Result<()> {
    let kernel = combined_kernel();
    let ripemd = kernel.global_labels["ripemd"];

    let mut initial_stack = vec![U256::from(num_bytes)];
    initial_stack.extend(bytes);

    let after_ripemd = run(&kernel.code, ripemd, initial_stack, &kernel.prover_inputs)?;
    let result = after_ripemd.stack()[1];
    let actual = format!("{:X}", result);

    EXPECTED = "0xf71c27109c692c1b56bbdceb5b9d2865b3708dbc"
    assert_eq!(EXPECTED, actual);

    Ok(())
}
