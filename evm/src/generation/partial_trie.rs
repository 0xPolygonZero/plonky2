use ethereum_types::U256;

#[derive(Clone, Debug)]
/// A partial trie, or a sub-trie thereof. This mimics the structure of an Ethereum trie, except
/// with an additional `Hash` node type, representing a node whose data is not needed to process
/// our transaction.
pub enum PartialTrie {
    /// An empty trie.
    Empty,
    /// The digest of trie whose data does not need to be stored.
    Hash(U256),
    /// A branch node, which consists of 16 children and an optional value.
    Branch {
        children: [Box<PartialTrie>; 16],
        value: Option<U256>,
    },
    /// An extension node, which consists of a list of nibbles and a single child.
    Extension {
        nibbles: Nibbles,
        child: Box<PartialTrie>,
    },
    /// A leaf node, which consists of a list of nibbles and a value.
    Leaf { nibbles: Nibbles, value: Vec<u8> },
}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// A sequence of nibbles.
pub struct Nibbles {
    /// The number of nibbles in this sequence.
    pub count: usize,
    /// A packed encoding of these nibbles. Only the first (least significant) `4 * count` bits are
    /// used. The rest are unused and should be zero.
    pub packed: U256,
}
