use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::U256;

use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::generation::TrieInputs;

pub(crate) fn all_mpt_prover_inputs_reversed(trie_inputs: &TrieInputs) -> Vec<U256> {
    let mut inputs = all_mpt_prover_inputs(trie_inputs);
    inputs.reverse();
    inputs
}

/// Generate prover inputs for the initial MPT data, in the format expected by `mpt/load.asm`.
pub(crate) fn all_mpt_prover_inputs(trie_inputs: &TrieInputs) -> Vec<U256> {
    let mut prover_inputs = vec![];

    mpt_prover_inputs(&trie_inputs.state_trie, &mut prover_inputs, &|rlp| {
        rlp::decode_list(rlp)
    });

    mpt_prover_inputs(&trie_inputs.transactions_trie, &mut prover_inputs, &|rlp| {
        rlp::decode_list(rlp)
    });

    mpt_prover_inputs(&trie_inputs.receipts_trie, &mut prover_inputs, &|_rlp| {
        // TODO: Decode receipt RLP.
        vec![]
    });

    prover_inputs.push(trie_inputs.storage_tries.len().into());
    for (addr, storage_trie) in &trie_inputs.storage_tries {
        prover_inputs.push(addr.0.as_ref().into());
        mpt_prover_inputs(storage_trie, &mut prover_inputs, &|leaf_be| {
            vec![U256::from_big_endian(leaf_be)]
        });
    }

    prover_inputs
}

/// Given a trie, generate the prover input data for that trie. In essence, this serializes a trie
/// into a `U256` array, in a simple format which the kernel understands. For example, a leaf node
/// is serialized as `(TYPE_LEAF, key, value)`, where key is a `(nibbles, depth)` pair and `value`
/// is a variable-length structure which depends on which trie we're dealing with.
pub(crate) fn mpt_prover_inputs<F>(
    trie: &PartialTrie,
    prover_inputs: &mut Vec<U256>,
    parse_leaf: &F,
) where
    F: Fn(&[u8]) -> Vec<U256>,
{
    prover_inputs.push((PartialTrieType::of(trie) as u32).into());
    match trie {
        PartialTrie::Empty => {}
        PartialTrie::Hash(h) => prover_inputs.push(*h),
        PartialTrie::Branch { children, value } => {
            for child in children {
                mpt_prover_inputs(child, prover_inputs, parse_leaf);
            }
            let leaf = parse_leaf(value);
            prover_inputs.push(leaf.len().into());
            prover_inputs.extend(leaf);
        }
        PartialTrie::Extension { nibbles, child } => {
            prover_inputs.push(nibbles.count.into());
            prover_inputs.push(nibbles.packed);
            mpt_prover_inputs(child, prover_inputs, parse_leaf);
        }
        PartialTrie::Leaf { nibbles, value } => {
            prover_inputs.push(nibbles.count.into());
            prover_inputs.push(nibbles.packed);
            let leaf = parse_leaf(value);
            prover_inputs.push(leaf.len().into());
            prover_inputs.extend(leaf);
        }
    }
}
