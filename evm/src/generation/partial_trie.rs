use ethereum_types::U256;

/// A partial trie, or a sub-trie thereof.
pub enum PartialTrie {
    /// An empty trie.
    Empty,
    /// The digest of trie whose data does not need to be stored.
    Hash(U256),
    /// A branch node, which consists of 16 children and an optional value.
    Branch([Box<PartialTrie>; 16], Option<U256>),
    /// An extension node, which consists of a list of nibbles and a single child.
    Extension(Nibbles, Box<PartialTrie>),
    /// A leaf node, which consists of a list of nibbles and a value.
    Leaf(Nibbles, Vec<u8>),
}

/// A sequence of nibbles.
pub struct Nibbles {
    /// The number of nibbles in this sequence.
    pub count: usize,
    /// A packed encoding of these nibbles. Only the first (least significant) `4 * count` bits are
    /// used. The rest are unused and should be zero.
    pub packed: U256,
}
