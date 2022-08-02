/// These metadata fields contain VM state specific to a particular context.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub(crate) enum ContextMetadata {
    /// The ID of the context which created this one.
    ParentContext = 0,
    /// The program counter to return to when we return to the parent context.
    ParentProgramCounter = 1,
    /// If we're in a system call, this holds the userspace program counter to return to.
    UserspaceProgramCounter = 2,
    CalldataSize = 3,
    ReturndataSize = 4,
}

impl ContextMetadata {
    pub(crate) const COUNT: usize = 5;

    pub(crate) fn all() -> [Self; Self::COUNT] {
        [
            Self::ParentContext,
            Self::ParentProgramCounter,
            Self::UserspaceProgramCounter,
            Self::CalldataSize,
            Self::ReturndataSize,
        ]
    }

    /// The variable name that gets passed into kernel assembly code.
    pub(crate) fn var_name(&self) -> &'static str {
        match self {
            ContextMetadata::ParentContext => "CTX_METADATA_PARENT_CONTEXT",
            ContextMetadata::ParentProgramCounter => "CTX_METADATA_PARENT_PC",
            ContextMetadata::UserspaceProgramCounter => "CTX_METADATA_USERSPACE_PC",
            ContextMetadata::CalldataSize => "CTX_METADATA_CALLDATA_SIZE",
            ContextMetadata::ReturndataSize => "CTX_METADATA_RETURNDATA_SIZE",
        }
    }
}
