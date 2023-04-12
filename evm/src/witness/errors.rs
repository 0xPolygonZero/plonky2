#[allow(dead_code)]
#[derive(Debug)]
pub enum ProgramError {
    OutOfGas,
    InvalidOpcode,
    StackUnderflow,
    InvalidJumpDestination,
    InvalidJumpiDestination,
    StackOverflow,
}
