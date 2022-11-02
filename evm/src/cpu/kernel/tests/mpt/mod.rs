use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};
use ethereum_types::{BigEndianHash, H256, U256};

use crate::generation::mpt::AccountRlp;

mod hash;
mod hex_prefix;
mod insert;
mod load;
mod read;

pub(crate) fn nibbles_64<T: Into<U256>>(v: T) -> Nibbles {
    let packed = v.into();
    Nibbles { count: 64, packed }
}

pub(crate) fn nibbles_count<T: Into<U256>>(v: T, count: usize) -> Nibbles {
    let packed = v.into();
    Nibbles { count, packed }
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
pub(crate) fn extension_to_leaf(value: Vec<u8>) -> PartialTrie {
    PartialTrie::Extension {
        nibbles: 0xABC_u64.into(),
        child: PartialTrie::Leaf {
            nibbles: Nibbles {
                count: 3,
                packed: 0xDEF.into(),
            },
            value,
        }
        .into(),
    }
}
