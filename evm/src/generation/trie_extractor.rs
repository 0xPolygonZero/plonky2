use eth_trie_utils::partial_trie::Nibbles;
use ethereum_types::{BigEndianHash, H256, U256};
use plonky2::field::extension::Extendable;
use plonky2::hash::hash_types::RichField;

use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::generation::mpt::AccountRlp;
use crate::generation::typed_trie::TypedPartialTrie;
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

/// F is the trie's value type, read_value reads a value from a slice of trie memory.
pub(crate) fn read_trie<F, V, const D: usize>(
    memory: &MemoryState,
    ptr: usize,
    read_value: fn(&[U256]) -> V,
) -> TypedPartialTrie<V>
where
    F: RichField + Extendable<D>,
{
    let load = |offset| memory.get(MemoryAddress::new(0, Segment::TrieData, offset));
    let load_slice_from = |init_offset| {
        &memory.contexts[0].segments[Segment::TrieData as usize].content[init_offset..]
    };

    let trie_type = PartialTrieType::all()[load(ptr).as_usize()];
    match trie_type {
        PartialTrieType::Empty => TypedPartialTrie::Empty,
        PartialTrieType::Hash => TypedPartialTrie::Hash(H256::from_uint(&load(ptr + 1))),
        PartialTrieType::Branch => {
            let ptr_payload = ptr + 1;
            let children = std::array::from_fn(|i| {
                read_trie::<F, V, D>(memory, load(ptr_payload + i).as_usize(), read_value).into()
            });
            let value_ptr = load(ptr_payload + 16).as_usize();
            let value = if value_ptr == 0 {
                None
            } else {
                Some(read_value(load_slice_from(value_ptr)))
            };
            TypedPartialTrie::Branch { children, value }
        }
        PartialTrieType::Extension => {
            let count = load(ptr + 1).as_usize();
            let packed = load(ptr + 2);
            let nibbles = Nibbles { count, packed };
            let child_ptr = load(ptr + 3).as_usize();
            let child = read_trie::<F, V, D>(memory, child_ptr, read_value).into();
            TypedPartialTrie::Extension { nibbles, child }
        }
        PartialTrieType::Leaf => {
            let count = load(ptr + 1).as_usize();
            let packed = load(ptr + 2);
            let nibbles = Nibbles { count, packed };
            let value_ptr = load(ptr + 3).as_usize();
            let value = read_value(load_slice_from(value_ptr));
            TypedPartialTrie::Leaf { nibbles, value }
        }
    }
}
