// Read a word from the current account's storage trie.
//
// Pre stack: kexit_info, slot
// Post stack: value

global sys_sload:
    // stack: kexit_info, slot
    SWAP1
    %stack (slot) -> (slot, after_storage_read, slot)
    %slot_to_storage_key
    // stack: storage_key, after_storage_read, slot, kexit_info
    PUSH 64 // storage_key has 64 nibbles
    %current_storage_trie
    // stack: storage_root_ptr, 64, storage_key, after_storage_read, slot, kexit_info
    %jump(mpt_read)


after_storage_read:
    // stack: value_ptr, slot, kexit_info
    DUP1 %jumpi(storage_key_exists)

    // Storage key not found. Return default value_ptr = 0,
    // which derefs to 0 since @SEGMENT_TRIE_DATA[0] = 0.
    %stack (value_ptr, slot, kexit_info) -> (slot, 0, kexit_info)
    %jump(sload_gas)

storage_key_exists:
    // stack: value_ptr, slot, kexit_info
    %mload_trie_data
    // stack: value, slot, kexit_info
    SWAP1
    %jump(sload_gas)

sload_gas:
    %stack (slot, value, kexit_info) -> (slot, value, kexit_info, value)
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

