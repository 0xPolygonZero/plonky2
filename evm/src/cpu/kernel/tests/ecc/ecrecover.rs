use std::str::FromStr;

use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::run_interpreter;
use crate::cpu::kernel::tests::u256ify;

fn test_valid_ecrecover(hash: &str, v: &str, r: &str, s: &str, expected: &str) -> Result<()> {
    let ecrecover = KERNEL.global_labels["ecrecover"];
    let initial_stack = u256ify(["0xdeadbeef", s, r, v, hash])?;
    let stack = run_interpreter(ecrecover, initial_stack)?.stack().to_vec();
    assert_eq!(stack[0], U256::from_str(expected).unwrap());

    Ok(())
}

fn test_invalid_ecrecover(hash: &str, v: &str, r: &str, s: &str) -> Result<()> {
    let ecrecover = KERNEL.global_labels["ecrecover"];
    let initial_stack = u256ify(["0xdeadbeef", s, r, v, hash])?;
    let stack = run_interpreter(ecrecover, initial_stack)?.stack().to_vec();
    assert_eq!(stack, vec![U256::MAX]);

    Ok(())
}

#[test]
fn test_ecrecover_real_block() -> Result<()> {
    let f = include_str!("ecrecover_test_data");
    let convert_v = |v| match v {
        // TODO: do this properly.
        "0" => "0x1b",
        "1" => "0x1c",
        "37" => "0x1b",
        "38" => "0x1c",
        _ => panic!("Invalid v."),
    };
    for line in f.lines().filter(|s| !s.starts_with("//")) {
        let line = line.split_whitespace().collect::<Vec<_>>();
        test_valid_ecrecover(line[4], convert_v(line[0]), line[1], line[2], line[3])?;
    }
    Ok(())
}

#[test]
fn test_ecrecover() -> Result<()> {
    test_valid_ecrecover(
        "0x55f77e8909b1f1c9531c4a309bb2d40388e9ed4b87830c8f90363c6b36255fb9",
        "0x1b",
        "0xd667c5a20fa899b253924099e10ae92998626718585b8171eb98de468bbebc",
        "0x58351f48ce34bf134ee611fb5bf255a5733f0029561d345a7d46bfa344b60ac0",
        "0x67f3c0Da351384838d7F7641AB0fCAcF853E1844",
    )?;
    test_valid_ecrecover(
        "0x55f77e8909b1f1c9531c4a309bb2d40388e9ed4b87830c8f90363c6b36255fb9",
        "0x1c",
        "0xd667c5a20fa899b253924099e10ae92998626718585b8171eb98de468bbebc",
        "0x58351f48ce34bf134ee611fb5bf255a5733f0029561d345a7d46bfa344b60ac0",
        "0xaA58436DeABb64982a386B2De1A8015AA28fCCc0",
    )?;
    test_valid_ecrecover(
        "0x0",
        "0x1c",
        "0x1",
        "0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364140",
        "0x3344c6f6eeCA588be132142DB0a32C71ABFAAe7B",
    )?;

    test_invalid_ecrecover(
        "0x0",
        "0x42", // v not in {27,28}
        "0x1",
        "0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364140",
    )?;
    test_invalid_ecrecover(
        "0x0",
        "0x42",
        "0xd667c5a20fa899b253924099e10ae92998626718585b8171eb98de468bbebc",
        "0x0", // s=0
    )?;
    test_invalid_ecrecover(
        "0x0",
        "0x42",
        "0x0", // r=0
        "0xd667c5a20fa899b253924099e10ae92998626718585b8171eb98de468bbebc",
    )?;
    test_invalid_ecrecover(
        "0x0",
        "0x1c",
        "0x3a18b21408d275dde53c0ea86f9c1982eca60193db0ce15008fa408d43024847", // r^3 + 7 isn't a square
        "0x5db9745f44089305b2f2c980276e7025a594828d878e6e36dd2abd34ca6b9e3d",
    )?;

    Ok(())
}
