use anyhow::{ensure, Result};
use ethereum_types::U256;
use hex_literal::hex;
use keccak_hash::keccak;

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::assembler::Kernel;
use crate::cpu::kernel::interpreter::run;
use crate::cpu::kernel::tests::u256ify;

fn pubkey_to_addr(x: U256, y: U256) -> Vec<u8> {
    let mut buf = [0; 64];
    x.to_big_endian(&mut buf[0..32]);
    y.to_big_endian(&mut buf[32..64]);
    let hash = keccak(buf);
    hash.0[12..].to_vec()
}

fn test_valid_ecrecover(
    hash: &str,
    v: &str,
    r: &str,
    s: &str,
    expected: &str,
    kernel: &Kernel,
) -> Result<()> {
    let ecrecover = kernel.global_labels["ecrecover"];
    let initial_stack = u256ify([s, r, v, hash])?;
    let stack = run(&kernel.code, ecrecover, initial_stack);
    let got = pubkey_to_addr(stack[1], stack[0]);
    assert_eq!(got, hex::decode(expected).unwrap());

    Ok(())
}

#[test]
fn test_ecrecover() -> Result<()> {
    let kernel = combined_kernel();

    test_valid_ecrecover(
        "0x55f77e8909b1f1c9531c4a309bb2d40388e9ed4b87830c8f90363c6b36255fb9",
        "0x1b",
        "0xd667c5a20fa899b253924099e10ae92998626718585b8171eb98de468bbebc",
        "0x58351f48ce34bf134ee611fb5bf255a5733f0029561d345a7d46bfa344b60ac0",
        "67f3c0Da351384838d7F7641AB0fCAcF853E1844",
        &kernel,
    )?;
    test_valid_ecrecover(
        "0x55f77e8909b1f1c9531c4a309bb2d40388e9ed4b87830c8f90363c6b36255fb9",
        "0x1c",
        "0xd667c5a20fa899b253924099e10ae92998626718585b8171eb98de468bbebc",
        "0x58351f48ce34bf134ee611fb5bf255a5733f0029561d345a7d46bfa344b60ac0",
        "aA58436DeABb64982a386B2De1A8015AA28fCCc0",
        &kernel,
    )?;
    // test_valid_ecrecover(
    //     "0x0",
    //     "0x1c",
    //     "0x3a18b21408d275dde53c0ea86f9c1982eca60193db0ce15008fa408d43024847",
    //     "0x5db9745f44089305b2f2c980276e7025a594828d878e6e36dd2abd34ca6b9e3d",
    //     "aA58436DeABb64982a386B2De1A8015AA28fCCc0",
    //     &kernel,
    // )?;

    Ok(())
}
