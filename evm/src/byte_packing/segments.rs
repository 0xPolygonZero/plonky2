#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub enum Segment {
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
    /// Another segment for general purpose kernel use.
    KernelGeneral2 = 8,
    /// Segment to hold account code for opcodes like `CODESIZE, CODECOPY,...`.
    KernelAccountCode = 9,
    /// Contains normalized transaction fields; see `NormalizedTxnField`.
    TxnFields = 10,
    /// Contains the data field of a transaction.
    TxnData = 11,
    /// A buffer used to hold raw RLP data.
    RlpRaw = 12,
    /// Contains all trie data. It is owned by the kernel, so it only lives on context 0.
    TrieData = 13,
    /// A buffer used to store the encodings of a branch node's children.
    TrieEncodedChild = 14,
    /// A buffer used to store the lengths of the encodings of a branch node's children.
    TrieEncodedChildLen = 15,
    /// A table of values 2^i for i=0..255 for use with shift
    /// instructions; initialised by `kernel/asm/shift.asm::init_shift_table()`.
    ShiftTable = 16,
    JumpdestBits = 17,
    EcdsaTable = 18,
    BnWnafA = 19,
    BnWnafB = 20,
    BnTableQ = 21,
    BnPairing = 22,
    /// List of addresses that have been accessed in the current transaction.
    AccessedAddresses = 23,
    /// List of storage keys that have been accessed in the current transaction.
    AccessedStorageKeys = 24,
    /// List of addresses that have called SELFDESTRUCT in the current transaction.
    SelfDestructList = 25,
    /// Contains the bloom filter of a transaction.
    TxnBloom = 26,
    /// Contains the bloom filter of a block.
    BlockBloom = 27,
    /// List of log pointers pointing to the LogsData segment.
    Logs = 28,
    LogsData = 29,
    /// Journal of state changes. List of pointers to `JournalData`. Length in `GlobalMetadata`.
    Journal = 30,
    JournalData = 31,
    JournalCheckpoints = 32,
    /// List of addresses that have been touched in the current transaction.
    TouchedAddresses = 33,
    /// List of checkpoints for the current context. Length in `ContextMetadata`.
    ContextCheckpoints = 34,
}

impl Segment {
    pub(crate) const COUNT: usize = 35;

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
            Self::KernelGeneral2,
            Self::KernelAccountCode,
            Self::TxnFields,
            Self::TxnData,
            Self::RlpRaw,
            Self::TrieData,
            Self::TrieEncodedChild,
            Self::TrieEncodedChildLen,
            Self::ShiftTable,
            Self::JumpdestBits,
            Self::EcdsaTable,
            Self::BnWnafA,
            Self::BnWnafB,
            Self::BnTableQ,
            Self::BnPairing,
            Self::AccessedAddresses,
            Self::AccessedStorageKeys,
            Self::SelfDestructList,
            Self::TxnBloom,
            Self::BlockBloom,
            Self::Logs,
            Self::LogsData,
            Self::Journal,
            Self::JournalData,
            Self::JournalCheckpoints,
            Self::TouchedAddresses,
            Self::ContextCheckpoints,
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
            Segment::KernelGeneral2 => "SEGMENT_KERNEL_GENERAL_2",
            Segment::KernelAccountCode => "SEGMENT_KERNEL_ACCOUNT_CODE",
            Segment::TxnFields => "SEGMENT_NORMALIZED_TXN",
            Segment::TxnData => "SEGMENT_TXN_DATA",
            Segment::RlpRaw => "SEGMENT_RLP_RAW",
            Segment::TrieData => "SEGMENT_TRIE_DATA",
            Segment::TrieEncodedChild => "SEGMENT_TRIE_ENCODED_CHILD",
            Segment::TrieEncodedChildLen => "SEGMENT_TRIE_ENCODED_CHILD_LEN",
            Segment::ShiftTable => "SEGMENT_SHIFT_TABLE",
            Segment::JumpdestBits => "SEGMENT_JUMPDEST_BITS",
            Segment::EcdsaTable => "SEGMENT_KERNEL_ECDSA_TABLE",
            Segment::BnWnafA => "SEGMENT_KERNEL_BN_WNAF_A",
            Segment::BnWnafB => "SEGMENT_KERNEL_BN_WNAF_B",
            Segment::BnTableQ => "SEGMENT_KERNEL_BN_TABLE_Q",
            Segment::BnPairing => "SEGMENT_KERNEL_BN_PAIRING",
            Segment::AccessedAddresses => "SEGMENT_ACCESSED_ADDRESSES",
            Segment::AccessedStorageKeys => "SEGMENT_ACCESSED_STORAGE_KEYS",
            Segment::SelfDestructList => "SEGMENT_SELFDESTRUCT_LIST",
            Segment::TxnBloom => "SEGMENT_TXN_BLOOM",
            Segment::BlockBloom => "SEGMENT_BLOCK_BLOOM",
            Segment::Logs => "SEGMENT_LOGS",
            Segment::LogsData => "SEGMENT_LOGS_DATA",
            Segment::Journal => "SEGMENT_JOURNAL",
            Segment::JournalData => "SEGMENT_JOURNAL_DATA",
            Segment::JournalCheckpoints => "SEGMENT_JOURNAL_CHECKPOINTS",
            Segment::TouchedAddresses => "SEGMENT_TOUCHED_ADDRESSES",
            Segment::ContextCheckpoints => "SEGMENT_CONTEXT_CHECKPOINTS",
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
            Segment::KernelGeneral2 => 256,
            Segment::KernelAccountCode => 8,
            Segment::TxnFields => 256,
            Segment::TxnData => 8,
            Segment::RlpRaw => 8,
            Segment::TrieData => 256,
            Segment::TrieEncodedChild => 256,
            Segment::TrieEncodedChildLen => 6,
            Segment::ShiftTable => 256,
            Segment::JumpdestBits => 1,
            Segment::EcdsaTable => 256,
            Segment::BnWnafA => 8,
            Segment::BnWnafB => 8,
            Segment::BnTableQ => 256,
            Segment::BnPairing => 256,
            Segment::AccessedAddresses => 256,
            Segment::AccessedStorageKeys => 256,
            Segment::SelfDestructList => 256,
            Segment::TxnBloom => 8,
            Segment::BlockBloom => 8,
            Segment::Logs => 256,
            Segment::LogsData => 256,
            Segment::Journal => 256,
            Segment::JournalData => 256,
            Segment::JournalCheckpoints => 256,
            Segment::TouchedAddresses => 256,
            Segment::ContextCheckpoints => 256,
        }
    }
}
