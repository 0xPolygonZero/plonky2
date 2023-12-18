use anyhow::{anyhow, Result};
use ethereum_types::{Address, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::txn_fields::NormalizedTxnField;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::account_code::initialize_mpts;
use crate::generation::mpt::{LegacyReceiptRlp, LogRlp};
use crate::memory::segments::Segment;

#[test]
fn test_process_receipt() -> Result<()> {
    /* Tests process_receipt, which:
    - computes the cumulative gas
    - computes the bloom filter
    - inserts the receipt data in MPT_TRIE_DATA
    - inserts a node in receipt_trie
    - resets the bloom filter to 0 for the next transaction. */
    let process_receipt = KERNEL.global_labels["process_receipt"];
    let success = U256::from(1);
    let leftover_gas = U256::from(4000);
    let prev_cum_gas = U256::from(1000);
    let retdest = 0xDEADBEEFu32.into();

    // Log.
    let address: Address = thread_rng().gen();
    let num_topics = 1;

    let mut topic = vec![0_u8; 32];
    topic[31] = 4;

    // Compute the expected Bloom filter.
    let test_logs_list = vec![(address.to_fixed_bytes().to_vec(), vec![topic])];
    let expected_bloom = logs_bloom_bytes_fn(test_logs_list).to_vec();

    // Set memory.
    let num_nibbles = 2.into();
    let initial_stack: Vec<U256> = vec![
        retdest,
        num_nibbles,
        0.into(),
        prev_cum_gas,
        leftover_gas,
        success,
    ];
    let mut interpreter = Interpreter::new_with_kernel(process_receipt, initial_stack);
    interpreter.set_memory_segment(
        Segment::LogsData,
        vec![
            56.into(),                                        // payload len
            U256::from_big_endian(&address.to_fixed_bytes()), // address
            num_topics.into(),                                // num_topics
            4.into(),                                         // topic
            0.into(),                                         // data_len
        ],
    );
    interpreter.set_txn_field(NormalizedTxnField::GasLimit, U256::from(5000));
    interpreter.set_memory_segment(Segment::TxnBloom, vec![0.into(); 256]);
    interpreter.set_memory_segment(Segment::Logs, vec![0.into()]);
    interpreter.set_global_metadata_field(GlobalMetadata::LogsPayloadLen, 58.into());
    interpreter.set_global_metadata_field(GlobalMetadata::LogsLen, U256::from(1));
    interpreter.set_global_metadata_field(GlobalMetadata::ReceiptTrieRoot, 500.into());
    interpreter.run()?;

    let segment_read = interpreter.get_memory_segment(Segment::TrieData);

    // The expected TrieData has the form [payload_len, status, cum_gas_used, bloom_filter, logs_payload_len, num_logs, [logs]]
    let mut expected_trie_data: Vec<U256> = vec![323.into(), success, 2000.into()];
    expected_trie_data.extend(
        expected_bloom
            .into_iter()
            .map(|elt| elt.into())
            .collect::<Vec<U256>>(),
    );
    expected_trie_data.push(58.into()); // logs_payload_len
    expected_trie_data.push(1.into()); // num_logs
    expected_trie_data.extend(vec![
        56.into(),                                        // payload len
        U256::from_big_endian(&address.to_fixed_bytes()), // address
        num_topics.into(),                                // num_topics
        4.into(),                                         // topic
        0.into(),                                         // data_len
    ]);

    assert_eq!(
        expected_trie_data,
        segment_read[0..expected_trie_data.len()]
    );

    Ok(())
}

/// Values taken from the block 1000000 of Goerli: https://goerli.etherscan.io/txs?block=1000000
#[test]
fn test_receipt_encoding() -> Result<()> {
    // Initialize interpreter.
    let success = U256::from(1);

    let retdest = 0xDEADBEEFu32.into();
    let num_topics = 3;

    let encode_receipt = KERNEL.global_labels["encode_receipt"];

    // Logs and receipt in encodable form.
    let log_1 = LogRlp {
        address: hex!("7ef66b77759e12Caf3dDB3E4AFF524E577C59D8D").into(),
        topics: vec![
            hex!("8a22ee899102a366ac8ad0495127319cb1ff2403cfae855f83a89cda1266674d").into(),
            hex!("0000000000000000000000000000000000000000000000000000000000000004").into(),
            hex!("00000000000000000000000000000000000000000000000000000000004920ea").into(),
        ],
        data: hex!("a814f7df6a2203dc0e472e8828be95957c6b329fee8e2b1bb6f044c1eb4fc243")
            .to_vec()
            .into(),
    };

    let receipt_1 = LegacyReceiptRlp {
            status: true,
            cum_gas_used: 0x02dcb6u64.into(),
            bloom: hex!("00000000000000000000000000000000000000000000000000800000000000000040000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000008000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000400000000000000000000000000000002000040000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000008000000000000000000000000").to_vec().into(),
            logs: vec![log_1],
        };
    // Get the expected RLP encoding.
    let expected_rlp = rlp::encode(&rlp::encode(&receipt_1));

    let initial_stack: Vec<U256> = vec![retdest, 0.into(), 0.into(), 0.into()];
    let mut interpreter = Interpreter::new_with_kernel(encode_receipt, initial_stack);

    // Write data to memory.
    let expected_bloom_bytes = vec![
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 0x80, 00, 00, 00, 00, 00, 00, 00, 0x40, 00, 00, 00, 00, 0x10, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x02, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 0x08, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 0x01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 0x01, 00, 00, 00, 0x40, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 0x20, 00, 0x04, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x80, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x08,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
    ];
    let expected_bloom: Vec<U256> = expected_bloom_bytes
        .into_iter()
        .map(|elt| elt.into())
        .collect();

    let addr = U256::from([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x7e, 0xf6, 0x6b, 0x77, 0x75, 0x9e, 0x12, 0xca, 0xf3,
        0xdd, 0xb3, 0xe4, 0xaf, 0xf5, 0x24, 0xe5, 0x77, 0xc5, 0x9d, 0x8d,
    ]);

    let topic1 = U256::from([
        0x8a, 0x22, 0xee, 0x89, 0x91, 0x02, 0xa3, 0x66, 0xac, 0x8a, 0xd0, 0x49, 0x51, 0x27, 0x31,
        0x9c, 0xb1, 0xff, 0x24, 0x03, 0xcf, 0xae, 0x85, 0x5f, 0x83, 0xa8, 0x9c, 0xda, 0x12, 0x66,
        0x67, 0x4d,
    ]);

    let topic2 = 4.into();
    let topic3 = 0x4920ea.into();

    let mut logs = vec![
        155.into(), // unused
        addr,
        num_topics.into(), // num_topics
        topic1,            // topic1
        topic2,            // topic2
        topic3,            // topic3
        32.into(),         // data length
    ];
    let cur_data = hex!("a814f7df6a2203dc0e472e8828be95957c6b329fee8e2b1bb6f044c1eb4fc243")
        .iter()
        .copied()
        .map(U256::from);
    logs.extend(cur_data);

    let mut receipt = vec![423.into(), success, receipt_1.cum_gas_used];
    receipt.extend(expected_bloom.clone());
    receipt.push(157.into()); // logs_payload_len
    receipt.push(1.into()); // num_logs
    receipt.extend(logs.clone());
    interpreter.set_memory_segment(Segment::LogsData, logs);

    interpreter.set_memory_segment(Segment::TxnBloom, expected_bloom);

    interpreter.set_memory_segment(Segment::Logs, vec![0.into()]);
    interpreter.set_global_metadata_field(GlobalMetadata::LogsLen, 1.into());
    interpreter.set_global_metadata_field(GlobalMetadata::LogsPayloadLen, 157.into());
    interpreter.set_memory_segment(Segment::TrieData, receipt);

    interpreter.run()?;
    let rlp_pos = interpreter.pop().expect("The stack should not be empty");

    let rlp_read: Vec<u8> = interpreter.get_rlp_memory();

    assert_eq!(rlp_pos.as_usize(), expected_rlp.len());
    for i in 0..rlp_read.len() {
        assert_eq!(rlp_read[i], expected_rlp[i]);
    }

    Ok(())
}

/// Values taken from the block 1000000 of Goerli: https://goerli.etherscan.io/txs?block=1000000
#[test]
fn test_receipt_bloom_filter() -> Result<()> {
    let logs_bloom = KERNEL.global_labels["logs_bloom"];

    let num_topics = 3;

    // Expected bloom
    let first_bloom_bytes = vec![
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 0x80, 00, 00, 00, 00, 00, 00, 00, 0x40, 00, 00, 00, 00, 0x50, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x02, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 0x08, 00, 0x08, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x50, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x10,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x20, 00, 00, 00, 00, 00, 0x08, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
    ];

    let retdest = 0xDEADBEEFu32.into();

    let addr = U256::from([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x7e, 0xf6, 0x6b, 0x77, 0x75, 0x9e, 0x12, 0xca, 0xf3,
        0xdd, 0xb3, 0xe4, 0xaf, 0xf5, 0x24, 0xe5, 0x77, 0xc5, 0x9d, 0x8d,
    ]);

    let topic1 = U256::from([
        0x8a, 0x22, 0xee, 0x89, 0x91, 0x02, 0xa3, 0x66, 0xac, 0x8a, 0xd0, 0x49, 0x51, 0x27, 0x31,
        0x9c, 0xb1, 0xff, 0x24, 0x03, 0xcf, 0xae, 0x85, 0x5f, 0x83, 0xa8, 0x9c, 0xda, 0x12, 0x66,
        0x67, 0x4d,
    ]);

    let topic02 = 0x2a.into();
    let topic03 = 0xbd9fe6.into();

    // Set logs memory and initialize TxnBloom and BlockBloom segments.
    let initial_stack: Vec<U256> = vec![retdest];

    let mut interpreter = Interpreter::new_with_kernel(logs_bloom, initial_stack);
    let mut logs = vec![
        0.into(), // unused
        addr,
        num_topics.into(), // num_topics
        topic1,            // topic1
        topic02,           // topic2
        topic03,           // topic3
        32.into(),         // data_len
    ];
    let cur_data = hex!("a814f7df6a2203dc0e472e8828be95957c6b329fee8e2b1bb6f044c1eb4fc243")
        .iter()
        .copied()
        .map(U256::from);
    logs.extend(cur_data);
    // The Bloom filter initialization is required for this test to ensure we have the correct length for the filters. Otherwise, some trailing zeroes could be missing.
    interpreter.set_memory_segment(Segment::TxnBloom, vec![0.into(); 256]); // Initialize transaction Bloom filter.
    interpreter.set_memory_segment(Segment::LogsData, logs);
    interpreter.set_memory_segment(Segment::Logs, vec![0.into()]);
    interpreter.set_global_metadata_field(GlobalMetadata::LogsLen, U256::from(1));
    interpreter.run()?;

    // Second transaction.
    let loaded_bloom_u256 = interpreter.get_memory_segment(Segment::TxnBloom);
    let loaded_bloom: Vec<u8> = loaded_bloom_u256
        .into_iter()
        .map(|elt| elt.0[0] as u8)
        .collect();

    assert_eq!(first_bloom_bytes, loaded_bloom);
    let topic12 = 0x4.into();
    let topic13 = 0x4920ea.into();
    let mut logs2 = vec![
        0.into(), // unused
        addr,
        num_topics.into(), // num_topics
        topic1,            // topic1
        topic12,           // topic2
        topic13,           // topic3
        32.into(),         // data_len
    ];
    let cur_data = hex!("a814f7df6a2203dc0e472e8828be95957c6b329fee8e2b1bb6f044c1eb4fc243")
        .iter()
        .copied()
        .map(U256::from);
    logs2.extend(cur_data);

    interpreter
        .push(retdest)
        .expect("The stack should not overflow");
    interpreter.generation_state.registers.program_counter = logs_bloom;
    interpreter.set_memory_segment(Segment::TxnBloom, vec![0.into(); 256]); // Initialize transaction Bloom filter.
    interpreter.set_memory_segment(Segment::LogsData, logs2);
    interpreter.set_memory_segment(Segment::Logs, vec![0.into()]);
    interpreter.set_global_metadata_field(GlobalMetadata::LogsLen, U256::from(1));
    interpreter.run()?;

    let second_bloom_bytes = vec![
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 0x80, 00, 00, 00, 00, 00, 00, 00, 0x40, 00, 00, 00, 00, 0x10, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x02, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 0x08, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 0x01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 0x01, 00, 00, 00, 0x40, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 0x20, 00, 0x04, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x80, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x08,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
    ];

    let second_loaded_bloom_u256 = interpreter.get_memory_segment(Segment::TxnBloom);
    let second_loaded_bloom: Vec<u8> = second_loaded_bloom_u256
        .into_iter()
        .map(|elt| elt.0[0] as u8)
        .collect();

    assert_eq!(second_bloom_bytes, second_loaded_bloom);

    Ok(())
}

#[test]
fn test_mpt_insert_receipt() -> Result<()> {
    // This test simulates a receipt processing to test `mpt_insert_receipt_trie`.
    // For this, we need to set the data correctly in memory.
    // In TrieData, we need to insert a receipt of the form:
    // `[payload_len, status, cum_gas_used, bloom, logs_payload_len, num_logs, [logs]]`.
    // We also need to set TrieDataSize correctly.

    let retdest = 0xDEADBEEFu32.into();
    let trie_inputs = Default::default();
    let mpt_insert = KERNEL.global_labels["mpt_insert_receipt_trie"];
    let num_topics = 3; // Both transactions have the same number of topics.
    let payload_len = 423; // Total payload length for each receipt.
    let logs_payload_len = 157; // Payload length for all logs.
    let log_payload_len = 155; // Payload length for one log.
    let num_logs = 1;

    // Receipt_0:
    let status_0 = 1;
    let cum_gas_used_0 = 0x016e5b;
    let logs_bloom_0_bytes = vec![
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 0x80, 00, 00, 00, 00, 00, 00, 00, 0x40, 00, 00, 00, 00, 0x50, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x02, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 0x08, 00, 0x08, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x50, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x10,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x20, 00, 00, 00, 00, 00, 0x08, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
    ];

    // Logs_0:
    let logs_bloom_0: Vec<U256> = logs_bloom_0_bytes
        .into_iter()
        .map(|elt| elt.into())
        .collect();

    let addr = U256::from([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x7e, 0xf6, 0x6b, 0x77, 0x75, 0x9e, 0x12, 0xca, 0xf3,
        0xdd, 0xb3, 0xe4, 0xaf, 0xf5, 0x24, 0xe5, 0x77, 0xc5, 0x9d, 0x8d,
    ]);

    // The first topic is shared by the two transactions.
    let topic1 = U256::from([
        0x8a, 0x22, 0xee, 0x89, 0x91, 0x02, 0xa3, 0x66, 0xac, 0x8a, 0xd0, 0x49, 0x51, 0x27, 0x31,
        0x9c, 0xb1, 0xff, 0x24, 0x03, 0xcf, 0xae, 0x85, 0x5f, 0x83, 0xa8, 0x9c, 0xda, 0x12, 0x66,
        0x67, 0x4d,
    ]);

    let topic02 = 0x2a.into();
    let topic03 = 0xbd9fe6.into();

    let mut logs_0 = vec![
        log_payload_len.into(), // payload_len
        addr,
        num_topics.into(), // num_topics
        topic1,            // topic1
        topic02,           // topic2
        topic03,           // topic3
        32.into(),         // data_len
    ];
    let cur_data = hex!("f7af1cc94b1aef2e0fa15f1b4baefa86eb60e78fa4bd082372a0a446d197fb58")
        .iter()
        .copied()
        .map(U256::from);
    logs_0.extend(cur_data);

    let mut receipt: Vec<U256> = vec![423.into(), status_0.into(), cum_gas_used_0.into()];
    receipt.extend(logs_bloom_0);
    receipt.push(logs_payload_len.into()); // logs_payload_len
    receipt.push(num_logs.into()); // num_logs
    receipt.extend(logs_0.clone());

    // First, we load all mpts.
    let initial_stack: Vec<U256> = vec![retdest];

    let mut interpreter = Interpreter::new_with_kernel(0, vec![]);
    initialize_mpts(&mut interpreter, &trie_inputs);

    // If TrieData is empty, we need to push 0 because the first value is always 0.
    let mut cur_trie_data = interpreter.get_memory_segment(Segment::TrieData);
    if cur_trie_data.is_empty() {
        cur_trie_data.push(0.into());
    }

    // stack: transaction_nb, value_ptr, retdest
    let num_nibbles = 2;
    let initial_stack: Vec<U256> = vec![
        retdest,
        cur_trie_data.len().into(),
        0x80.into(),
        num_nibbles.into(),
    ];
    for i in 0..initial_stack.len() {
        interpreter
            .push(initial_stack[i])
            .expect("The stack should not overflow");
    }

    interpreter.generation_state.registers.program_counter = mpt_insert;

    // Set memory.
    cur_trie_data.extend(receipt);
    interpreter.set_memory_segment(Segment::TrieData, cur_trie_data.clone());
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, cur_trie_data.len().into());
    // First insertion.
    interpreter.run()?;

    // receipt_1:
    let status_1 = 1;
    let cum_gas_used_1 = 0x02dcb6;
    let logs_bloom_1_bytes = vec![
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 0x80, 00, 00, 00, 00, 00, 00, 00, 0x40, 00, 00, 00, 00, 0x10, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x02, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 0x08, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 0x01, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 0x01, 00, 00, 00, 0x40, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 0x20, 00, 0x04, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x80, 00, 00, 00, 00, 00, 00, 00, 00, 00,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 0x08,
        00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00,
    ];

    // Logs_1:
    let logs_bloom_1: Vec<U256> = logs_bloom_1_bytes
        .into_iter()
        .map(|elt| elt.into())
        .collect();

    let topic12 = 4.into();
    let topic13 = 0x4920ea.into();

    let mut logs_1 = vec![
        log_payload_len.into(), // payload length
        addr,
        num_topics.into(), // nb topics
        topic1,            // topic1
        topic12,           // topic2
        topic13,           // topic3
        32.into(),         // data length
    ];
    let cur_data = hex!("a814f7df6a2203dc0e472e8828be95957c6b329fee8e2b1bb6f044c1eb4fc243")
        .iter()
        .copied()
        .map(U256::from);
    logs_1.extend(cur_data);

    let mut receipt_1: Vec<U256> = vec![payload_len.into(), status_1.into(), cum_gas_used_1.into()];
    receipt_1.extend(logs_bloom_1);
    receipt_1.push(logs_payload_len.into()); // logs payload len
    receipt_1.push(num_logs.into()); // nb logs
    receipt_1.extend(logs_1.clone());

    // Get updated TrieData segment.
    cur_trie_data = interpreter.get_memory_segment(Segment::TrieData);
    let num_nibbles = 2;
    let initial_stack2: Vec<U256> = vec![
        retdest,
        cur_trie_data.len().into(),
        0x01.into(),
        num_nibbles.into(),
    ];
    for i in 0..initial_stack2.len() {
        interpreter
            .push(initial_stack2[i])
            .expect("The stack should not overflow");
    }
    cur_trie_data.extend(receipt_1);

    // Set memory.
    interpreter.generation_state.registers.program_counter = mpt_insert;
    interpreter.set_memory_segment(Segment::TrieData, cur_trie_data.clone());
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, cur_trie_data.len().into());
    interpreter.run()?;

    // Finally, check that the hashes correspond.
    let mpt_hash_receipt = KERNEL.global_labels["mpt_hash_receipt_trie"];
    interpreter.generation_state.registers.program_counter = mpt_hash_receipt;
    interpreter
        .push(retdest)
        .expect("The stack should not overflow");
    interpreter
        .push(1.into()) // Initial length of the trie data segment, unused.; // Initial length of the trie data segment, unused.
        .expect("The stack should not overflow");
    interpreter.run()?;
    assert_eq!(
        interpreter.stack()[1],
        U256::from(hex!(
            "da46cdd329bfedace32da95f2b344d314bc6f55f027d65f9f4ac04ee425e1f98"
        ))
    );
    Ok(())
}

