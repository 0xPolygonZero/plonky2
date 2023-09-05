use anyhow::Result;
use ethereum_types::{H256, U256};
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::memory::segments::Segment;

#[test]
fn test_correct_block_hash() -> Result<()> {
    let mut rng = rand::thread_rng();

    let blockhash_label = KERNEL.global_labels["blockhash"];
    let retdest = 0xDEADBEEFu32.into();

    let block_number: u8 = rng.gen();
    let initial_stack = vec![retdest, block_number.into()];

    let hashes: Vec<U256> = vec![U256::from_big_endian(&thread_rng().gen::<H256>().0); 257];

    let mut interpreter = Interpreter::new_with_kernel(blockhash_label, initial_stack);
    interpreter.set_memory_segment(Segment::BlockHashes, hashes[0..256].to_vec());
    interpreter.set_global_metadata_field(GlobalMetadata::BlockCurrentHash, hashes[256]);
    interpreter.set_global_metadata_field(GlobalMetadata::BlockNumber, 256.into());
    interpreter.run()?;

    let result = interpreter.stack();
    assert_eq!(
        result[0], hashes[block_number as usize],
        "Resulting block hash {:?} different from expected hash {:?}",
        result[0], hashes[block_number as usize]
    );

    Ok(())
}

#[test]
fn test_big_index_block_hash() -> Result<()> {
    let mut rng = rand::thread_rng();

    let blockhash_label = KERNEL.global_labels["blockhash"];
    let retdest = 0xDEADBEEFu32.into();
    let cur_block_number = 3;
    let block_number: usize = rng.gen::<u8>() as usize;
    let actual_block_number = block_number + cur_block_number;
    let initial_stack = vec![retdest, actual_block_number.into()];

    let hashes: Vec<U256> = vec![U256::from_big_endian(&thread_rng().gen::<H256>().0); 257];

    let mut interpreter = Interpreter::new_with_kernel(blockhash_label, initial_stack);
    interpreter.set_memory_segment(Segment::BlockHashes, hashes[0..256].to_vec());
    interpreter.set_global_metadata_field(GlobalMetadata::BlockCurrentHash, hashes[256]);
    interpreter.set_global_metadata_field(GlobalMetadata::BlockNumber, cur_block_number.into());
    interpreter.run()?;

    let result = interpreter.stack();
    assert_eq!(
        result[0],
        0.into(),
        "Resulting block hash {:?} different from expected hash {:?}",
        result[0],
        0
    );

    Ok(())
}

#[test]
fn test_small_index_block_hash() -> Result<()> {
    let mut rng = rand::thread_rng();

    let blockhash_label = KERNEL.global_labels["blockhash"];
    let retdest = 0xDEADBEEFu32.into();
    let cur_block_number = 512;
    let block_number = rng.gen::<u8>() as usize;
    let initial_stack = vec![retdest, block_number.into()];

    let hashes: Vec<U256> = vec![U256::from_big_endian(&thread_rng().gen::<H256>().0); 257];

    let mut interpreter = Interpreter::new_with_kernel(blockhash_label, initial_stack);
    interpreter.set_memory_segment(Segment::BlockHashes, hashes[0..256].to_vec());
    interpreter.set_global_metadata_field(GlobalMetadata::BlockCurrentHash, hashes[256]);
    interpreter.set_global_metadata_field(GlobalMetadata::BlockNumber, cur_block_number.into());
    interpreter.run()?;

    let result = interpreter.stack();
    assert_eq!(
        result[0],
        0.into(),
        "Resulting block hash {:?} different from expected hash {:?}",
        result[0],
        0
    );

    Ok(())
}

#[test]
fn test_block_hash_with_overflow() -> Result<()> {
    let blockhash_label = KERNEL.global_labels["blockhash"];
    let retdest = 0xDEADBEEFu32.into();
    let cur_block_number = 1;
    let block_number = U256::MAX;
    let initial_stack = vec![retdest, block_number];

    let hashes: Vec<U256> = vec![U256::from_big_endian(&thread_rng().gen::<H256>().0); 257];

    let mut interpreter = Interpreter::new_with_kernel(blockhash_label, initial_stack);
    interpreter.set_memory_segment(Segment::BlockHashes, hashes[0..256].to_vec());
    interpreter.set_global_metadata_field(GlobalMetadata::BlockCurrentHash, hashes[256]);
    interpreter.set_global_metadata_field(GlobalMetadata::BlockNumber, cur_block_number.into());
    interpreter.run()?;

    let result = interpreter.stack();
    assert_eq!(
        result[0],
        0.into(),
        "Resulting block hash {:?} different from expected hash {:?}",
        result[0],
        0
    );

    Ok(())
}
