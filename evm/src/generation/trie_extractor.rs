use std::collections::HashMap;

use eth_trie_utils::partial_trie::Nibbles;
use ethereum_types::{BigEndianHash, H256, U256};
use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;

use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::generation::mpt::AccountRlp;
use crate::memory::segments::Segment;
use crate::witness::memory::{MemoryAddress, MemoryState};

pub(crate) fn read_state_trie_value(slice: &[U256]) -> AccountRlp {
    AccountRlp {
        nonce: slice[0],
        balance: slice[1],
        storage_root: H256::from_uint(&slice[2]),
        code_hash: H256::from_uint(&slice[3]),
    }
}

pub(crate) fn read_trie<F, V, const D: usize>(
    memory: &MemoryState,
    ptr: usize,
    read_value: fn(&[U256]) -> V,
) -> HashMap<Nibbles, V>
where
    F: RichField + Extendable<D>,
{
    let mut res = HashMap::new();
    let empty_nibbles = Nibbles {
        count: 0,
        packed: U256::zero(),
    };
    read_trie_helper::<F, V, D>(memory, ptr, read_value, empty_nibbles, &mut res);
    res
}

pub(crate) fn read_trie_helper<F, V, const D: usize>(
    memory: &MemoryState,
    ptr: usize,
    read_value: fn(&[U256]) -> V,
    prefix: Nibbles,
    res: &mut HashMap<Nibbles, V>,
) where
    F: RichField + Extendable<D>,
{
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
                read_trie_helper::<F, V, D>(
                    memory,
                    child_ptr,
                    read_value,
                    prefix.merge_nibble(i),
                    res,
                );
            }
            let value_ptr = load(ptr_payload + 16).as_usize();
            if value_ptr != 0 {
                res.insert(prefix, read_value(load_slice_from(value_ptr)));
            };
        }
        PartialTrieType::Extension => {
            let count = load(ptr + 1).as_usize();
            let packed = load(ptr + 2);
            let nibbles = Nibbles { count, packed };
            let child_ptr = load(ptr + 3).as_usize();
            read_trie_helper::<F, V, D>(
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
            let nibbles = Nibbles { count, packed };
            let value_ptr = load(ptr + 3).as_usize();
            res.insert(
                prefix.merge_nibbles(&nibbles),
                read_value(load_slice_from(value_ptr)),
            );
        }
    }
}
