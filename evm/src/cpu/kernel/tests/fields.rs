use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run_with_kernel;

fn make_stack(xs: &[u32]) -> Vec<U256> {
    Vec::from(xs)
        .iter()
        .map(|&x| U256::from(x as u32))
        .rev()
        .collect()
}

#[test]
fn test_fp6() -> Result<()> {
    let kernel = combined_kernel();
    let initial_offset = kernel.global_labels["test_mul_Fp6"];
    let initial_stack: Vec<U256> = make_stack(&[1, 1, 0, 0, 1, 0, 3, 0, 0, 1, 0, 0]);
    let final_stack: Vec<U256> = run_with_kernel(&kernel, initial_offset, initial_stack)?
        .stack()
        .to_vec();
    let expected: Vec<U256> = make_stack(&[2, 12, 100, 1, 3, 0]);

    assert_eq!(final_stack, expected);

    Ok(())
}

#[test]
fn test_fp12() -> Result<()> {
    let kernel = combined_kernel();
    let initial_offset = kernel.global_labels["test_mul_Fp12"];
    let initial_stack: Vec<U256> = make_stack(&[
        1, 1, 0, 0, 1, 0, 3, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 3, 0, 0, 1, 0, 0,
    ]);
    let final_stack: Vec<U256> = run_with_kernel(&kernel, initial_offset, initial_stack)?
        .stack()
        .to_vec();
    let expected: Vec<U256> = make_stack(&[5, 5, 9, 0, 5, 3, 17, 12, 100, 1, 3, 0]);

    assert_eq!(final_stack, expected);

    Ok(())
}
