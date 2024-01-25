use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment;

#[test]
fn test_mstore_unpacking() -> Result<()> {
    let mstore_unpacking = KERNEL.global_labels["mstore_unpacking"];

    let retdest = 0xDEADBEEFu32.into();
    let len = 4.into();
    let value = 0xABCD1234u32.into();
    let addr = (Segment::TxnData as u64).into();
    let initial_stack = vec![retdest, len, value, addr];

    let mut interpreter = Interpreter::new_with_kernel(mstore_unpacking, initial_stack);

    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![addr + U256::from(4)]);
    assert_eq!(
        &interpreter.get_txn_data(),
        &[0xAB.into(), 0xCD.into(), 0x12.into(), 0x34.into()]
    );

    Ok(())
}
