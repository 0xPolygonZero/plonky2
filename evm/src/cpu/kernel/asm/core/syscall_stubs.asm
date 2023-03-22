// Labels for unimplemented syscalls to make the kernel assemble.
// Each label should be removed from this file once it is implemented.

global sys_sdiv:
    PANIC
global sys_smod:
    PANIC
global sys_signextend:
    PANIC
global sys_slt:
    PANIC
global sys_sgt:
    PANIC
global sys_sar:
    PANIC
global sys_blockhash:
    PANIC
global sys_prevrandao:
    // TODO: What semantics will this have for Edge?
    PANIC
global sys_chainid:
    // TODO: Return the block's chain ID instead of the txn's, even though they should match.
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %mload_txn_field(@TXN_FIELD_CHAIN_ID)
    // stack: chain_id, kexit_info
    SWAP1
    EXIT_KERNEL
global sys_log0:
    PANIC
global sys_log1:
    PANIC
global sys_log2:
    PANIC
global sys_log3:
    PANIC
global sys_log4:
    PANIC
