//! Code for extracting trie data after witness generation. This is intended only for debugging.

use std::collections::HashMap;

use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, Node, PartialTrie, WrappedNode};
use ethereum_types::{BigEndianHash, H256, U256, U512};

use super::mpt::{AccountRlp, LegacyReceiptRlp, LogRlp};
use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::memory::segments::Segment;
use crate::util::{u256_to_bool, u256_to_h160, u256_to_u8, u256_to_usize};
use crate::witness::errors::ProgramError;
use crate::witness::memory::{MemoryAddress, MemoryState};

/// Account data as it's stored in the state trie, with a pointer to the storage trie.
#[derive(Debug)]
pub(crate) struct AccountTrieRecord {
    pub(crate) nonce: u64,
    pub(crate) balance: U256,
    pub(crate) storage_ptr: usize,
    pub(crate) code_hash: H256,
}

pub(crate) fn read_state_trie_value(slice: &[U256]) -> Result<AccountTrieRecord, ProgramError> {
    Ok(AccountTrieRecord {
        nonce: slice[0].low_u64(),
        balance: slice[1],
        storage_ptr: u256_to_usize(slice[2])?,
        code_hash: H256::from_uint(&slice[3]),
    })
}

pub(crate) const fn read_storage_trie_value(slice: &[U256]) -> U256 {
    slice[0]
}

pub(crate) fn read_trie<V>(
    memory: &MemoryState,
    ptr: usize,
    read_value: fn(&[U256]) -> Result<V, ProgramError>,
) -> Result<HashMap<Nibbles, V>, ProgramError> {
    let mut res = HashMap::new();
    let empty_nibbles = Nibbles {
        count: 0,
        packed: U512::zero(),
    };
    read_trie_helper::<V>(memory, ptr, read_value, empty_nibbles, &mut res)?;
    Ok(res)
}

pub(crate) fn read_trie_helper<V>(
    memory: &MemoryState,
    ptr: usize,
    read_value: fn(&[U256]) -> Result<V, ProgramError>,
    prefix: Nibbles,
    res: &mut HashMap<Nibbles, V>,
) -> Result<(), ProgramError> {
    let load = |offset| memory.get(MemoryAddress::new(0, Segment::TrieData, offset));
    let load_slice_from = |init_offset| {
        &memory.contexts[0].segments[Segment::TrieData.unscale()].content[init_offset..]
    };

    let trie_type = PartialTrieType::all()[u256_to_usize(load(ptr))?];
    match trie_type {
        PartialTrieType::Empty => Ok(()),
        PartialTrieType::Hash => Ok(()),
        PartialTrieType::Branch => {
            let ptr_payload = ptr + 1;
            for i in 0u8..16 {
                let child_ptr = u256_to_usize(load(ptr_payload + i as usize))?;
                read_trie_helper::<V>(memory, child_ptr, read_value, prefix.merge_nibble(i), res)?;
            }
            let value_ptr = u256_to_usize(load(ptr_payload + 16))?;
            if value_ptr != 0 {
                res.insert(prefix, read_value(load_slice_from(value_ptr))?);
            };

            Ok(())
        }
        PartialTrieType::Extension => {
            let count = u256_to_usize(load(ptr + 1))?;
            let packed = load(ptr + 2);
            let nibbles = Nibbles {
                count,
                packed: packed.into(),
            };
            let child_ptr = u256_to_usize(load(ptr + 3))?;
            read_trie_helper::<V>(
                memory,
                child_ptr,
                read_value,
                prefix.merge_nibbles(&nibbles),
                res,
            )
        }
        PartialTrieType::Leaf => {
            let count = u256_to_usize(load(ptr + 1))?;
            let packed = load(ptr + 2);
            let nibbles = Nibbles {
                count,
                packed: packed.into(),
            };
            let value_ptr = u256_to_usize(load(ptr + 3))?;
            res.insert(
                prefix.merge_nibbles(&nibbles),
                read_value(load_slice_from(value_ptr))?,
            );

            Ok(())
        }
    }
}

pub(crate) fn read_receipt_trie_value(
    slice: &[U256],
) -> Result<(Option<u8>, LegacyReceiptRlp), ProgramError> {
    let first_value = slice[0];
    // Skip two elements for non-legacy Receipts, and only one otherwise.
    let (first_byte, slice) = if first_value == U256::one() || first_value == U256::from(2u8) {
        (Some(first_value.as_u32() as u8), &slice[2..])
    } else {
        (None, &slice[1..])
    };

    let status = u256_to_bool(slice[0])?;
    let cum_gas_used = slice[1];
    let bloom = slice[2..2 + 256]
        .iter()
        .map(|&x| u256_to_u8(x))
        .collect::<Result<_, _>>()?;
    // We read the number of logs at position `2 + 256 + 1`, and skip over the next element before parsing the logs.
    let logs = read_logs(u256_to_usize(slice[2 + 256 + 1])?, &slice[2 + 256 + 3..])?;

    Ok((
        first_byte,
        LegacyReceiptRlp {
            status,
            cum_gas_used,
            bloom,
            logs,
        },
    ))
}

pub(crate) fn read_logs(num_logs: usize, slice: &[U256]) -> Result<Vec<LogRlp>, ProgramError> {
    let mut offset = 0;
    (0..num_logs)
        .map(|_| {
            let address = u256_to_h160(slice[offset])?;
            let num_topics = u256_to_usize(slice[offset + 1])?;

            let topics = (0..num_topics)
                .map(|i| H256::from_uint(&slice[offset + 2 + i]))
                .collect();

            let data_len = u256_to_usize(slice[offset + 2 + num_topics])?;
            let log = LogRlp {
                address,
                topics,
                data: slice[offset + 2 + num_topics + 1..offset + 2 + num_topics + 1 + data_len]
                    .iter()
                    .map(|&x| u256_to_u8(x))
                    .collect::<Result<_, _>>()?,
            };
            offset += 2 + num_topics + 1 + data_len;
            Ok(log)
        })
        .collect()
}

