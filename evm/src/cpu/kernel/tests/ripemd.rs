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
    // let input: Vec<u8> = make_input("12345678901234567890123456789012345678901234567890123456789012345678901234567890");
    // let expected = U256::from("0x9b752e45573d4b39f4dbd3323cab82bf63326bfb");

    let input: Vec<u8> = make_input("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789");
    let expected = U256::from("0xb0e20b6e3116640286ed3a87a5713079b21f5189");

    // let input: Vec<u8> = make_input("");
    // let expected = U256::from("0x9c1185a5c5e9fc54612808977ee8f548b2258d31");


    let kernel = combined_kernel();
    let label = kernel.global_labels["ripemd_alt"];
    let stack_input: Vec<U256> = input.iter().map(|&x| U256::from(x as u8)).rev().collect();
    let stack_output: Vec<U256> = run_with_kernel(&kernel, label, stack_input)?.stack().to_vec();

    let read_out: Vec<String> = stack_output.iter().map(|x| format!("{:x}", x)).rev().collect();
    println!("{:x?}", read_out);

    let actual = stack_output[0];
    assert_eq!(actual, expected);

    Ok(())
}
