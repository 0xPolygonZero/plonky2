#[allow(dead_code)]
pub enum ProgramError {
    OutOfGas,
    InvalidOpcode,
    StackUnderflow,
    InvalidJumpDestination,
    InvalidJumpiDestination,
    StackOverflow,
}
