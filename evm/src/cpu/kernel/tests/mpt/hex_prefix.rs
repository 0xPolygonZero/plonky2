use anyhow::Result;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

#[test]
fn hex_prefix_even_nonterminated() -> Result<()> {
    let hex_prefix = KERNEL.global_labels["hex_prefix"];

    let retdest = 0xDEADBEEFu32.into();
    let terminated = 0.into();
    let packed_nibbles = 0xABCDEF.into();
    let num_nibbles = 6.into();
    let initial_stack = vec![retdest, terminated, packed_nibbles, num_nibbles];
    let mut interpreter = Interpreter::new_with_kernel(hex_prefix, initial_stack);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![4.into()]);

    assert_eq!(
        interpreter.get_kernel_general_data(),
        vec![0.into(), 0xAB.into(), 0xCD.into(), 0xEF.into(),]
    );

    Ok(())
}

#[test]
fn hex_prefix_odd_terminated() -> Result<()> {
    let hex_prefix = KERNEL.global_labels["hex_prefix"];

    let retdest = 0xDEADBEEFu32.into();
    let terminated = 1.into();
    let packed_nibbles = 0xABCDE.into();
    let num_nibbles = 5.into();
    let initial_stack = vec![retdest, terminated, packed_nibbles, num_nibbles];
    let mut interpreter = Interpreter::new_with_kernel(hex_prefix, initial_stack);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![3.into()]);

    assert_eq!(
        interpreter.get_kernel_general_data(),
        vec![
            (terminated * 2 + 1u32) * 16 + 0xAu32,
            0xBC.into(),
            0xDE.into(),
        ]
    );

    Ok(())
}
