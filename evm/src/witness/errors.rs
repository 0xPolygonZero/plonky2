use ethereum_types::U256;

#[allow(dead_code)]
#[derive(Debug)]
pub enum ProgramError {
    OutOfGas,
    InvalidOpcode,
    StackUnderflow,
    InvalidJumpDestination,
    InvalidJumpiDestination,
    StackOverflow,
    KernelPanic,
    MemoryError(MemoryError),
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum MemoryError {
    ContextTooLarge { context: U256 },
    SegmentTooLarge { segment: U256 },
    VirtTooLarge { virt: U256 },
}
