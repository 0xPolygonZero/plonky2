use ethereum_types::U256;

#[derive(Debug)]
pub enum ProgramError {
    OutOfGas,
    InvalidOpcode,
    StackUnderflow,
    InvalidRlp,
    InvalidJumpDestination,
    InvalidJumpiDestination,
    StackOverflow,
    KernelPanic,
    MemoryError(MemoryError),
    GasLimitError,
    InterpreterError,
    IntegerTooLarge,
    ProverInputError(ProverInputError),
    UnknownContractCode,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum MemoryError {
    ContextTooLarge { context: U256 },
    SegmentTooLarge { segment: U256 },
    VirtTooLarge { virt: U256 },
}

#[derive(Debug)]
pub enum ProverInputError {
    OutOfMptData,
    OutOfRlpData,
    OutOfWithdrawalData,
    CodeHashNotFound,
    InvalidMptInput,
    InvalidInput,
    InvalidFunction,
    NumBitsError,
    InvalidJumpDestination,
    InvalidJumpdestSimulation,
}
