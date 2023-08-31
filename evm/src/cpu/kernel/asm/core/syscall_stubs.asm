// Labels for unimplemented syscalls to make the kernel assemble.
// Each label should be removed from this file once it is implemented.

// This is a temporary version that returns 0 on all inputs.
// TODO: Fix this.
global sys_blockhash:
    // stack: kexit_info, block_number
    %charge_gas_const(@GAS_BLOCKHASH)
    %stack (kexit_info, block_number) -> (kexit_info, 0)
    EXIT_KERNEL

// This is a temporary version that returns the block difficulty (i.e. the old version of this opcode).
global sys_prevrandao:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_RANDOM)
    %stack (random, kexit_info) -> (kexit_info, random)
    EXIT_KERNEL
