/// These metadata fields contain global VM state, stored in the `Segment::Metadata` segment of the
/// kernel's context (which is zero).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum GlobalMetadata {
    /// The largest context ID that has been used so far in this execution. Tracking this allows us
    /// give each new context a unique ID, so that its memory will be zero-initialized.
    LargestContext = 0,
}

impl GlobalMetadata {
    pub(crate) const COUNT: usize = 1;

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [Self::LargestContext]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
        match self {
            GlobalMetadata::LargestContext => "GLOBAL_METADATA_LARGEST_CONTEXT",
        }
    }
}
