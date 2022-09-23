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

    let input: Vec<u8> = make_input("abcdefghijklmnopqrstuvwxyz");
    let expected = "f71c27109c692c1b56bbdceb5b9d2865b3708dbc";

    let kernel = combined_kernel();
    let label = kernel.global_labels["ripemd_alt"];
    let stack_input: Vec<U256> = input.iter().map(|&x| U256::from(x as u8)).rev().collect();
    let output: String = run_with_kernel(&kernel, label, stack_input)?
        .stack()
        .iter()
        .map(|&x| format!("{:x}", x))
        .rev()
        .collect();
    assert_eq!(output, expected);

    Ok(())
}
