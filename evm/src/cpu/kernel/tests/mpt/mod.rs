use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};

mod hash;
mod hex_prefix;
mod load;
mod read;

/// A `PartialTrie` where an extension node leads to a leaf node containing an account.
pub(crate) fn extension_to_leaf(value: Vec<u8>) -> PartialTrie {
    PartialTrie::Extension {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        child: Box::new(PartialTrie::Leaf {
            nibbles: Nibbles {
                count: 3,
                packed: 0xDEF.into(),
            },
            value,
        }),
    }
}
