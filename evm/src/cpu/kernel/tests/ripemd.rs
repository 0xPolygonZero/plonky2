use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run_with_kernel;

fn make_input(word: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = vec![word.len().try_into().unwrap()];
    bytes.append(&mut word.as_bytes().to_vec());
    bytes
}

#[test]
fn test_ripemd() -> Result<()> {
    let input: Vec<u8> = make_input("a");
    let expected = U256::from("0x0bdc9d2d256b3ee9daae347be6f4dc835a467ffe");

    let kernel = combined_kernel();
    let label = kernel.global_labels["ripemd_alt"];
    let stack_input: Vec<U256> = input.iter().map(|&x| U256::from(x as u8)).rev().collect();
    let output: U256 = run_with_kernel(&kernel, label, stack_input)?.stack()[0];
    assert_eq!(output, expected);

    Ok(())
}
