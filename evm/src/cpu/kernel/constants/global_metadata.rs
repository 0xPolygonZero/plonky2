use crate::memory::segments::Segment;

/// These metadata fields contain global VM state, stored in the `Segment::Metadata` segment of the
/// kernel's context (which is zero).
///
/// Each value is directly scaled by the corresponding `Segment::GlobalMetadata` value for faster
/// memory access in the kernel.
#[allow(clippy::enum_clike_unportable_variant)]
#[repr(usize)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum GlobalMetadata {
    /// The largest context ID that has been used so far in this execution. Tracking this allows us
    /// give each new context a unique ID, so that its memory will be zero-initialized.
    LargestContext = Segment::GlobalMetadata as usize,
    /// The size of active memory, in bytes.
    MemorySize,
    /// The size of the `TrieData` segment, in bytes. In other words, the next address available for
    /// appending additional trie data.
    TrieDataSize,
    /// The size of the `TrieData` segment, in bytes, represented as a whole address.
    /// In other words, the next address available for appending additional trie data.
    RlpDataSize,
    /// A pointer to the root of the state trie within the `TrieData` buffer.
    StateTrieRoot,
    /// A pointer to the root of the transaction trie within the `TrieData` buffer.
    TransactionTrieRoot,
    /// A pointer to the root of the receipt trie within the `TrieData` buffer.
    ReceiptTrieRoot,

    // The root digests of each Merkle trie before these transactions.
    StateTrieRootDigestBefore,
    TransactionTrieRootDigestBefore,
    ReceiptTrieRootDigestBefore,

    // The root digests of each Merkle trie after these transactions.
    StateTrieRootDigestAfter,
    TransactionTrieRootDigestAfter,
    ReceiptTrieRootDigestAfter,

    // Block metadata.
    BlockBeneficiary,
    BlockTimestamp,
    BlockNumber,
    BlockDifficulty,
    BlockRandom,
    BlockGasLimit,
    BlockChainId,
    BlockBaseFee,
    BlockGasUsed,
    /// Before current transactions block values.
    BlockGasUsedBefore,
    /// After current transactions block values.
    BlockGasUsedAfter,
    /// Current block header hash
    BlockCurrentHash,

    /// Gas to refund at the end of the transaction.
    RefundCounter,
    /// Length of the addresses access list.
    AccessedAddressesLen,
    /// Length of the storage keys access list.
    AccessedStorageKeysLen,
    /// Length of the self-destruct list.
    SelfDestructListLen,
    /// Length of the bloom entry buffer.
    BloomEntryLen,

    /// Length of the journal.
    JournalLen,
    /// Length of the `JournalData` segment.
    JournalDataLen,
    /// Current checkpoint.
    CurrentCheckpoint,
    TouchedAddressesLen,
    // Gas cost for the access list in type-1 txns. See EIP-2930.
    AccessListDataCost,
    // Start of the access list in the RLP for type-1 txns.
    AccessListRlpStart,
    // Length of the access list in the RLP for type-1 txns.
    AccessListRlpLen,
    // Boolean flag indicating if the txn is a contract creation txn.
    ContractCreation,
    IsPrecompileFromEoa,
    CallStackDepth,
    /// Transaction logs list length
    LogsLen,
    LogsDataLen,
    LogsPayloadLen,
    TxnNumberBefore,
    TxnNumberAfter,

    KernelHash,
    KernelLen,
}

impl GlobalMetadata {
    pub(crate) const COUNT: usize = 47;

    /// Unscales this virtual offset by their respective `Segment` value.
    pub(crate) const fn unscale(&self) -> usize {
        *self as usize - Segment::GlobalMetadata as usize
    }

    pub(crate) const fn all() -> [Self; Self::COUNT] {
        [
            Self::LargestContext,
            Self::MemorySize,
            Self::TrieDataSize,
            Self::RlpDataSize,
            Self::StateTrieRoot,
            Self::TransactionTrieRoot,
            Self::ReceiptTrieRoot,
            Self::StateTrieRootDigestBefore,
            Self::TransactionTrieRootDigestBefore,
            Self::ReceiptTrieRootDigestBefore,
            Self::StateTrieRootDigestAfter,
            Self::TransactionTrieRootDigestAfter,
            Self::ReceiptTrieRootDigestAfter,
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
            Self::RlpDataSize => "GLOBAL_METADATA_RLP_DATA_SIZE",
            Self::StateTrieRoot => "GLOBAL_METADATA_STATE_TRIE_ROOT",
            Self::TransactionTrieRoot => "GLOBAL_METADATA_TXN_TRIE_ROOT",
            Self::ReceiptTrieRoot => "GLOBAL_METADATA_RECEIPT_TRIE_ROOT",
            Self::StateTrieRootDigestBefore => "GLOBAL_METADATA_STATE_TRIE_DIGEST_BEFORE",
            Self::TransactionTrieRootDigestBefore => "GLOBAL_METADATA_TXN_TRIE_DIGEST_BEFORE",
            Self::ReceiptTrieRootDigestBefore => "GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_BEFORE",
            Self::StateTrieRootDigestAfter => "GLOBAL_METADATA_STATE_TRIE_DIGEST_AFTER",
            Self::TransactionTrieRootDigestAfter => "GLOBAL_METADATA_TXN_TRIE_DIGEST_AFTER",
            Self::ReceiptTrieRootDigestAfter => "GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_AFTER",
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
