#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum Segment {
    /// Contains EVM bytecode.
    Code = 0,
    /// The program stack.
    Stack = 1,
    /// Main memory, owned by the contract code.
    MainMemory = 2,
    /// Data passed to the current context by its caller.
    Calldata = 3,
    /// Data returned to the current context by its latest callee.
    Returndata = 4,
    /// A segment which contains a few fixed-size metadata fields, such as the caller's context, or the
    /// size of `CALLDATA` and `RETURNDATA`.
    GlobalMetadata = 5,
    ContextMetadata = 6,
    /// General purpose kernel memory, used by various kernel functions.
    /// In general, calling a helper function can result in this memory being clobbered.
    KernelGeneral = 7,
    /// Contains normalized transaction fields; see `NormalizedTxnField`.
    TxnFields = 8,
    /// Contains the data field of a transaction.
    TxnData = 9,
    /// A buffer used to hold raw RLP data.
    RlpRaw = 10,
    /// Contains all trie data. Tries are stored as immutable, copy-on-write trees, so this is an
    /// append-only buffer. It is owned by the kernel, so it only lives on context 0.
    TrieData = 11,
    /// The account address associated with the `i`th storage trie. Only lives on context 0.
    StorageTrieAddresses = 12,
    /// A pointer to the `i`th storage trie within the `TrieData` buffer. Only lives on context 0.
    StorageTriePointers = 13,
    /// Like `StorageTriePointers`, except that these pointers correspond to the version of each
    /// trie at the creation of a given context. This lets us easily revert a context by replacing
    /// `StorageTriePointers` with `StorageTrieCheckpointPointers`.
    /// See also `StateTrieCheckpointPointer`.
    StorageTrieCheckpointPointers = 14,
}

impl Segment {
    pub(crate) const COUNT: usize = 15;

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [
            Self::Code,
            Self::Stack,
            Self::MainMemory,
            Self::Calldata,
            Self::Returndata,
            Self::GlobalMetadata,
            Self::ContextMetadata,
            Self::KernelGeneral,
            Self::TxnFields,
            Self::TxnData,
            Self::RlpRaw,
            Self::TrieData,
            Self::StorageTrieAddresses,
            Self::StorageTriePointers,
            Self::StorageTrieCheckpointPointers,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
        match self {
            Segment::Code => "SEGMENT_CODE",
            Segment::Stack => "SEGMENT_STACK",
            Segment::MainMemory => "SEGMENT_MAIN_MEMORY",
            Segment::Calldata => "SEGMENT_CALLDATA",
            Segment::Returndata => "SEGMENT_RETURNDATA",
            Segment::GlobalMetadata => "SEGMENT_GLOBAL_METADATA",
            Segment::ContextMetadata => "SEGMENT_CONTEXT_METADATA",
            Segment::KernelGeneral => "SEGMENT_KERNEL_GENERAL",
            Segment::TxnFields => "SEGMENT_NORMALIZED_TXN",
            Segment::TxnData => "SEGMENT_TXN_DATA",
            Segment::RlpRaw => "SEGMENT_RLP_RAW",
            Segment::TrieData => "SEGMENT_TRIE_DATA",
            Segment::StorageTrieAddresses => "SEGMENT_STORAGE_TRIE_ADDRS",
            Segment::StorageTriePointers => "SEGMENT_STORAGE_TRIE_PTRS",
            Segment::StorageTrieCheckpointPointers => "SEGMENT_STORAGE_TRIE_CHECKPOINT_PTRS",
        }
    }

    #[allow(dead_code)]
    pub(crate) fn bit_range(&self) -> usize {
        match self {
            Segment::Code => 8,
            Segment::Stack => 256,
            Segment::MainMemory => 8,
            Segment::Calldata => 8,
            Segment::Returndata => 8,
            Segment::GlobalMetadata => 256,
            Segment::ContextMetadata => 256,
            Segment::KernelGeneral => 256,
            Segment::TxnFields => 256,
            Segment::TxnData => 256,
            Segment::RlpRaw => 8,
            Segment::TrieData => 256,
            Segment::StorageTrieAddresses => 160,
            Segment::StorageTriePointers => 32,
            Segment::StorageTrieCheckpointPointers => 32,
        }
    }
}
