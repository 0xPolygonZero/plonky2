use anyhow::Result;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

#[test]
fn test_decode_rlp_string_len_short() -> Result<()> {
    let decode_rlp_string_len = KERNEL.global_labels["decode_rlp_string_len"];

    let initial_stack = vec![0xDEADBEEFu32.into(), 2.into()];
    let mut interpreter = Interpreter::new_with_kernel(decode_rlp_string_len, initial_stack);

    // A couple dummy bytes, followed by "0x70" which is its own encoding.
    interpreter.set_rlp_memory(vec![123, 234, 0x70]);

    interpreter.run()?;
    let expected_stack = vec![1.into(), 2.into()]; // len, pos
    assert_eq!(interpreter.stack(), expected_stack);

    Ok(())
}

#[test]
fn test_decode_rlp_string_len_medium() -> Result<()> {
    let decode_rlp_string_len = KERNEL.global_labels["decode_rlp_string_len"];

    let initial_stack = vec![0xDEADBEEFu32.into(), 2.into()];
    let mut interpreter = Interpreter::new_with_kernel(decode_rlp_string_len, initial_stack);

    // A couple dummy bytes, followed by the RLP encoding of "1 2 3 4 5".
    interpreter.set_rlp_memory(vec![123, 234, 0x85, 1, 2, 3, 4, 5]);

    interpreter.run()?;
    let expected_stack = vec![5.into(), 3.into()]; // len, pos
    assert_eq!(interpreter.stack(), expected_stack);

    Ok(())
}

#[test]
fn test_decode_rlp_string_len_long() -> Result<()> {
    let decode_rlp_string_len = KERNEL.global_labels["decode_rlp_string_len"];

    let initial_stack = vec![0xDEADBEEFu32.into(), 2.into()];
    let mut interpreter = Interpreter::new_with_kernel(decode_rlp_string_len, initial_stack);

    // The RLP encoding of the string "1 2 3 ... 56".
    interpreter.set_rlp_memory(vec![
        123, 234, 0xb8, 56, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43,
        44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56,
    ]);

    interpreter.run()?;
    let expected_stack = vec![56.into(), 4.into()]; // len, pos
    assert_eq!(interpreter.stack(), expected_stack);

    Ok(())
}

#[test]
fn test_decode_rlp_list_len_short() -> Result<()> {
    let decode_rlp_list_len = KERNEL.global_labels["decode_rlp_list_len"];

    let initial_stack = vec![0xDEADBEEFu32.into(), 0.into()];
    let mut interpreter = Interpreter::new_with_kernel(decode_rlp_list_len, initial_stack);

    // The RLP encoding of [1, 2, [3, 4]].
    interpreter.set_rlp_memory(vec![0xc5, 1, 2, 0xc2, 3, 4]);

    interpreter.run()?;
    let expected_stack = vec![5.into(), 1.into()]; // len, pos
    assert_eq!(interpreter.stack(), expected_stack);

    Ok(())
}

#[test]
fn test_decode_rlp_list_len_long() -> Result<()> {
    let decode_rlp_list_len = KERNEL.global_labels["decode_rlp_list_len"];

    let initial_stack = vec![0xDEADBEEFu32.into(), 0.into()];
    let mut interpreter = Interpreter::new_with_kernel(decode_rlp_list_len, initial_stack);

    // The RLP encoding of [1, ..., 56].
    interpreter.set_rlp_memory(vec![
        0xf8, 56, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56,
    ]);

    interpreter.run()?;
    let expected_stack = vec![56.into(), 2.into()]; // len, pos
    assert_eq!(interpreter.stack(), expected_stack);

    Ok(())
}

#[test]
fn test_decode_rlp_scalar() -> Result<()> {
    let decode_rlp_scalar = KERNEL.global_labels["decode_rlp_scalar"];

    let initial_stack = vec![0xDEADBEEFu32.into(), 0.into()];
    let mut interpreter = Interpreter::new_with_kernel(decode_rlp_scalar, initial_stack);

    // The RLP encoding of "12 34 56".
    interpreter.set_rlp_memory(vec![0x83, 0x12, 0x34, 0x56]);

    interpreter.run()?;
    let expected_stack = vec![0x123456.into(), 4.into()]; // scalar, pos
    assert_eq!(interpreter.stack(), expected_stack);

    Ok(())
}
