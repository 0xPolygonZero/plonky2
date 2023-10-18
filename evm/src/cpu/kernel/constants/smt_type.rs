use core::todo;
use std::ops::Deref;

use eth_trie_utils::partial_trie::HashedPartialTrie;

use crate::Node;

#[derive(Copy, Clone, Debug)]
pub(crate) enum PartialSmtType {
    Hash = 0,
    Internal = 1,
    Leaf = 2,
}

impl PartialSmtType {
    pub(crate) const COUNT: usize = 3;

    pub(crate) fn of(trie: &HashedPartialTrie) -> Self {
        todo!()
        // match trie.deref() {
        //     Node::Empty => Self::Empty,
        //     Node::Hash(_) => Self::Hash,
        //     Node::Branch { .. } => Self::Branch,
        //     Node::Extension { .. } => Self::Extension,
        //     Node::Leaf { .. } => Self::Leaf,
        // }
    }

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [Self::Hash, Self::Internal, Self::Leaf]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
        match self {
            Self::Hash => "SMT_NODE_HASH",
            Self::Internal => "SMT_NODE_BRANCH",
            Self::Leaf => "SMT_NODE_LEAF",
        }
    }
}
