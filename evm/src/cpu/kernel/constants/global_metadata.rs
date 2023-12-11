use crate::memory::segments::Segment;

/// These metadata fields contain global VM state, stored in the `Segment::Metadata` segment of the
/// kernel's context (which is zero).
///
/// Each value is directly scaled by the corresponding `Segment::GlobalMetadata` value for faster
/// memory access in the kernel.
#[repr(usize)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum GlobalMetadata {
    /// The largest context ID that has been used so far in this execution. Tracking this allows us
    /// give each new context a unique ID, so that its memory will be zero-initialized.
    LargestContext = Segment::GlobalMetadata as usize,
    /// The size of active memory, in bytes.
    MemorySize = Segment::GlobalMetadata as usize + 1,
    /// The size of the `TrieData` segment, in bytes. In other words, the next address available for
    /// appending additional trie data.
    TrieDataSize = Segment::GlobalMetadata as usize + 2,
    /// The size of the `TrieData` segment, in bytes, represented as a whole address.
    /// In other words, the next address available for appending additional trie data.
    RlpDataPos = Segment::GlobalMetadata as usize + 3,
    /// A pointer to the root of the state trie within the `TrieData` buffer.
    StateTrieRoot = Segment::GlobalMetadata as usize + 4,
    /// A pointer to the root of the transaction trie within the `TrieData` buffer.
    TransactionTrieRoot = Segment::GlobalMetadata as usize + 5,
    /// A pointer to the root of the receipt trie within the `TrieData` buffer.
    ReceiptTrieRoot = Segment::GlobalMetadata as usize + 6,

    // The root digests of each Merkle trie before these transactions.
    StateTrieRootDigestBefore = Segment::GlobalMetadata as usize + 7,
    TransactionTrieRootDigestBefore = Segment::GlobalMetadata as usize + 8,
    ReceiptTrieRootDigestBefore = Segment::GlobalMetadata as usize + 9,

    // The root digests of each Merkle trie after these transactions.
    StateTrieRootDigestAfter = Segment::GlobalMetadata as usize + 10,
    TransactionTrieRootDigestAfter = Segment::GlobalMetadata as usize + 11,
    ReceiptTrieRootDigestAfter = Segment::GlobalMetadata as usize + 12,

    /// The sizes of the `TrieEncodedChild` and `TrieEncodedChildLen` buffers. In other words, the
    /// next available offset in these buffers.
    TrieEncodedChildSize = Segment::GlobalMetadata as usize + 13,

    // Block metadata.
    BlockBeneficiary = Segment::GlobalMetadata as usize + 14,
    BlockTimestamp = Segment::GlobalMetadata as usize + 15,
    BlockNumber = Segment::GlobalMetadata as usize + 16,
    BlockDifficulty = Segment::GlobalMetadata as usize + 17,
    BlockRandom = Segment::GlobalMetadata as usize + 18,
    BlockGasLimit = Segment::GlobalMetadata as usize + 19,
    BlockChainId = Segment::GlobalMetadata as usize + 20,
    BlockBaseFee = Segment::GlobalMetadata as usize + 21,
    BlockGasUsed = Segment::GlobalMetadata as usize + 22,
    /// Before current transactions block values.
    BlockGasUsedBefore = Segment::GlobalMetadata as usize + 23,
    /// After current transactions block values.
    BlockGasUsedAfter = Segment::GlobalMetadata as usize + 24,
    /// Current block header hash
    BlockCurrentHash = Segment::GlobalMetadata as usize + 25,

    /// Gas to refund at the end of the transaction.
    RefundCounter = Segment::GlobalMetadata as usize + 26,
    /// Length of the addresses access list.
    AccessedAddressesLen = Segment::GlobalMetadata as usize + 27,
    /// Length of the storage keys access list.
    AccessedStorageKeysLen = Segment::GlobalMetadata as usize + 28,
    /// Length of the self-destruct list.
    SelfDestructListLen = Segment::GlobalMetadata as usize + 29,
    /// Length of the bloom entry buffer.
    BloomEntryLen = Segment::GlobalMetadata as usize + 30,

    /// Length of the journal.
    JournalLen = Segment::GlobalMetadata as usize + 31,
    /// Length of the `JournalData` segment.
    JournalDataLen = Segment::GlobalMetadata as usize + 32,
    /// Current checkpoint.
    CurrentCheckpoint = Segment::GlobalMetadata as usize + 33,
    TouchedAddressesLen = Segment::GlobalMetadata as usize + 34,
    // Gas cost for the access list in type-1 txns. See EIP-2930.
    AccessListDataCost = Segment::GlobalMetadata as usize + 35,
    // Start of the access list in the RLP for type-1 txns.
    AccessListRlpStart = Segment::GlobalMetadata as usize + 36,
    // Length of the access list in the RLP for type-1 txns.
    AccessListRlpLen = Segment::GlobalMetadata as usize + 37,
    // Boolean flag indicating if the txn is a contract creation txn.
    ContractCreation = Segment::GlobalMetadata as usize + 38,
    IsPrecompileFromEoa = Segment::GlobalMetadata as usize + 39,
    CallStackDepth = Segment::GlobalMetadata as usize + 40,
    /// Transaction logs list length
    LogsLen = Segment::GlobalMetadata as usize + 41,
    LogsDataLen = Segment::GlobalMetadata as usize + 42,
    LogsPayloadLen = Segment::GlobalMetadata as usize + 43,
    TxnNumberBefore = Segment::GlobalMetadata as usize + 44,
    TxnNumberAfter = Segment::GlobalMetadata as usize + 45,

    KernelHash = Segment::GlobalMetadata as usize + 46,
    KernelLen = Segment::GlobalMetadata as usize + 47,
}

