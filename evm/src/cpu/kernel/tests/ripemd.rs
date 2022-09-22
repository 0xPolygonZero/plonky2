use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run_with_kernel;

#[test]
fn test_ripemd() -> Result<()> {
    let expected = "0xf71c27109c692c1b56bbdceb5b9d2865b3708dbc";
    println!("{}", expected);

    let kernel = combined_kernel();

    let input: Vec<u32> = vec![
        0x1a, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e,
        0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a,
    ];
    let stack_init: Vec<U256> = input.iter().map(|&x| U256::from(x as u32)).rev().collect();

    let stack_result = run_with_kernel(&kernel, kernel.global_labels["ripemd_alt"], stack_init)?;
    let result = stack_result.stack()[0];
    let actual = format!("{:X}", result);
    println!("{}", actual);
    assert_eq!(expected, actual);

    Ok(())
}
