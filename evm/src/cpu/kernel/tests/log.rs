use anyhow::Result;
use ethereum_types::{Address, U256};
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment;

#[test]
fn test_log_0() -> Result<()> {
    let logs_entry = KERNEL.global_labels["log_n_entry"];
    let address: Address = thread_rng().gen();
    let num_topics = U256::from(0);
    let data_len = U256::from(0);
    let data_offset = U256::from(0);

    let retdest = 0xDEADBEEFu32.into();

    let initial_stack = vec![
        retdest,
        data_offset,
        data_len,
        num_topics,
        U256::from_big_endian(&address.to_fixed_bytes()),
    ];

    let mut interpreter = Interpreter::new_with_kernel(logs_entry, initial_stack);
    interpreter.set_global_metadata_field(GlobalMetadata::LogsLen, 0.into());
    interpreter.set_global_metadata_field(GlobalMetadata::LogsDataLen, 0.into());

    interpreter.run()?;

    // The address is encoded in 1+20 bytes. There are no topics or data, so each is encoded in 1 byte. This leads to a payload of 23.
    let payload_len = 23;
    assert_eq!(
        interpreter.get_memory_segment(Segment::LogsData),
        [
            payload_len.into(),
            U256::from_big_endian(&address.to_fixed_bytes()),
            0.into(),
            0.into(),
        ]
    );
    Ok(())
}

#[test]
fn test_log_2() -> Result<()> {
    let logs_entry = KERNEL.global_labels["log_n_entry"];
    let address: Address = thread_rng().gen();
    let num_topics = U256::from(2);
    let topics = [4.into(), 5.into()];
    let data_len = U256::from(3);
    let data_offset = U256::from(0);

    let memory = vec![10.into(), 20.into(), 30.into()];

    let retdest = 0xDEADBEEFu32.into();

    let initial_stack = vec![
        retdest,
        data_offset,
        data_len,
        topics[1],
        topics[0],
        num_topics,
        U256::from_big_endian(&address.to_fixed_bytes()),
    ];

    let mut interpreter = Interpreter::new_with_kernel(logs_entry, initial_stack);
    interpreter.set_global_metadata_field(GlobalMetadata::LogsLen, 2.into());
    interpreter.set_global_metadata_field(GlobalMetadata::LogsDataLen, 5.into());

    interpreter.set_memory_segment(Segment::MainMemory, memory);

    interpreter.run()?;
    assert_eq!(
        interpreter.get_memory_segment(Segment::Logs),
        [0.into(), 0.into(), 5.into(),]
    );

    // The data has length 3 bytes, and is encoded in 4 bytes. Each of the two topics is encoded in 1+32 bytes. The prefix for the topics list requires 2 bytes. The address is encoded in 1+20 bytes. Overall, we have a logs payload length of 93 bytes.
    let payload_len = 93;
    assert_eq!(
        interpreter.get_memory_segment(Segment::LogsData),
        [
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            payload_len.into(),
            U256::from_big_endian(&address.to_fixed_bytes()),
            2.into(),
            4.into(),
            5.into(),
            3.into(),
            10.into(),
            20.into(),
            30.into(),
        ]
    );
    Ok(())
}

#[test]
fn test_log_4() -> Result<()> {
    let logs_entry = KERNEL.global_labels["log_n_entry"];
    let address: Address = thread_rng().gen();
    let num_topics = U256::from(4);
    let topics = [45.into(), 46.into(), 47.into(), 48.into()];
    let data_len = U256::from(1);
    let data_offset = U256::from(2);

    let memory = vec![0.into(), 0.into(), 123.into()];

    let retdest = 0xDEADBEEFu32.into();

    let initial_stack = vec![
        retdest,
        data_offset,
        data_len,
        topics[3],
        topics[2],
        topics[1],
        topics[0],
        num_topics,
        U256::from_big_endian(&address.to_fixed_bytes()),
    ];

    let mut interpreter = Interpreter::new_with_kernel(logs_entry, initial_stack);
    interpreter.set_global_metadata_field(GlobalMetadata::LogsLen, 2.into());
    interpreter.set_global_metadata_field(GlobalMetadata::LogsDataLen, 5.into());

    interpreter.set_memory_segment(Segment::MainMemory, memory);

    interpreter.run()?;
    assert_eq!(
        interpreter.get_memory_segment(Segment::Logs),
        [0.into(), 0.into(), 5.into(),]
    );

    // The data is of length 1 byte, and is encoded in 1 byte. Each of the four topics is encoded in 1+32 bytes. The topics list is prefixed by 2 bytes. The address is encoded in 1+20 bytes. Overall, this leads to a log payload length of 156.
    let payload_len = 156;
    assert_eq!(
        interpreter.get_memory_segment(Segment::LogsData),
        [
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            payload_len.into(),
            U256::from_big_endian(&address.to_fixed_bytes()),
            4.into(),
            45.into(),
            46.into(),
            47.into(),
            48.into(),
            1.into(),
            123.into(),
        ]
    );
    Ok(())
}

#[test]
fn test_log_5() -> Result<()> {
    let logs_entry = KERNEL.global_labels["log_n_entry"];
    let address: Address = thread_rng().gen();
    let num_topics = U256::from(5);
    let topics = [1.into(), 2.into(), 3.into(), 4.into(), 5.into()];
    let data_len = U256::from(0);
    let data_offset = U256::from(0);

    let retdest = 0xDEADBEEFu32.into();

    let initial_stack = vec![
        retdest,
        data_offset,
        data_len,
        topics[4],
        topics[3],
        topics[2],
        topics[1],
        topics[0],
        num_topics,
        U256::from_big_endian(&address.to_fixed_bytes()),
    ];

    let mut interpreter = Interpreter::new_with_kernel(logs_entry, initial_stack);
    interpreter.set_global_metadata_field(GlobalMetadata::LogsLen, 0.into());
    interpreter.set_global_metadata_field(GlobalMetadata::LogsDataLen, 0.into());

    assert!(interpreter.run().is_err());
    Ok(())
}
