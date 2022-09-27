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
    let reference = vec![
        ("", "0x9c1185a5c5e9fc54612808977ee8f548b2258d31"),
        ("a", "0x0bdc9d2d256b3ee9daae347be6f4dc835a467ffe"),
        ("abc", "0x8eb208f7e05d987a9b044a8e98c6b087f15a0bfc"),
        (
            "message digest",
            "0x5d0689ef49d2fae572b881b123a85ffa21595f36",
        ),
        (
            "abcdefghijklmnopqrstuvwxyz",
            "0xf71c27109c692c1b56bbdceb5b9d2865b3708dbc",
        ),
        (
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "0x12a053384a9c0c88e405a06c27dcf49ada62eb2b",
        ),
        (
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
            "0xb0e20b6e3116640286ed3a87a5713079b21f5189",
        ),
        // (
        //     "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
        //     "0x9b752e45573d4b39f4dbd3323cab82bf63326bfb",
        // )
    ];

    for (x, y) in reference {
        let input: Vec<u8> = make_input(x);
        let expected = U256::from(y);

        let kernel = combined_kernel();
        let initial_offset = kernel.global_labels["ripemd_alt"];
        let initial_stack: Vec<U256> = input.iter().map(|&x| U256::from(x as u8)).rev().collect();
        let final_stack: Vec<U256> = run_with_kernel(&kernel, initial_offset, initial_stack)?
            .stack()
            .to_vec();

        let actual = final_stack[0];
    
        let read_out: Vec<String> = final_stack.iter().map(|x| format!("{:x}", x)).rev().collect();
        println!("{:x?}", read_out);

        assert_eq!(actual, expected);
    }
    Ok(())
}
