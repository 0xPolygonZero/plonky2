#[allow(dead_code)] // TODO: Not all segments are used yet.
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
    Metadata = 5,
    /// General purpose kernel memory, used by various kernel functions.
    /// In general, calling a helper function can result in this memory being clobbered.
    KernelGeneral = 6,
    /// Contains transaction data (after it's parsed and converted to a standard format).
    TxnData = 7,
    /// Raw RLP data.
    RlpRaw = 8,
}

impl Segment {
    pub(crate) const COUNT: usize = 9;

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [
            Self::Code,
            Self::Stack,
            Self::MainMemory,
            Self::Calldata,
            Self::Returndata,
            Self::Metadata,
            Self::KernelGeneral,
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
            Segment::Metadata => "SEGMENT_METADATA",
            Segment::KernelGeneral => "SEGMENT_KERNEL_GENERAL",
            Segment::TxnData => "SEGMENT_TXN_DATA",
            Segment::RlpRaw => "SEGMENT_RLP_RAW",
        }
    }
}
