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
    /// Contains normalized transaction fields; see `TxnField`.
    TxnFields = 8,
    /// Contains the data field of a transaction.
    TxnData = 9,
    /// Raw RLP data.
    RlpRaw = 10,
}

impl Segment {
    pub(crate) const COUNT: usize = 11;

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
        }
    }
}
