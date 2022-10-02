use anyhow::Result;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

#[test]
fn test_encode_rlp_scalar_small() -> Result<()> {
    let encode_rlp_scalar = KERNEL.global_labels["encode_rlp_scalar"];

    let retdest = 0xDEADBEEFu32.into();
    let scalar = 42.into();
    let pos = 2.into();
    let initial_stack = vec![retdest, scalar, pos];
    let mut interpreter = Interpreter::new_with_kernel(encode_rlp_scalar, initial_stack);

    interpreter.run()?;
    let expected_stack = vec![3.into()]; // pos' = pos + rlp_len = 2 + 1
    let expected_rlp = vec![0, 0, 42];
    assert_eq!(interpreter.stack(), expected_stack);
    assert_eq!(interpreter.get_rlp_memory(), expected_rlp);

    Ok(())
}

#[test]
fn test_encode_rlp_scalar_medium() -> Result<()> {
    let encode_rlp_scalar = KERNEL.global_labels["encode_rlp_scalar"];

    let retdest = 0xDEADBEEFu32.into();
    let scalar = 0x12345.into();
    let pos = 2.into();
    let initial_stack = vec![retdest, scalar, pos];
    let mut interpreter = Interpreter::new_with_kernel(encode_rlp_scalar, initial_stack);

    interpreter.run()?;
    let expected_stack = vec![6.into()]; // pos' = pos + rlp_len = 2 + 4
    let expected_rlp = vec![0, 0, 0x80 + 3, 0x01, 0x23, 0x45];
    assert_eq!(interpreter.stack(), expected_stack);
    assert_eq!(interpreter.get_rlp_memory(), expected_rlp);

    Ok(())
}

#[test]
fn test_encode_rlp_160() -> Result<()> {
    let encode_rlp_160 = KERNEL.global_labels["encode_rlp_160"];

    let retdest = 0xDEADBEEFu32.into();
    let string = 0x12345.into();
    let pos = 0.into();
    let initial_stack = vec![retdest, string, pos];
    let mut interpreter = Interpreter::new_with_kernel(encode_rlp_160, initial_stack);

    interpreter.run()?;
    let expected_stack = vec![(1 + 20).into()]; // pos'
    #[rustfmt::skip]
    let expected_rlp = vec![0x80 + 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0x23, 0x45];
    assert_eq!(interpreter.stack(), expected_stack);
    assert_eq!(interpreter.get_rlp_memory(), expected_rlp);

    Ok(())
}

#[test]
fn test_encode_rlp_256() -> Result<()> {
    let encode_rlp_256 = KERNEL.global_labels["encode_rlp_256"];

    let retdest = 0xDEADBEEFu32.into();
    let string = 0x12345.into();
    let pos = 0.into();
    let initial_stack = vec![retdest, string, pos];
    let mut interpreter = Interpreter::new_with_kernel(encode_rlp_256, initial_stack);

    interpreter.run()?;
    let expected_stack = vec![(1 + 32).into()]; // pos'
    #[rustfmt::skip]
    let expected_rlp = vec![0x80 + 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0x23, 0x45];
    assert_eq!(interpreter.stack(), expected_stack);
    assert_eq!(interpreter.get_rlp_memory(), expected_rlp);

    Ok(())
}

#[test]
fn test_prepend_rlp_list_prefix_small() -> Result<()> {
    let prepend_rlp_list_prefix = KERNEL.global_labels["prepend_rlp_list_prefix"];

    let retdest = 0xDEADBEEFu32.into();
    let end_pos = (9 + 5).into();
    let initial_stack = vec![retdest, end_pos];
    let mut interpreter = Interpreter::new_with_kernel(prepend_rlp_list_prefix, initial_stack);
    interpreter.set_rlp_memory(vec![
        // Nine 0s to leave room for the longest possible RLP list prefix.
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        // The actual RLP list payload, consisting of 5 tiny strings.
        1, 2, 3, 4, 5,
    ]);

    interpreter.run()?;

    let expected_rlp_len = 6.into();
    let expected_start_pos = 8.into();
    let expected_stack = vec![expected_rlp_len, expected_start_pos];
    let expected_rlp = vec![0, 0, 0, 0, 0, 0, 0, 0, 0xc0 + 5, 1, 2, 3, 4, 5];

    assert_eq!(interpreter.stack(), expected_stack);
    assert_eq!(interpreter.get_rlp_memory(), expected_rlp);

    Ok(())
}

#[test]
fn test_prepend_rlp_list_prefix_large() -> Result<()> {
    let prepend_rlp_list_prefix = KERNEL.global_labels["prepend_rlp_list_prefix"];

    let retdest = 0xDEADBEEFu32.into();
    let end_pos = (9 + 60).into();
    let initial_stack = vec![retdest, end_pos];
    let mut interpreter = Interpreter::new_with_kernel(prepend_rlp_list_prefix, initial_stack);

    #[rustfmt::skip]
    interpreter.set_rlp_memory(vec![
        // Nine 0s to leave room for the longest possible RLP list prefix.
        0, 0, 0, 0, 0, 0, 0, 0, 0,
        // The actual RLP list payload, consisting of 60 tiny strings.
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
        30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
        40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
        50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
    ]);

    interpreter.run()?;

    let expected_rlp_len = 62.into();
    let expected_start_pos = 7.into();
    let expected_stack = vec![expected_rlp_len, expected_start_pos];

    #[rustfmt::skip]
    let expected_rlp = vec![
        0, 0, 0, 0, 0, 0, 0, 0xf7 + 1, 60,
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
        30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
        40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
        50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
    ];

    assert_eq!(interpreter.stack(), expected_stack);
    assert_eq!(interpreter.get_rlp_memory(), expected_rlp);

    Ok(())
}