#[test]
fn test_bloom_two_logs() -> Result<()> {
    // Tests the Bloom filter computation with two logs in one transaction.

    // address
    let to = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x09, 0x5e, 0x7b, 0xae, 0xa6, 0xa6, 0xc7, 0xc4, 0xc2,
        0xdf, 0xeb, 0x97, 0x7e, 0xfa, 0xc3, 0x26, 0xaf, 0x55, 0x2d, 0x87,
    ];

    let retdest = 0xDEADBEEFu32.into();
    let logs_bloom = KERNEL.global_labels["logs_bloom"];

    let initial_stack: Vec<U256> = vec![retdest];

    // Set memory.
    let logs = vec![
        0.into(),  // unused
        to.into(), // address
        0.into(),  // num_topics
        0.into(),  // data_len,
        0.into(),  // unused: rlp
        to.into(),
        2.into(), // num_topics
        0x62.into(),
        0x63.into(),
        5.into(),
        [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xa1,
            0xb2, 0xc3, 0xd4, 0xe5,
        ]
        .into(),
    ];
    let mut interpreter = Interpreter::new_with_kernel(logs_bloom, initial_stack);
    interpreter.set_memory_segment(Segment::TxnBloom, vec![0.into(); 256]); // Initialize transaction Bloom filter.
    interpreter.set_memory_segment(Segment::LogsData, logs);
    interpreter.set_memory_segment(Segment::Logs, vec![0.into(), 4.into()]);
    interpreter.set_global_metadata_field(GlobalMetadata::LogsLen, U256::from(2));
    interpreter.run()?;

    let loaded_bloom_bytes: Vec<u8> = interpreter
        .get_memory_segment(Segment::TxnBloom)
        .into_iter()
        .map(|elt| elt.0[0] as u8)
        .collect();

    let expected = hex!("00000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000004000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000000000000000400000000000040000000000000000000000000002000000000000000000000000000").to_vec();

    assert_eq!(expected, loaded_bloom_bytes);
    Ok(())
}

fn logs_bloom_bytes_fn(logs_list: Vec<(Vec<u8>, Vec<Vec<u8>>)>) -> [u8; 256] {
    // The first element of logs_list.
    let mut bloom = [0_u8; 256];

    for log in logs_list {
        let cur_addr = log.0;
        let topics = log.1;

        add_to_bloom(&mut bloom, &cur_addr);
        for topic in topics {
            add_to_bloom(&mut bloom, &topic);
        }
    }
    bloom
}

fn add_to_bloom(bloom: &mut [u8; 256], bloom_entry: &[u8]) {
    let bloom_hash = keccak(bloom_entry).to_fixed_bytes();

    for idx in 0..3 {
        let bit_pair = u16::from_be_bytes(bloom_hash[2 * idx..2 * (idx + 1)].try_into().unwrap());
        let bit_to_set = 0x07FF - (bit_pair & 0x07FF);
        let byte_index = bit_to_set / 8;
        let bit_value = 1 << (7 - bit_to_set % 8);
        bloom[byte_index as usize] |= bit_value;
    }
}
