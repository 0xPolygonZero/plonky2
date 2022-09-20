use anyhow::Result;

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run;

#[test]
fn test_ripemd() -> Result<()> {
    let kernel = combined_kernel();
    let ripemd = kernel.global_labels["ripemd_alt"];

    let initial_stack = vec![
        0x61, 0x62, 0x63, 0x64,
        0x65, 0x66, 0x67, 0x68,
        0x69, 0x6a, 0x6b, 0x6c,
        0x6d, 0x6e, 0x6f, 0x70,
        0x71, 0x72, 0x73, 0x74,
        0x75, 0x76, 0x77, 0x78,
        0x79, 0x7a
    ];

    let hashed = run(&kernel.code, ripemd, initial_stack, &kernel.prover_inputs)?;
    let result = hashed.stack()[1];
    let actual = format!("{:X}", result);

    let expected = "0xf71c27109c692c1b56bbdceb5b9d2865b3708dbc";
    assert_eq!(expected, actual);

    Ok(())
}
