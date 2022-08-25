/// These metadata fields contain global VM state, stored in the `Segment::Metadata` segment of the
/// kernel's context (which is zero).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum GlobalMetadata {
    /// The largest context ID that has been used so far in this execution. Tracking this allows us
    /// give each new context a unique ID, so that its memory will be zero-initialized.
    LargestContext = 0,
    /// The address of the sender of the transaction.
    Origin = 1,
    /// The size of active memory, in bytes.
    MemorySize = 2,
    /// The size of the `TrieData` segment, in bytes. In other words, the next address available for
    /// appending additional trie data.
    TrieDataSize = 3,
    /// A pointer to the root of the state trie within the `TrieData` buffer.
    StateTrieRoot = 4,
    /// A pointer to the root of the transaction trie within the `TrieData` buffer.
    TransactionTrieRoot = 5,
    /// A pointer to the root of the receipt trie within the `TrieData` buffer.
    ReceiptTrieRoot = 6,
    /// The number of storage tries involved in this transaction. I.e. the number of values in
    /// `StorageTrieAddresses`, `StorageTriePointers` and `StorageTrieCheckpointPointers`.
    NumStorageTries = 7,
}

impl GlobalMetadata {
    pub(crate) const COUNT: usize = 8;

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [
            Self::LargestContext,
            Self::Origin,
            Self::MemorySize,
            Self::TrieDataSize,
            Self::StateTrieRoot,
            Self::TransactionTrieRoot,
            Self::ReceiptTrieRoot,
            Self::NumStorageTries,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
        match self {
            GlobalMetadata::LargestContext => "GLOBAL_METADATA_LARGEST_CONTEXT",
            GlobalMetadata::Origin => "GLOBAL_METADATA_ORIGIN",
            GlobalMetadata::MemorySize => "GLOBAL_METADATA_MEMORY_SIZE",
            GlobalMetadata::TrieDataSize => "GLOBAL_METADATA_TRIE_DATA_SIZE",
            GlobalMetadata::StateTrieRoot => "GLOBAL_METADATA_STATE_TRIE_ROOT",
            GlobalMetadata::TransactionTrieRoot => "GLOBAL_METADATA_TXN_TRIE_ROOT",
            GlobalMetadata::ReceiptTrieRoot => "GLOBAL_METADATA_RECEIPT_TRIE_ROOT",
            GlobalMetadata::NumStorageTries => "GLOBAL_METADATA_NUM_STORAGE_TRIES",
        }
    }
}
