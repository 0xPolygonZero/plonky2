use anyhow::Result;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

#[test]
fn test_num_bytes_0() -> Result<()> {
    let num_bytes = KERNEL.global_labels["num_bytes"];

    let retdest = 0xDEADBEEFu32.into();
    let x = 0.into();
    let initial_stack = vec![retdest, x];
    let mut interpreter = Interpreter::new_with_kernel(num_bytes, initial_stack);

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![1.into()]);
    Ok(())
}

#[test]
fn test_num_bytes_small() -> Result<()> {
    let num_bytes = KERNEL.global_labels["num_bytes"];

    let retdest = 0xDEADBEEFu32.into();
    let x = 42.into();
    let initial_stack = vec![retdest, x];
    let mut interpreter = Interpreter::new_with_kernel(num_bytes, initial_stack);

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![1.into()]);
    Ok(())
}

#[test]
fn test_num_bytes_medium() -> Result<()> {
    let num_bytes = KERNEL.global_labels["num_bytes"];

    let retdest = 0xDEADBEEFu32.into();
    let x = 0xAABBCCDDu32.into();
    let initial_stack = vec![retdest, x];
    let mut interpreter = Interpreter::new_with_kernel(num_bytes, initial_stack);

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![4.into()]);
    Ok(())
}
