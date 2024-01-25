use ethereum_types::U256;
use num::traits::AsPrimitive;

pub(crate) const SEGMENT_SCALING_FACTOR: usize = 32;

/// This contains all the existing memory segments. The values in the enum are shifted by 32 bits
/// to allow for convenient address components (context / segment / virtual) bundling in the kernel.
#[allow(dead_code)]
#[allow(clippy::enum_clike_unportable_variant)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum Segment {
    /// Contains EVM bytecode.
    // The Kernel has optimizations relying on the Code segment being 0.
    // This shouldn't be changed!
    Code = 0,
    /// The program stack.
    Stack = 1 << SEGMENT_SCALING_FACTOR,
    /// Main memory, owned by the contract code.
    MainMemory = 2 << SEGMENT_SCALING_FACTOR,
    /// Data passed to the current context by its caller.
    Calldata = 3 << SEGMENT_SCALING_FACTOR,
    /// Data returned to the current context by its latest callee.
    Returndata = 4 << SEGMENT_SCALING_FACTOR,
    /// A segment which contains a few fixed-size metadata fields, such as the caller's context, or the
    /// size of `CALLDATA` and `RETURNDATA`.
    GlobalMetadata = 5 << SEGMENT_SCALING_FACTOR,
    ContextMetadata = 6 << SEGMENT_SCALING_FACTOR,
    /// General purpose kernel memory, used by various kernel functions.
    /// In general, calling a helper function can result in this memory being clobbered.
    KernelGeneral = 7 << SEGMENT_SCALING_FACTOR,
    /// Another segment for general purpose kernel use.
    KernelGeneral2 = 8 << SEGMENT_SCALING_FACTOR,
    /// Segment to hold account code for opcodes like `CODESIZE, CODECOPY,...`.
    KernelAccountCode = 9 << SEGMENT_SCALING_FACTOR,
    /// Contains normalized transaction fields; see `NormalizedTxnField`.
    TxnFields = 10 << SEGMENT_SCALING_FACTOR,
    /// Contains the data field of a transaction.
    TxnData = 11 << SEGMENT_SCALING_FACTOR,
    /// A buffer used to hold raw RLP data.
    RlpRaw = 12 << SEGMENT_SCALING_FACTOR,
    /// Contains all trie data. It is owned by the kernel, so it only lives on context 0.
    TrieData = 13 << SEGMENT_SCALING_FACTOR,
    ShiftTable = 14 << SEGMENT_SCALING_FACTOR,
    JumpdestBits = 15 << SEGMENT_SCALING_FACTOR,
    EcdsaTable = 16 << SEGMENT_SCALING_FACTOR,
    BnWnafA = 17 << SEGMENT_SCALING_FACTOR,
    BnWnafB = 18 << SEGMENT_SCALING_FACTOR,
    BnTableQ = 19 << SEGMENT_SCALING_FACTOR,
    BnPairing = 20 << SEGMENT_SCALING_FACTOR,
    /// List of addresses that have been accessed in the current transaction.
    AccessedAddresses = 21 << SEGMENT_SCALING_FACTOR,
    /// List of storage keys that have been accessed in the current transaction.
    AccessedStorageKeys = 22 << SEGMENT_SCALING_FACTOR,
    /// List of addresses that have called SELFDESTRUCT in the current transaction.
    SelfDestructList = 23 << SEGMENT_SCALING_FACTOR,
    /// Contains the bloom filter of a transaction.
    TxnBloom = 24 << SEGMENT_SCALING_FACTOR,
    /// Contains the bloom filter present in the block header.
    GlobalBlockBloom = 25 << SEGMENT_SCALING_FACTOR,
    /// List of log pointers pointing to the LogsData segment.
    Logs = 26 << SEGMENT_SCALING_FACTOR,
    LogsData = 27 << SEGMENT_SCALING_FACTOR,
    /// Journal of state changes. List of pointers to `JournalData`. Length in `GlobalMetadata`.
    Journal = 28 << SEGMENT_SCALING_FACTOR,
    JournalData = 29 << SEGMENT_SCALING_FACTOR,
    JournalCheckpoints = 30 << SEGMENT_SCALING_FACTOR,
    /// List of addresses that have been touched in the current transaction.
    TouchedAddresses = 31 << SEGMENT_SCALING_FACTOR,
    /// List of checkpoints for the current context. Length in `ContextMetadata`.
    ContextCheckpoints = 32 << SEGMENT_SCALING_FACTOR,
    /// List of 256 previous block hashes.
    BlockHashes = 33 << SEGMENT_SCALING_FACTOR,
}

impl Segment {
    pub(crate) const COUNT: usize = 34;

    /// Unscales this segment by `SEGMENT_SCALING_FACTOR`.
    pub(crate) const fn unscale(&self) -> usize {
        *self as usize >> SEGMENT_SCALING_FACTOR
    }

    pub(crate) const fn all() -> [Self; Self::COUNT] {
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
            Self::GlobalBlockBloom,
            Self::Logs,
            Self::LogsData,
            Self::Journal,
            Self::JournalData,
            Self::JournalCheckpoints,
            Self::TouchedAddresses,
            Self::ContextCheckpoints,
            Self::BlockHashes,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) const fn var_name(&self) -> &'static str {
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
            Segment::ShiftTable => "SEGMENT_SHIFT_TABLE",
            Segment::JumpdestBits => "SEGMENT_JUMPDEST_BITS",
            Segment::EcdsaTable => "SEGMENT_ECDSA_TABLE",
            Segment::BnWnafA => "SEGMENT_BN_WNAF_A",
            Segment::BnWnafB => "SEGMENT_BN_WNAF_B",
            Segment::BnTableQ => "SEGMENT_BN_TABLE_Q",
            Segment::BnPairing => "SEGMENT_BN_PAIRING",
            Segment::AccessedAddresses => "SEGMENT_ACCESSED_ADDRESSES",
            Segment::AccessedStorageKeys => "SEGMENT_ACCESSED_STORAGE_KEYS",
            Segment::SelfDestructList => "SEGMENT_SELFDESTRUCT_LIST",
            Segment::TxnBloom => "SEGMENT_TXN_BLOOM",
            Segment::GlobalBlockBloom => "SEGMENT_GLOBAL_BLOCK_BLOOM",
            Segment::Logs => "SEGMENT_LOGS",
            Segment::LogsData => "SEGMENT_LOGS_DATA",
            Segment::Journal => "SEGMENT_JOURNAL",
            Segment::JournalData => "SEGMENT_JOURNAL_DATA",
            Segment::JournalCheckpoints => "SEGMENT_JOURNAL_CHECKPOINTS",
            Segment::TouchedAddresses => "SEGMENT_TOUCHED_ADDRESSES",
            Segment::ContextCheckpoints => "SEGMENT_CONTEXT_CHECKPOINTS",
            Segment::BlockHashes => "SEGMENT_BLOCK_HASHES",
        }
    }

    pub(crate) const fn bit_range(&self) -> usize {
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
            Segment::GlobalBlockBloom => 256,
            Segment::Logs => 256,
            Segment::LogsData => 256,
            Segment::Journal => 256,
            Segment::JournalData => 256,
            Segment::JournalCheckpoints => 256,
            Segment::TouchedAddresses => 256,
            Segment::ContextCheckpoints => 256,
            Segment::BlockHashes => 256,
        }
    }

    pub(crate) fn constant(&self, virt: usize) -> Option<U256> {
        match self {
            Segment::RlpRaw => {
                if virt == 0xFFFFFFFF {
                    Some(U256::from(0x80))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
