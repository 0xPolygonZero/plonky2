use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment;

#[test]
fn test_mload_packing_1_byte() -> Result<()> {
    let mstore_unpacking = KERNEL.global_labels["mload_packing"];

    let retdest = 0xDEADBEEFu32.into();
    let len = 1.into();
    let offset = 2.into();
    let segment = (Segment::RlpRaw as u32).into();
    let context = 0.into();
    let initial_stack = vec![retdest, len, offset, segment, context];

    let mut interpreter = Interpreter::new_with_kernel(mstore_unpacking, initial_stack);
    interpreter.set_rlp_memory(vec![0, 0, 0xAB]);

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![0xAB.into()]);

    Ok(())
}

#[test]
fn test_mload_packing_3_bytes() -> Result<()> {
    let mstore_unpacking = KERNEL.global_labels["mload_packing"];

    let retdest = 0xDEADBEEFu32.into();
    let len = 3.into();
    let offset = 2.into();
    let segment = (Segment::RlpRaw as u32).into();
    let context = 0.into();
    let initial_stack = vec![retdest, len, offset, segment, context];

    let mut interpreter = Interpreter::new_with_kernel(mstore_unpacking, initial_stack);
    interpreter.set_rlp_memory(vec![0, 0, 0xAB, 0xCD, 0xEF]);

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![0xABCDEF.into()]);

    Ok(())
}

#[test]
fn test_mload_packing_32_bytes() -> Result<()> {
    let mstore_unpacking = KERNEL.global_labels["mload_packing"];

    let retdest = 0xDEADBEEFu32.into();
    let len = 32.into();
    let offset = 0.into();
    let segment = (Segment::RlpRaw as u32).into();
    let context = 0.into();
    let initial_stack = vec![retdest, len, offset, segment, context];

    let mut interpreter = Interpreter::new_with_kernel(mstore_unpacking, initial_stack);
    interpreter.set_rlp_memory(vec![0xFF; 32]);

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![U256::MAX]);

    Ok(())
}

#[test]
fn test_mstore_unpacking() -> Result<()> {
    let mstore_unpacking = KERNEL.global_labels["mstore_unpacking"];

    let retdest = 0xDEADBEEFu32.into();
    let len = 4.into();
    let value = 0xABCD1234u32.into();
    let offset = 0.into();
    let segment = (Segment::TxnData as u32).into();
    let context = 0.into();
    let initial_stack = vec![retdest, len, value, offset, segment, context];

    let mut interpreter = Interpreter::new_with_kernel(mstore_unpacking, initial_stack);

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![4.into()]);
    assert_eq!(
        &interpreter.get_txn_data(),
        &[0xAB.into(), 0xCD.into(), 0x12.into(), 0x34.into()]
    );

    Ok(())
}
