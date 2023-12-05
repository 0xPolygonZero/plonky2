/// These metadata fields contain VM state specific to a particular context.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum ContextMetadata {
    /// The ID of the context which created this one.
    ParentContext = 0,
    /// The program counter to return to when we return to the parent context.
    ParentProgramCounter = 1,
    CalldataSize = 2,
    ReturndataSize = 3,
    /// The address of the account associated with this context.
    Address = 4,
    /// The size of the code under the account associated with this context.
    /// While this information could be obtained from the state trie, it is best to cache it since
    /// the `CODESIZE` instruction is very cheap.
    CodeSize = 5,
    /// The address of the caller who spawned this context.
    Caller = 6,
    /// The value (in wei) deposited by the caller.
    CallValue = 7,
    /// Whether this context was created by `STATICCALL`, in which case state changes are
    /// prohibited.
    Static = 8,
    /// Pointer to the initial version of the state trie, at the creation of this context. Used when
    /// we need to revert a context.
    StateTrieCheckpointPointer = 9,
    /// Size of the active main memory, in (32 byte) words.
    MemWords = 10,
    StackSize = 11,
    /// The gas limit for this call (not the entire transaction).
    GasLimit = 12,
    ContextCheckpointsLen = 13,
}

impl ContextMetadata {
    pub(crate) const COUNT: usize = 14;

    pub(crate) const fn all() -> [Self; Self::COUNT] {
        [
            Self::ParentContext,
            Self::ParentProgramCounter,
            Self::CalldataSize,
            Self::ReturndataSize,
            Self::Address,
            Self::CodeSize,
            Self::Caller,
            Self::CallValue,
            Self::Static,
            Self::StateTrieCheckpointPointer,
            Self::MemWords,
            Self::StackSize,
            Self::GasLimit,
            Self::ContextCheckpointsLen,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) const fn var_name(&self) -> &'static str {
        match self {
            ContextMetadata::ParentContext => "CTX_METADATA_PARENT_CONTEXT",
            ContextMetadata::ParentProgramCounter => "CTX_METADATA_PARENT_PC",
            ContextMetadata::CalldataSize => "CTX_METADATA_CALLDATA_SIZE",
            ContextMetadata::ReturndataSize => "CTX_METADATA_RETURNDATA_SIZE",
            ContextMetadata::Address => "CTX_METADATA_ADDRESS",
            ContextMetadata::CodeSize => "CTX_METADATA_CODE_SIZE",
            ContextMetadata::Caller => "CTX_METADATA_CALLER",
            ContextMetadata::CallValue => "CTX_METADATA_CALL_VALUE",
            ContextMetadata::Static => "CTX_METADATA_STATIC",
            ContextMetadata::StateTrieCheckpointPointer => "CTX_METADATA_STATE_TRIE_CHECKPOINT_PTR",
            ContextMetadata::MemWords => "CTX_METADATA_MEM_WORDS",
            ContextMetadata::StackSize => "CTX_METADATA_STACK_SIZE",
            ContextMetadata::GasLimit => "CTX_METADATA_GAS_LIMIT",
            ContextMetadata::ContextCheckpointsLen => "CTX_METADATA_CHECKPOINTS_LEN",
        }
    }
}
