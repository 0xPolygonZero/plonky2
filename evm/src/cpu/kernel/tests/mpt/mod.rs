use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::HashedPartialTrie;
use ethereum_types::{BigEndianHash, H256, U256};

use crate::generation::mpt::AccountRlp;
use crate::Node;

mod delete;
mod hash;
mod hex_prefix;
mod insert;
mod load;
mod read;

pub(crate) fn nibbles_64<T: Into<U256>>(v: T) -> Nibbles {
    let packed: U256 = v.into();
    Nibbles {
        count: 64,
        packed: packed.into(),
    }
}

pub(crate) fn nibbles_count<T: Into<U256>>(v: T, count: usize) -> Nibbles {
    let packed: U256 = v.into();
    Nibbles {
        count,
        packed: packed.into(),
    }
}

pub(crate) fn test_account_1() -> AccountRlp {
    AccountRlp {
        nonce: U256::from(1111),
        balance: U256::from(2222),
        storage_root: H256::from_uint(&U256::from(3333)),
        code_hash: H256::from_uint(&U256::from(4444)),
    }
}

pub(crate) fn test_account_1_rlp() -> Vec<u8> {
    rlp::encode(&test_account_1()).to_vec()
}

pub(crate) fn test_account_2() -> AccountRlp {
    AccountRlp {
        nonce: U256::from(5555),
        balance: U256::from(6666),
        storage_root: H256::from_uint(&U256::from(7777)),
        code_hash: H256::from_uint(&U256::from(8888)),
    }
}

pub(crate) fn test_account_2_rlp() -> Vec<u8> {
    rlp::encode(&test_account_2()).to_vec()
}

/// A `PartialTrie` where an extension node leads to a leaf node containing an account.
pub(crate) fn extension_to_leaf(value: Vec<u8>) -> HashedPartialTrie {
    Node::Extension {
        nibbles: 0xABC_u64.into(),
        child: Node::Leaf {
            nibbles: Nibbles {
                count: 3,
                packed: 0xDEF.into(),
            },
            value,
        }
        .into(),
    }
    .into()
}
