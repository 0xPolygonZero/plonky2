// Write a word to the current account's storage trie.
//
// Pre stack: kexit_info, slot, value
// Post stack: (empty)

global sys_sstore:
    %check_static
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

global sstore_dirty_clear1:
    %mload_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    %sub_const(@REFUND_SCLEAR)
    %mstore_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    %jump(sstore_dirty_reset)

global sstore_dirty_clear2:
    %mload_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    %add_const(@REFUND_SCLEAR)
    %mstore_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)

global sstore_dirty_reset:
    %stack (current_value, value, original_value, slot, kexit_info) -> (original_value, value, current_value, value, original_value, slot, kexit_info)
    EQ %jumpi(sstore_dirty_reset2)
    %stack (current_value, value, original_value, slot, kexit_info) -> (kexit_info, current_value, slot, value)
    %jump(sstore_after_refund)
global sstore_dirty_reset2:
    %stack (current_value, value, original_value, slot, kexit_info) -> (original_value, current_value, value, original_value, slot, kexit_info)
    ISZERO %jumpi(sstore_dirty_reset_sset)
    %mload_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    %add_const(@GAS_SRESET)
    %sub_const(@GAS_WARMACCESS)
    %mstore_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    %stack (current_value, value, original_value, slot, kexit_info) -> (kexit_info, current_value, slot, value)
    %jump(sstore_after_refund)
global sstore_dirty_reset_sset:
    %mload_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    %add_const(@GAS_SSET)
    %sub_const(@GAS_WARMACCESS)
    %mstore_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    %stack (current_value, value, original_value, slot, kexit_info) -> (kexit_info, current_value, slot, value)
    %jump(sstore_after_refund)

global sstore_refund_original:
    %stack (current_value, value, original_value, slot, kexit_info) -> (original_value, current_value, value, slot, kexit_info)
    ISZERO %jumpi(sstore_sclear)
    %stack (current_value, value, slot, kexit_info) -> (kexit_info, current_value, slot, value)
    %jump(sstore_after_refund)
global sstore_sclear:
    %mload_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    %add_const(@REFUND_SCLEAR)
    %mstore_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    %stack (current_value, value, slot, kexit_info) -> (kexit_info, current_value, slot, value)
    %jump(sstore_after_refund)

global sstore_no_refund:
    %stack (current_value, value, original_value, slot, kexit_info) -> (kexit_info, current_value, slot, value)
global sstore_after_refund:
    %mload_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER) POP
    // stack: kexit_info, current_value, slot, value
    // Check if `value` is equal to `current_value`, and if so exit the kernel early.
    %stack (kexit_info, current_value, slot, value) -> (value, current_value, slot, value, kexit_info)
    EQ %jumpi(sstore_noop)

    // TODO: If value = 0, delete the key instead of inserting 0.
    // stack: slot, value, kexit_info

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
    // stack: old_account_ptr, new_storage_root_ptr, kexit_info
    %make_account_copy
    // stack: new_account_ptr, new_storage_root_ptr, kexit_info

    // Update the copied account with our new storage root pointer.
    %stack (new_account_ptr, new_storage_root_ptr) -> (new_account_ptr, new_storage_root_ptr, new_account_ptr)
    %add_const(2)
    // stack: new_account_storage_root_ptr_ptr, new_storage_root_ptr, new_account_ptr, kexit_info
    %mstore_trie_data
    // stack: new_account_ptr, kexit_info

    // Save this updated account to the state trie.
    %stack (new_account_ptr) -> (new_account_ptr, after_state_insert)
    %address %addr_to_state_key
    // stack: state_key, new_account_ptr, after_state_insert, kexit_info
    %jump(mpt_insert_state_trie)

after_state_insert:
    // stack: kexit_info
    EXIT_KERNEL

sstore_noop:
    // stack: slot, value, kexit_info
    %pop2
    EXIT_KERNEL
