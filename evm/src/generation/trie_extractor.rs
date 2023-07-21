//! Code for extracting trie data after witness generation. This is intended only for debugging.

use std::collections::HashMap;

use eth_trie_utils::nibbles::Nibbles;
use ethereum_types::{BigEndianHash, H256, U256, U512};

use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::memory::segments::Segment;
use crate::witness::memory::{MemoryAddress, MemoryState};

/// Account data as it's stored in the state trie, with a pointer to the storage trie.
#[derive(Debug)]
pub(crate) struct AccountTrieRecord {
    pub(crate) nonce: u64,
    pub(crate) balance: U256,
    pub(crate) storage_ptr: usize,
    pub(crate) code_hash: H256,
}

pub(crate) fn read_state_trie_value(slice: &[U256]) -> AccountTrieRecord {
    AccountTrieRecord {
        nonce: slice[0].as_u64(),
        balance: slice[1],
        storage_ptr: slice[2].as_usize(),
        code_hash: H256::from_uint(&slice[3]),
    }
}

pub(crate) fn read_storage_trie_value(slice: &[U256]) -> U256 {
    slice[0]
}

pub(crate) fn read_trie<V>(
    memory: &MemoryState,
    ptr: usize,
    read_value: fn(&[U256]) -> V,
) -> HashMap<Nibbles, V> {
    let mut res = HashMap::new();
    let empty_nibbles = Nibbles {
        count: 0,
        packed: U512::zero(),
    };
    read_trie_helper::<V>(memory, ptr, read_value, empty_nibbles, &mut res);
    res
}

pub(crate) fn read_trie_helper<V>(
    memory: &MemoryState,
    ptr: usize,
    read_value: fn(&[U256]) -> V,
    prefix: Nibbles,
    res: &mut HashMap<Nibbles, V>,
) {
    let load = |offset| memory.get(MemoryAddress::new(0, Segment::TrieData, offset));
    let load_slice_from = |init_offset| {
        &memory.contexts[0].segments[Segment::TrieData as usize].content[init_offset..]
    };

    let trie_type = PartialTrieType::all()[load(ptr).as_usize()];
    match trie_type {
        PartialTrieType::Empty => {}
        PartialTrieType::Hash => {}
        PartialTrieType::Branch => {
            let ptr_payload = ptr + 1;
            for i in 0u8..16 {
                let child_ptr = load(ptr_payload + i as usize).as_usize();
                read_trie_helper::<V>(memory, child_ptr, read_value, prefix.merge_nibble(i), res);
            }
            let value_ptr = load(ptr_payload + 16).as_usize();
            if value_ptr != 0 {
                res.insert(prefix, read_value(load_slice_from(value_ptr)));
            };
        }
        PartialTrieType::Extension => {
            let count = load(ptr + 1).as_usize();
            let packed = load(ptr + 2);
            let nibbles = Nibbles { count, packed: packed.into() };
            let child_ptr = load(ptr + 3).as_usize();
            read_trie_helper::<V>(
                memory,
                child_ptr,
                read_value,
                prefix.merge_nibbles(&nibbles),
                res,
            );
        }
        PartialTrieType::Leaf => {
            let count = load(ptr + 1).as_usize();
            let packed = load(ptr + 2);
            let nibbles = Nibbles { count, packed: packed.into() };
            let value_ptr = load(ptr + 3).as_usize();
            res.insert(
                prefix.merge_nibbles(&nibbles),
                read_value(load_slice_from(value_ptr)),
            );
        }
    }
}
