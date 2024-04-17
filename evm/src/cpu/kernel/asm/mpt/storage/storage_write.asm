// Write a word to the current account's storage trie.
//
// Pre stack: kexit_info, slot, value
// Post stack: (empty)

global sys_sstore:
    %check_static
    DUP1 %leftover_gas %le_const(@GAS_CALLSTIPEND) %jumpi(fault_exception)
    %stack (kexit_info, slot, value) -> (slot, kexit_info, slot, value)
    %sload_current
    %address
    %stack (addr, current_value, kexit_info, slot, value) -> (addr, slot, current_value, current_value, kexit_info, slot, value)
    %insert_accessed_storage_keys
    // stack: cold_access, original_value, current_value, kexit_info, slot, value
    %mul_const(@GAS_COLDSLOAD)

    // Check for warm access.
    %stack (gas, original_value, current_value, kexit_info, slot, value) ->
        (value, current_value, current_value, original_value, gas, original_value, current_value, kexit_info, slot, value)
    EQ SWAP2 EQ ISZERO
    // stack: current_value==original_value, value==current_value, gas, original_value, current_value, kexit_info, slot, value)
    ADD // OR
    %jumpi(sstore_warm)

    // Check for sset (set a zero storage slot to a non-zero value).
    // stack: gas, original_value, current_value, kexit_info, slot, value
    DUP2 ISZERO %mul_const(@GAS_SSET) ADD

    // Check for sreset (set a non-zero storage slot to a non-zero value).
    // stack: gas, original_value, current_value, kexit_info, slot, value
    DUP2 ISZERO ISZERO %mul_const(@GAS_SRESET) ADD
    %jump(sstore_charge_gas)

sstore_warm:
    // stack: gas, original_value, current_value, kexit_info, slot, value)
    %add_const(@GAS_WARMACCESS)

sstore_charge_gas:
    %stack (gas, original_value, current_value, kexit_info, slot, value) -> (gas, kexit_info, current_value, value, original_value, slot)
    %charge_gas

sstore_refund:
    %stack (kexit_info, current_value, value, original_value, slot) -> (current_value, value, current_value, value, original_value, slot, kexit_info)
    EQ %jumpi(sstore_no_refund)
    %stack (current_value, value, original_value, slot, kexit_info) -> (current_value, original_value, current_value, value, original_value, slot, kexit_info)
    EQ %jumpi(sstore_refund_original)
    %stack (current_value, value, original_value, slot, kexit_info) -> (original_value, current_value, value, original_value, slot, kexit_info)
    ISZERO %jumpi(sstore_dirty_reset)
    %stack (current_value, value, original_value, slot, kexit_info) -> (current_value, current_value, value, original_value, slot, kexit_info)
    ISZERO %jumpi(sstore_dirty_clear1)
    %stack (current_value, value, original_value, slot, kexit_info) -> (value, current_value, value, original_value, slot, kexit_info)
    ISZERO %jumpi(sstore_dirty_clear2)
    %jump(sstore_dirty_reset)

sstore_dirty_clear1:
    PUSH @REFUND_SCLEAR PUSH 0 SUB %refund_gas
    %jump(sstore_dirty_reset)

sstore_dirty_clear2:
    PUSH @REFUND_SCLEAR %refund_gas

sstore_dirty_reset:
    %stack (current_value, value, original_value, slot, kexit_info) -> (original_value, value, current_value, value, original_value, slot, kexit_info)
    EQ %jumpi(sstore_dirty_reset2)
    %jump(sstore_no_refund)
sstore_dirty_reset2:
    %stack (current_value, value, original_value, slot, kexit_info) -> (original_value, current_value, value, original_value, slot, kexit_info)
    ISZERO %jumpi(sstore_dirty_reset_sset)
    PUSH @GAS_WARMACCESS PUSH @GAS_SRESET SUB %refund_gas
    %jump(sstore_no_refund)
sstore_dirty_reset_sset:
    PUSH @GAS_WARMACCESS PUSH @GAS_SSET SUB %refund_gas
    %jump(sstore_no_refund)

sstore_refund_original:
    %stack (current_value, value, original_value, slot, kexit_info) -> (value, current_value, value, original_value, slot, kexit_info)
    ISZERO %jumpi(sstore_sclear)
    %jump(sstore_no_refund)
sstore_sclear:
    PUSH @REFUND_SCLEAR %refund_gas
    %jump(sstore_no_refund)

sstore_no_refund:
    %stack (current_value, value, original_value, slot, kexit_info) -> (kexit_info, current_value, slot, value)
sstore_after_refund:
    // stack: kexit_info, current_value, slot, value
    // Check if `value` is equal to `current_value`, and if so exit the kernel early.
    %stack (kexit_info, current_value, slot, value) -> (value, current_value, current_value, slot, value, kexit_info)
    EQ %jumpi(sstore_noop)

    // stack: current_value, slot, value, kexit_info
    DUP2 %address %journal_add_storage_change
    // stack: slot, value, kexit_info

    // If the value is zero, delete the slot from the storage trie.
    // stack: slot, value, kexit_info
    DUP2 ISZERO %jumpi(sstore_delete)

    // First we write the value to MPT data, and get a pointer to it.
    %get_trie_data_size
    // stack: value_ptr, slot, value, kexit_info
    SWAP2
    // stack: value, slot, value_ptr, kexit_info
    %append_to_trie_data
    // stack: slot, value_ptr, kexit_info

    // Next, call mpt_insert on the current account's storage root.
    %stack (slot, value_ptr) -> (slot, value_ptr, after_storage_insert)
    %slot_to_storage_key
    // stack: storage_key, value_ptr, after_storage_insert, kexit_info
    PUSH 64 // storage_key has 64 nibbles
    %current_storage_trie
    // stack: storage_root_ptr, 64, storage_key, value_ptr, after_storage_insert, kexit_info
    %jump(mpt_insert)

after_storage_insert:
    // stack: new_storage_root_ptr, kexit_info
    %current_account_data
    // stack: account_ptr, new_storage_root_ptr, kexit_info

    // Update the copied account with our new storage root pointer.
    %add_const(2)
    // stack: account_storage_root_ptr_ptr, new_storage_root_ptr, kexit_info
    %mstore_trie_data
    // stack: kexit_info
    EXIT_KERNEL

sstore_noop:
    // stack: current_value, slot, value, kexit_info
    %pop3
    EXIT_KERNEL

// Delete the slot from the storage trie.
sstore_delete:
    // stack: slot, value, kexit_info
    SWAP1 POP
    PUSH after_storage_insert SWAP1
    // stack: slot, after_storage_insert, kexit_info
    %slot_to_storage_key
    // stack: storage_key, after_storage_insert, kexit_info
    PUSH 64 // storage_key has 64 nibbles
    %current_storage_trie
    // stack: storage_root_ptr, 64, storage_key, after_storage_insert, kexit_info
    %jump(mpt_delete)