impl GlobalMetadata {
    pub(crate) const COUNT: usize = 48;

    pub(crate) const fn all() -> [Self; Self::COUNT] {
        [
            Self::LargestContext,
            Self::MemorySize,
            Self::TrieDataSize,
            Self::RlpDataPos,
            Self::StateTrieRoot,
            Self::TransactionTrieRoot,
            Self::ReceiptTrieRoot,
            Self::StateTrieRootDigestBefore,
            Self::TransactionTrieRootDigestBefore,
            Self::ReceiptTrieRootDigestBefore,
            Self::StateTrieRootDigestAfter,
            Self::TransactionTrieRootDigestAfter,
            Self::ReceiptTrieRootDigestAfter,
            Self::TrieEncodedChildSize,
            Self::BlockBeneficiary,
            Self::BlockTimestamp,
            Self::BlockNumber,
            Self::BlockDifficulty,
            Self::BlockRandom,
            Self::BlockGasLimit,
            Self::BlockChainId,
            Self::BlockBaseFee,
            Self::BlockGasUsed,
            Self::BlockGasUsedBefore,
            Self::BlockGasUsedAfter,
            Self::RefundCounter,
            Self::AccessedAddressesLen,
            Self::AccessedStorageKeysLen,
            Self::SelfDestructListLen,
            Self::BloomEntryLen,
            Self::JournalLen,
            Self::JournalDataLen,
            Self::CurrentCheckpoint,
            Self::TouchedAddressesLen,
            Self::AccessListDataCost,
            Self::AccessListRlpStart,
            Self::AccessListRlpLen,
            Self::ContractCreation,
            Self::IsPrecompileFromEoa,
            Self::CallStackDepth,
            Self::LogsLen,
            Self::LogsDataLen,
            Self::LogsPayloadLen,
            Self::BlockCurrentHash,
            Self::TxnNumberBefore,
            Self::TxnNumberAfter,
            Self::KernelHash,
            Self::KernelLen,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) const fn var_name(&self) -> &'static str {
        match self {
            Self::LargestContext => "GLOBAL_METADATA_LARGEST_CONTEXT",
            Self::MemorySize => "GLOBAL_METADATA_MEMORY_SIZE",
            Self::TrieDataSize => "GLOBAL_METADATA_TRIE_DATA_SIZE",
            Self::RlpDataPos => "GLOBAL_METADATA_RLP_DATA_POS",
            Self::StateTrieRoot => "GLOBAL_METADATA_STATE_TRIE_ROOT",
            Self::TransactionTrieRoot => "GLOBAL_METADATA_TXN_TRIE_ROOT",
            Self::ReceiptTrieRoot => "GLOBAL_METADATA_RECEIPT_TRIE_ROOT",
            Self::StateTrieRootDigestBefore => "GLOBAL_METADATA_STATE_TRIE_DIGEST_BEFORE",
            Self::TransactionTrieRootDigestBefore => "GLOBAL_METADATA_TXN_TRIE_DIGEST_BEFORE",
            Self::ReceiptTrieRootDigestBefore => "GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_BEFORE",
            Self::StateTrieRootDigestAfter => "GLOBAL_METADATA_STATE_TRIE_DIGEST_AFTER",
            Self::TransactionTrieRootDigestAfter => "GLOBAL_METADATA_TXN_TRIE_DIGEST_AFTER",
            Self::ReceiptTrieRootDigestAfter => "GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_AFTER",
            Self::TrieEncodedChildSize => "GLOBAL_METADATA_TRIE_ENCODED_CHILD_SIZE",
            Self::BlockBeneficiary => "GLOBAL_METADATA_BLOCK_BENEFICIARY",
            Self::BlockTimestamp => "GLOBAL_METADATA_BLOCK_TIMESTAMP",
            Self::BlockNumber => "GLOBAL_METADATA_BLOCK_NUMBER",
            Self::BlockDifficulty => "GLOBAL_METADATA_BLOCK_DIFFICULTY",
            Self::BlockRandom => "GLOBAL_METADATA_BLOCK_RANDOM",
            Self::BlockGasLimit => "GLOBAL_METADATA_BLOCK_GAS_LIMIT",
            Self::BlockChainId => "GLOBAL_METADATA_BLOCK_CHAIN_ID",
            Self::BlockBaseFee => "GLOBAL_METADATA_BLOCK_BASE_FEE",
            Self::BlockGasUsed => "GLOBAL_METADATA_BLOCK_GAS_USED",
            Self::BlockGasUsedBefore => "GLOBAL_METADATA_BLOCK_GAS_USED_BEFORE",
            Self::BlockGasUsedAfter => "GLOBAL_METADATA_BLOCK_GAS_USED_AFTER",
            Self::BlockCurrentHash => "GLOBAL_METADATA_BLOCK_CURRENT_HASH",
            Self::RefundCounter => "GLOBAL_METADATA_REFUND_COUNTER",
            Self::AccessedAddressesLen => "GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN",
            Self::AccessedStorageKeysLen => "GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN",
            Self::SelfDestructListLen => "GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN",
            Self::BloomEntryLen => "GLOBAL_METADATA_BLOOM_ENTRY_LEN",
            Self::JournalLen => "GLOBAL_METADATA_JOURNAL_LEN",
            Self::JournalDataLen => "GLOBAL_METADATA_JOURNAL_DATA_LEN",
            Self::CurrentCheckpoint => "GLOBAL_METADATA_CURRENT_CHECKPOINT",
            Self::TouchedAddressesLen => "GLOBAL_METADATA_TOUCHED_ADDRESSES_LEN",
            Self::AccessListDataCost => "GLOBAL_METADATA_ACCESS_LIST_DATA_COST",
            Self::AccessListRlpStart => "GLOBAL_METADATA_ACCESS_LIST_RLP_START",
            Self::AccessListRlpLen => "GLOBAL_METADATA_ACCESS_LIST_RLP_LEN",
            Self::ContractCreation => "GLOBAL_METADATA_CONTRACT_CREATION",
            Self::IsPrecompileFromEoa => "GLOBAL_METADATA_IS_PRECOMPILE_FROM_EOA",
            Self::CallStackDepth => "GLOBAL_METADATA_CALL_STACK_DEPTH",
            Self::LogsLen => "GLOBAL_METADATA_LOGS_LEN",
            Self::LogsDataLen => "GLOBAL_METADATA_LOGS_DATA_LEN",
            Self::LogsPayloadLen => "GLOBAL_METADATA_LOGS_PAYLOAD_LEN",
            Self::TxnNumberBefore => "GLOBAL_METADATA_TXN_NUMBER_BEFORE",
            Self::TxnNumberAfter => "GLOBAL_METADATA_TXN_NUMBER_AFTER",
            Self::KernelHash => "GLOBAL_METADATA_KERNEL_HASH",
            Self::KernelLen => "GLOBAL_METADATA_KERNEL_LEN",
        }
    }
}
