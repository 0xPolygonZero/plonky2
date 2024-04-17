use std::ops::Deref;

use eth_trie_utils::partial_trie::HashedPartialTrie;

use crate::Node;

#[derive(Copy, Clone, Debug)]
pub(crate) enum PartialTrieType {
    Empty = 0,
    Hash = 1,
    Branch = 2,
    Extension = 3,
    Leaf = 4,
}

impl PartialTrieType {
    pub(crate) const COUNT: usize = 5;

    pub(crate) fn of(trie: &HashedPartialTrie) -> Self {
        match trie.deref() {
            Node::Empty => Self::Empty,
            Node::Hash(_) => Self::Hash,
            Node::Branch { .. } => Self::Branch,
            Node::Extension { .. } => Self::Extension,
            Node::Leaf { .. } => Self::Leaf,
        }
    }

    pub(crate) const fn all() -> [Self; Self::COUNT] {
        [
            Self::Empty,
            Self::Hash,
            Self::Branch,
            Self::Extension,
            Self::Leaf,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) const fn var_name(&self) -> &'static str {
        match self {
            Self::Empty => "MPT_NODE_EMPTY",
            Self::Hash => "MPT_NODE_HASH",
            Self::Branch => "MPT_NODE_BRANCH",
            Self::Extension => "MPT_NODE_EXTENSION",
            Self::Leaf => "MPT_NODE_LEAF",
        }
    }
}
