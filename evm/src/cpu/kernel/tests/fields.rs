use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::run_with_kernel;


#[test]
fn test_field() -> Result<()> {

    let kernel = combined_kernel();
    let initial_offset = kernel.global_labels["mul_Fp6"];
    let initial_stack: Vec<U256> = vec![0, 0, 3, 1, 0, 1, 0, 1, 0, 1, 0, 0].iter().map(|&x| U256::from(x as u32)).rev().collect();
    let final_stack: Vec<U256> = run_with_kernel(&kernel, initial_offset, initial_stack)?
        .stack()
        .to_vec();

    let expected: Vec<U256> = vec![2, 12, 100, 1, 3, 0].iter().map(|&x| U256::from(x as u32)).rev().collect();
    assert_eq!(final_stack, expected);
    
    Ok(())
}
