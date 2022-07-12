/// Contains EVM bytecode.
pub const CODE: usize = 0;

pub const STACK: usize = 1;

/// Main memory, owned by the contract code.
pub const MAIN_MEM: usize = 2;

/// Memory owned by the kernel.
pub const KERNEL_MEM: usize = 3;

/// Data passed to the current context by its caller.
pub const CALLDATA: usize = 4;

/// Data returned to the current context by its latest callee.
pub const RETURNDATA: usize = 5;

/// A segment which contains a few fixed-size metadata fields, such as the caller's context, or the
/// size of `CALLDATA` and `RETURNDATA`.
pub const METADATA: usize = 6;

pub const NUM_SEGMENTS: usize = 7;
