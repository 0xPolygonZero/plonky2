use eth_trie_utils::partial_trie::PartialTrie;

pub(crate) enum PartialTrieType {
    Empty = 0,
    Hash = 1,
    Branch = 2,
    Extension = 3,
    Leaf = 4,
}

impl PartialTrieType {
    pub(crate) const COUNT: usize = 5;

    pub(crate) fn of(trie: &PartialTrie) -> Self {
        match trie {
            PartialTrie::Empty => Self::Empty,
            PartialTrie::Hash(_) => Self::Hash,
            PartialTrie::Branch { .. } => Self::Branch,
            PartialTrie::Extension { .. } => Self::Extension,
            PartialTrie::Leaf { .. } => Self::Leaf,
        }
    }

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [
            Self::Empty,
            Self::Hash,
            Self::Branch,
            Self::Extension,
            Self::Leaf,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
        match self {
            Self::Empty => "MPT_NODE_EMPTY",
            Self::Hash => "MPT_NODE_HASH",
            Self::Branch => "MPT_NODE_BRANCH",
            Self::Extension => "MPT_NODE_EXTENSION",
            Self::Leaf => "MPT_NODE_LEAF",
        }
    }
}
