#[derive(Copy, Clone, Debug)]
pub(crate) enum PartialSmtType {
    Hash = 0,
    Internal = 1,
    Leaf = 2,
}

impl PartialSmtType {
    pub(crate) const COUNT: usize = 3;

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [Self::Hash, Self::Internal, Self::Leaf]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
        match self {
            Self::Hash => "SMT_NODE_HASH",
            Self::Internal => "SMT_NODE_INTERNAL",
            Self::Leaf => "SMT_NODE_LEAF",
        }
    }
}
