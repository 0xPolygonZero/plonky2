%macro sload_current
    %stack (slot) -> (slot, %%after)
    %jump(sload_current)
%%after:
%endmacro

global sload_current:
    // stack: slot, retdest
    %address
    // stack: addr, slot, retdest
    %key_storage %smt_read_state
global watt3:
    %mload_trie_data
global watt2:
    // stack: value, retdest
    SWAP1 JUMP

// Read a word from the current account's storage trie.
//
// Pre stack: kexit_info, slot
// Post stack: value
global sys_sload:
    // stack: kexit_info, slot
    SWAP1
    DUP1
    // stack: slot, slot, kexit_info
    %sload_current
global wattt:

    %stack (value, slot, kexit_info) -> (slot, value, kexit_info, value)
    %address
    // stack: addr, slot, value, kexit_info, value
    %insert_accessed_storage_keys
    // stack: cold_access, old_value, kexit_info, value
    SWAP1 POP
    // stack: cold_access, kexit_info, value
    %mul_const(@GAS_COLDSLOAD_MINUS_WARMACCESS)
    %add_const(@GAS_WARMACCESS)
    %charge_gas
    // stack: kexit_info, value
    EXIT_KERNEL

