// Read a word from the current account's storage trie.
//
// Pre stack: kexit_info, slot
// Post stack: value

global sys_sload:
    // stack: kexit_info, slot
    SWAP1
    // stack: slot, kexit_info
    DUP1 %address
    // stack: addr, slot, slot, kexit_info
    %insert_accessed_storage_keys PUSH @GAS_COLDSLOAD_MINUS_WARMACCESS
    MUL
    PUSH @GAS_WARMACCESS
    ADD
    %stack (gas, slot, kexit_info) -> (gas, kexit_info, slot)
    %charge_gas
    // stack: kexit_info, slot

    SWAP1
    %stack (slot) -> (slot, after_storage_read)
    %slot_to_storage_key
    // stack: storage_key, after_storage_read, kexit_info
    PUSH 64 // storage_key has 64 nibbles
    %current_storage_trie
    // stack: storage_root_ptr, 64, storage_key, after_storage_read, kexit_info
    %jump(mpt_read)

after_storage_read:
    // stack: value_ptr, kexit_info
    DUP1 %jumpi(storage_key_exists)

    // Storage key not found. Return default value_ptr = 0,
    // which derefs to 0 since @SEGMENT_TRIE_DATA[0] = 0.
    %stack (value_ptr, kexit_info) -> (kexit_info, 0)
    EXIT_KERNEL

storage_key_exists:
    // stack: value_ptr, kexit_info
    %mload_trie_data
    // stack: value, kexit_info
    SWAP1
    EXIT_KERNEL
