use anyhow::Result;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

#[test]
fn hex_prefix_even_nonterminated() -> Result<()> {
    let hex_prefix = KERNEL.global_labels["hex_prefix_rlp"];

    let retdest = 0xDEADBEEFu32.into();
    let terminated = 0.into();
    let packed_nibbles = 0xABCDEF.into();
    let num_nibbles = 6.into();
    let rlp_pos = 0.into();
    let initial_stack = vec![retdest, terminated, packed_nibbles, num_nibbles, rlp_pos];
    let mut interpreter = Interpreter::new_with_kernel(hex_prefix, initial_stack);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![5.into()]);

    assert_eq!(
        interpreter.get_rlp_memory(),
        vec![
            0x80 + 4, // prefix
            0,        // neither flag is set
            0xAB,
            0xCD,
            0xEF
        ]
    );

    Ok(())
}

#[test]
fn hex_prefix_odd_terminated() -> Result<()> {
    let hex_prefix = KERNEL.global_labels["hex_prefix_rlp"];

    let retdest = 0xDEADBEEFu32.into();
    let terminated = 1.into();
    let packed_nibbles = 0xABCDE.into();
    let num_nibbles = 5.into();
    let rlp_pos = 0.into();
    let initial_stack = vec![retdest, terminated, packed_nibbles, num_nibbles, rlp_pos];
    let mut interpreter = Interpreter::new_with_kernel(hex_prefix, initial_stack);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![4.into()]);

    assert_eq!(
        interpreter.get_rlp_memory(),
        vec![
            0x80 + 3, // prefix
            (2 + 1) * 16 + 0xA,
            0xBC,
            0xDE,
        ]
    );

    Ok(())
}

#[test]
fn hex_prefix_odd_terminated_tiny() -> Result<()> {
    let hex_prefix = KERNEL.global_labels["hex_prefix_rlp"];

    let retdest = 0xDEADBEEFu32.into();
    let terminated = 1.into();
    let packed_nibbles = 0xA.into();
    let num_nibbles = 1.into();
    let rlp_pos = 2.into();
    let initial_stack = vec![retdest, terminated, packed_nibbles, num_nibbles, rlp_pos];
    let mut interpreter = Interpreter::new_with_kernel(hex_prefix, initial_stack);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![3.into()]);

    assert_eq!(
        interpreter.get_rlp_memory(),
        vec![
            // Since rlp_pos = 2, we skipped over the first two bytes.
            0,
            0,
            // No length prefix; this tiny string is its own RLP encoding.
            (2 + 1) * 16 + 0xA,
        ]
    );

    Ok(())
}
