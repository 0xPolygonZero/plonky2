/// These metadata fields contain global VM state, stored in the `Segment::Metadata` segment of the
/// kernel's context (which is zero).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum GlobalMetadata {
    /// The largest context ID that has been used so far in this execution. Tracking this allows us
    /// give each new context a unique ID, so that its memory will be zero-initialized.
    LargestContext = 0,
    /// The size of active memory, in bytes.
    MemorySize = 1,
    /// The size of the `TrieData` segment, in bytes. In other words, the next address available for
    /// appending additional trie data.
    TrieDataSize = 2,
    /// The size of the `TrieData` segment, in bytes. In other words, the next address available for
    /// appending additional trie data.
    RlpDataSize = 3,
    /// A pointer to the root of the state trie within the `TrieData` buffer.
    StateTrieRoot = 4,
    /// A pointer to the root of the transaction trie within the `TrieData` buffer.
    TransactionTrieRoot = 5,
    /// A pointer to the root of the receipt trie within the `TrieData` buffer.
    ReceiptTrieRoot = 6,

    // The root digests of each Merkle trie before these transactions.
    StateTrieRootDigestBefore = 8,
    TransactionTrieRootDigestBefore = 9,
    ReceiptTrieRootDigestBefore = 10,

    // The root digests of each Merkle trie after these transactions.
    StateTrieRootDigestAfter = 11,
    TransactionTrieRootDigestAfter = 12,
    ReceiptTrieRootDigestAfter = 13,

    /// The sizes of the `TrieEncodedChild` and `TrieEncodedChildLen` buffers. In other words, the
    /// next available offset in these buffers.
    TrieEncodedChildSize = 14,

    // Block metadata.
    BlockBeneficiary = 15,
    BlockTimestamp = 16,
    BlockNumber = 17,
    BlockDifficulty = 18,
    BlockGasLimit = 19,
    BlockChainId = 20,
    BlockBaseFee = 21,

    /// Gas to refund at the end of the transaction.
    RefundCounter = 22,
    /// Length of the addresses access list.
    AccessedAddressesLen = 23,
    /// Length of the storage keys access list.
    AccessedStorageKeysLen = 24,
    /// Length of the self-destruct list.
    SelfDestructListLen = 25,

    /// Length of the journal.
    JournalLen = 26,
    /// Length of the `JournalData` segment.
    JournalDataLen = 27,
}

impl GlobalMetadata {
    pub(crate) const COUNT: usize = 26;

    pub(crate) fn all() -> [Self; Self::COUNT] {
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
            Self::TrieEncodedChildSize,
            Self::BlockBeneficiary,
            Self::BlockTimestamp,
            Self::BlockNumber,
            Self::BlockDifficulty,
            Self::BlockGasLimit,
            Self::BlockChainId,
            Self::BlockBaseFee,
            Self::RefundCounter,
            Self::AccessedAddressesLen,
            Self::AccessedStorageKeysLen,
            Self::SelfDestructListLen,
            Self::JournalLen,
            Self::JournalDataLen,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
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
            Self::TrieEncodedChildSize => "GLOBAL_METADATA_TRIE_ENCODED_CHILD_SIZE",
            Self::BlockBeneficiary => "GLOBAL_METADATA_BLOCK_BENEFICIARY",
            Self::BlockTimestamp => "GLOBAL_METADATA_BLOCK_TIMESTAMP",
            Self::BlockNumber => "GLOBAL_METADATA_BLOCK_NUMBER",
            Self::BlockDifficulty => "GLOBAL_METADATA_BLOCK_DIFFICULTY",
            Self::BlockGasLimit => "GLOBAL_METADATA_BLOCK_GAS_LIMIT",
            Self::BlockChainId => "GLOBAL_METADATA_BLOCK_CHAIN_ID",
            Self::BlockBaseFee => "GLOBAL_METADATA_BLOCK_BASE_FEE",
            Self::RefundCounter => "GLOBAL_METADATA_REFUND_COUNTER",
            Self::AccessedAddressesLen => "GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN",
            Self::AccessedStorageKeysLen => "GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN",
            Self::SelfDestructListLen => "GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN",
            Self::JournalLen => "GLOBAL_METADATA_JOURNAL_LEN",
            Self::JournalDataLen => "GLOBAL_METADATA_JOURNAL_DATA_LEN",
        }
    }
}