pub(crate) fn read_state_rlp_value(
    memory: &MemoryState,
    slice: &[U256],
) -> Result<Vec<u8>, ProgramError> {
    let storage_trie: HashedPartialTrie = get_trie(memory, slice[2].as_usize(), |_, x| {
        Ok(rlp::encode(&read_storage_trie_value(x)).to_vec())
    })?;
    let account = AccountRlp {
        nonce: slice[0],
        balance: slice[1],
        storage_root: storage_trie.hash(),
        code_hash: H256::from_uint(&slice[3]),
    };
    Ok(rlp::encode(&account).to_vec())
}

pub(crate) fn read_txn_rlp_value(
    _memory: &MemoryState,
    slice: &[U256],
) -> Result<Vec<u8>, ProgramError> {
    let txn_rlp_len = u256_to_usize(slice[0])?;
    slice[1..txn_rlp_len + 1]
        .iter()
        .map(|&x| u256_to_u8(x))
        .collect::<Result<_, _>>()
}

pub(crate) fn read_receipt_rlp_value(
    _memory: &MemoryState,
    slice: &[U256],
) -> Result<Vec<u8>, ProgramError> {
    let (first_byte, receipt) = read_receipt_trie_value(slice)?;
    let mut bytes = rlp::encode(&receipt).to_vec();
    if let Some(txn_byte) = first_byte {
        bytes.insert(0, txn_byte);
    }

    Ok(bytes)
}

pub(crate) fn get_state_trie<N: PartialTrie>(
    memory: &MemoryState,
    ptr: usize,
) -> Result<N, ProgramError> {
    get_trie(memory, ptr, read_state_rlp_value)
}

pub(crate) fn get_txn_trie<N: PartialTrie>(
    memory: &MemoryState,
    ptr: usize,
) -> Result<N, ProgramError> {
    get_trie(memory, ptr, read_txn_rlp_value)
}

pub(crate) fn get_receipt_trie<N: PartialTrie>(
    memory: &MemoryState,
    ptr: usize,
) -> Result<N, ProgramError> {
    get_trie(memory, ptr, read_receipt_rlp_value)
}

pub(crate) fn get_trie<N: PartialTrie>(
    memory: &MemoryState,
    ptr: usize,
    read_rlp_value: fn(&MemoryState, &[U256]) -> Result<Vec<u8>, ProgramError>,
) -> Result<N, ProgramError> {
    let empty_nibbles = Nibbles {
        count: 0,
        packed: U512::zero(),
    };
    Ok(N::new(get_trie_helper(
        memory,
        ptr,
        read_rlp_value,
        empty_nibbles,
    )?))
}

pub(crate) fn get_trie_helper<N: PartialTrie>(
    memory: &MemoryState,
    ptr: usize,
    read_value: fn(&MemoryState, &[U256]) -> Result<Vec<u8>, ProgramError>,
    prefix: Nibbles,
) -> Result<Node<N>, ProgramError> {
    let load = |offset| memory.get(MemoryAddress::new(0, Segment::TrieData, offset));
    let load_slice_from = |init_offset| {
        &memory.contexts[0].segments[Segment::TrieData.unscale()].content[init_offset..]
    };

    let trie_type = PartialTrieType::all()[u256_to_usize(load(ptr))?];
    match trie_type {
        PartialTrieType::Empty => Ok(Node::Empty),
        PartialTrieType::Hash => {
            let ptr_payload = ptr + 1;
            let hash = H256::from_uint(&load(ptr_payload));
            Ok(Node::Hash(hash))
        }
        PartialTrieType::Branch => {
            let ptr_payload = ptr + 1;
            let children = (0..16)
                .map(|i| {
                    let child_ptr = u256_to_usize(load(ptr_payload + i as usize))?;
                    get_trie_helper(memory, child_ptr, read_value, prefix.merge_nibble(i as u8))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let children = core::array::from_fn(|i| WrappedNode::from(children[i].clone()));
            let value_ptr = u256_to_usize(load(ptr_payload + 16))?;
            let mut value: Vec<u8> = vec![];
            if value_ptr != 0 {
                value = read_value(memory, load_slice_from(value_ptr))?;
            };
            Ok(Node::Branch { children, value })
        }
        PartialTrieType::Extension => {
            let count = u256_to_usize(load(ptr + 1))?;
            let packed = load(ptr + 2);
            let nibbles = Nibbles {
                count,
                packed: packed.into(),
            };
            let child_ptr = u256_to_usize(load(ptr + 3))?;
            let child = WrappedNode::from(get_trie_helper(
                memory,
                child_ptr,
                read_value,
                prefix.merge_nibbles(&nibbles),
            )?);
            Ok(Node::Extension { nibbles, child })
        }
        PartialTrieType::Leaf => {
            let count = u256_to_usize(load(ptr + 1))?;
            let packed = load(ptr + 2);
            let nibbles = Nibbles {
                count,
                packed: packed.into(),
            };
            let value_ptr = u256_to_usize(load(ptr + 3))?;
            let value = read_value(memory, load_slice_from(value_ptr))?;
            Ok(Node::Leaf { nibbles, value })
        }
    }
}
