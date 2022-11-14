use std::collections::HashMap;

use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};
use ethereum_types::{BigEndianHash, H256, U256};
use keccak_hash::keccak;
use rlp_derive::{RlpDecodable, RlpEncodable};

use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::generation::TrieInputs;

#[derive(RlpEncodable, RlpDecodable, Debug)]
pub(crate) struct AccountRlp {
    pub(crate) nonce: U256,
    pub(crate) balance: U256,
    pub(crate) storage_root: H256,
    pub(crate) code_hash: H256,
}

pub(crate) fn all_mpt_prover_inputs_reversed(trie_inputs: &TrieInputs) -> Vec<U256> {
    let mut inputs = all_mpt_prover_inputs(trie_inputs);
    inputs.reverse();
    inputs
}

/// Generate prover inputs for the initial MPT data, in the format expected by `mpt/load.asm`.
pub(crate) fn all_mpt_prover_inputs(trie_inputs: &TrieInputs) -> Vec<U256> {
    let mut prover_inputs = vec![];

    let storage_tries_by_state_key = trie_inputs
        .storage_tries
        .iter()
        .map(|(address, storage_trie)| (Nibbles::from(keccak(address)), storage_trie))
        .collect();

    mpt_prover_inputs_state_trie(
        &trie_inputs.state_trie,
        empty_nibbles(),
        &mut prover_inputs,
        &storage_tries_by_state_key,
    );

    mpt_prover_inputs(&trie_inputs.transactions_trie, &mut prover_inputs, &|rlp| {
        rlp::decode_list(rlp)
    });

    mpt_prover_inputs(&trie_inputs.receipts_trie, &mut prover_inputs, &|_rlp| {
        // TODO: Decode receipt RLP.
        vec![]
    });

    prover_inputs
}

/// Given a trie, generate the prover input data for that trie. In essence, this serializes a trie
/// into a `U256` array, in a simple format which the kernel understands. For example, a leaf node
/// is serialized as `(TYPE_LEAF, key, value)`, where key is a `(nibbles, depth)` pair and `value`
/// is a variable-length structure which depends on which trie we're dealing with.
pub(crate) fn mpt_prover_inputs<F>(
    trie: &PartialTrie,
    prover_inputs: &mut Vec<U256>,
    parse_value: &F,
) where
    F: Fn(&[u8]) -> Vec<U256>,
{
    prover_inputs.push((PartialTrieType::of(trie) as u32).into());
    match trie {
        PartialTrie::Empty => {}
        PartialTrie::Hash(h) => prover_inputs.push(U256::from_big_endian(h.as_bytes())),
        PartialTrie::Branch { children, value } => {
            if value.is_empty() {
                prover_inputs.push(U256::zero()); // value_present = 0
            } else {
                let parsed_value = parse_value(value);
                prover_inputs.push(U256::one()); // value_present = 1
                prover_inputs.extend(parsed_value);
            }
            for child in children {
                mpt_prover_inputs(child, prover_inputs, parse_value);
            }
        }
        PartialTrie::Extension { nibbles, child } => {
            prover_inputs.push(nibbles.count.into());
            prover_inputs.push(nibbles.packed);
            mpt_prover_inputs(child, prover_inputs, parse_value);
        }
        PartialTrie::Leaf { nibbles, value } => {
            prover_inputs.push(nibbles.count.into());
            prover_inputs.push(nibbles.packed);
            let leaf = parse_value(value);
            prover_inputs.extend(leaf);
        }
    }
}

/// Like `mpt_prover_inputs`, but for the state trie, which is a bit unique since each value
/// leads to a storage trie which we recursively traverse.
pub(crate) fn mpt_prover_inputs_state_trie(
    trie: &PartialTrie,
    key: Nibbles,
    prover_inputs: &mut Vec<U256>,
    storage_tries_by_state_key: &HashMap<Nibbles, &PartialTrie>,
) {
    prover_inputs.push((PartialTrieType::of(trie) as u32).into());
    match trie {
        PartialTrie::Empty => {}
        PartialTrie::Hash(h) => prover_inputs.push(U256::from_big_endian(h.as_bytes())),
        PartialTrie::Branch { children, value } => {
            assert!(value.is_empty(), "State trie should not have branch values");
            prover_inputs.push(U256::zero()); // value_present = 0

            for (i, child) in children.iter().enumerate() {
                let extended_key = key.merge_nibbles(&Nibbles {
                    count: 1,
                    packed: i.into(),
                });
                mpt_prover_inputs_state_trie(
                    child,
                    extended_key,
                    prover_inputs,
                    storage_tries_by_state_key,
                );
            }
        }
        PartialTrie::Extension { nibbles, child } => {
            prover_inputs.push(nibbles.count.into());
            prover_inputs.push(nibbles.packed);
            let extended_key = key.merge_nibbles(nibbles);
            mpt_prover_inputs_state_trie(
                child,
                extended_key,
                prover_inputs,
                storage_tries_by_state_key,
            );
        }
        PartialTrie::Leaf { nibbles, value } => {
            let account: AccountRlp = rlp::decode(value).expect("Decoding failed");
            let AccountRlp {
                nonce,
                balance,
                storage_root,
                code_hash,
            } = account;

            let storage_hash_only = PartialTrie::Hash(storage_root);
            let storage_trie: &PartialTrie = storage_tries_by_state_key
                .get(&key)
                .copied()
                .unwrap_or(&storage_hash_only);

            assert_eq!(storage_trie.calc_hash(), storage_root,
                       "In TrieInputs, an account's storage_root didn't match the associated storage trie hash");

            prover_inputs.push(nibbles.count.into());
            prover_inputs.push(nibbles.packed);
            prover_inputs.push(nonce);
            prover_inputs.push(balance);
            mpt_prover_inputs(storage_trie, prover_inputs, &parse_storage_value);
            prover_inputs.push(code_hash.into_uint());
        }
    }
}

fn parse_storage_value(value_rlp: &[u8]) -> Vec<U256> {
    let value: U256 = rlp::decode(value_rlp).expect("Decoding failed");
    vec![value]
}

fn empty_nibbles() -> Nibbles {
    Nibbles {
        count: 0,
        packed: U256::zero(),
    }
}
