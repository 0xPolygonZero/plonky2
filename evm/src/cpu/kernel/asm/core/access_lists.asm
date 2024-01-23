/// Access lists for addresses and storage keys.
/// The access list is stored in a sorted linked list in SEGMENT_ACCESSED_ADDRESSES for addresses and
/// SEGMENT_ACCESSED_STORAGE_KEYS segment for storage keys. The length of
/// the segments is stored in the global metadata.
/// Both arrays are stored in the kernel memory (context=0).
/// Searching and inserting is done by guessing the predecessor in the list.
/// If the address/storage key isn't found in the array, it is inserted at the end.

// Initialize the set of accessed addresses and storage keys with an empty list of the form (@U256_MAX)⮌
// wich is written as [@U256_MAX, @SEGMENT_ACCESSED_ADDRESSES] in SEGMENT_ACCESSED_ADDRESSES
// and as [@U256_MAX, _, _, @SEGMENT_ACCESSED_STORAGE_KEYS] in SEGMENT_ACCESSED_STORAGE_KEYS
global init_access_lists:
    // stack: (empty)
    // Initialize SEGMENT_ACCESSED_ADDRESSES
    // Store @U256_MAX at the beggining of the segment
    PUSH @SEGMENT_ACCESSED_ADDRESSES
    DUP1
    PUSH @U256_MAX
    MSTORE_GENERAL
    // Store @SEGMENT_ACCESSED_ADDRESSES at address 1
    %increment
    DUP1
    PUSH @SEGMENT_ACCESSED_ADDRESSES
    MSTORE_GENERAL

    //Store the segment scaled length
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN)
    // stack: (empty)

    // Initialize SEGMENT_ACCESSED_STORAGE_KEYS
    // Store @U256_MAX at the beggining of the segment
    PUSH @SEGMENT_ACCESSED_STORAGE_KEYS
    DUP1
    PUSH @U256_MAX
    MSTORE_GENERAL
    // Store @SEGMENT_ACCESSED_STORAGE_KEYS at address 3
    %add_const(3)
    DUP1
    PUSH @SEGMENT_ACCESSED_STORAGE_KEYS
    MSTORE_GENERAL
    
    //Store the segment scaled length
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN)
    JUMP

%macro init_access_lists
    PUSH %%after
    %jump(init_access_lists)
%%after:
%endmacro

%macro insert_accessed_addresses
    %stack (addr) -> (addr, %%after)
    %jump(insert_accessed_addresses)
%%after:
    // stack: cold_access
%endmacro

%macro insert_accessed_addresses_no_return
    %insert_accessed_addresses
    POP
%endmacro

/// Inserts the address into the access list if it is not already present.
/// Return 1 if the address was inserted, 0 if it was already present.
global insert_accessed_addresses:
    // stack: addr, retdest
    PROVER_INPUT(access_lists::address_pred)
    // stack: pred_ptr, addr, retdest
    DUP1
    MLOAD_GENERAL
    // stack: pred_addr, pred_ptr, addr, retdest
    DUP3 SUB
    %jumpi(insert_new_address)
    // Check that this is not a deleted node
    %increment
    MLOAD_GENERAL
    PUSH @U256_MAX
    SUB
    %jumpi(address_found)
    %jump(panic)
address_found:
    // The address was already in the list
    %stack (addr, retdest) -> (retdest, 0) // Return 0 to indicate that the address was already present.
    JUMP

insert_new_address:
    // stack: pred_ptr, addr, retdest
    // get the value of the next address
    %increment
    // stack: next_ptr_ptr, 
    %mload_global_metadata(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN)
    DUP2
    MLOAD_GENERAL
    // stack: next_ptr, new_ptr, next_ptr_ptr, addr, retdest
    // Check that this is not a deleted node
    DUP1
    PUSH @U256_MAX
    SUB
    %assert_nonzero
    DUP1
    MLOAD_GENERAL
    // stack: next_val, next_ptr, new_ptr, next_ptr_ptr, addr, retdest
    DUP5
    // Since the list is correctly ordered, addr != pred_addr and addr < next_val implies that
    // pred_addr < addr < next_val and hence the new value can be inserted between pred and next
    %assert_lt
    // stack: next_ptr, new_ptr, next_ptr_ptr, addr, retdest
    SWAP2
    DUP2
    MSTORE_GENERAL
    // stack: new_ptr, next_ptr, addr, retdest
    DUP1
    DUP4
    MSTORE_GENERAL
    // stack: new_ptr, next_ptr, addr, retdest
    %increment
    DUP1
    // stack: new_next_ptr, new_next_ptr, next_ptr, addr, retdest
    SWAP2
    MSTORE_GENERAL
    // stack: new_next_ptr, addr, retdest
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN)
    // stack: addr, retdest
    %journal_add_account_loaded
    PUSH 1
    SWAP1
    JUMP

/// Remove the address from the access list.
/// Panics if the address is not in the access list.
global remove_accessed_addresses:
    // stack: addr, retdest
    PROVER_INPUT(access_lists::address_pred)
    // stack: pred_ptr, addr, retdest
    %increment
    DUP1
    MLOAD_GENERAL
    // stack: next_ptr, pred_next_ptr, addr, retdest
    DUP1
    MLOAD_GENERAL
    // stack: next_val, next_ptr, pred_next_ptr, addr, retdest
    DUP4
    %assert_eq
    // stack: next_ptr, pred_next_ptr, addr, retdest
    %increment
    DUP1
    MLOAD_GENERAL
    // stack: next_next_ptr, next_ptr, pred_next_ptr, addr, retdest
    SWAP1
    PUSH @U256_MAX
    MSTORE_GENERAL
    // stack: next_next_ptr, pred_next_ptr, addr, retdest
    MSTORE_GENERAL
    POP
    JUMP


%macro insert_accessed_storage_keys
    %stack (addr, key, value) -> (addr, key, value, %%after)
    %jump(insert_accessed_storage_keys)
%%after:
    // stack: cold_access, original_value
%endmacro

/// Inserts the storage key and value into the access list if it is not already present.
/// `value` should be the current storage value at the slot `(addr, key)`.
/// Return `1, original_value` if the storage key was inserted, `0, original_value` if it was already present.
global insert_accessed_storage_keys:
    // stack: addr, key, value, retdest
    PROVER_INPUT(access_lists::storage_pred)
    // stack: pred_ptr, addr, key, value, retdest
    DUP1
    MLOAD_GENERAL
global debug_storage_pred_addr:
    // stack: pred_addr, pred_ptr, addr, key, value, retdest
    DUP3 SUB
global debug_before_jump:
    %jumpi(insert_new_storage_key)
    // stack: pred_ptr, addr, key, value, retdest
    // Check that this is not a deleted node
    DUP1
    %add_const(3)
    MLOAD_GENERAL
    PUSH @U256_MAX
    SUB
    %jumpi(storage_key_found)
    %jump(panic)
storage_key_found:
    // The address was already in the list
    %add_const(2)
    MLOAD_GENERAL
    %stack (original_value, addr, key, value, retdest) -> (retdest, 0, original_value) // Return 0 to indicate that the address was already present.
    JUMP
insert_new_storage_key:
global debug_new_storage_key:
    // stack: pred_ptr, addr, key, value, retdest
    // get the value of the next address
    %add_const(3)
    // stack: next_ptr_ptr, addr, key, value, retdest
    %mload_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN)
    DUP2
    MLOAD_GENERAL
    // stack: next_ptr, new_ptr, next_ptr_ptr, addr, key, value, retdest
    // Check that this is not a deleted node
    DUP1
    PUSH @U256_MAX
    SUB
    %assert_nonzero
    DUP1
    MLOAD_GENERAL
    // stack: next_val, next_ptr, new_ptr, next_ptr_ptr, addr, key, value, retdest
    DUP5
    // Since the list is correctly ordered, addr != pred_addr and addr < next_val implies that
    // pred_addr < addr < next_val and hence the new value can be inserted between pred and next
    %assert_lt
    // stack: next_ptr, new_ptr, next_ptr_ptr, addr, key, value, retdest
    SWAP2
    DUP2
    MSTORE_GENERAL
    // stack: new_ptr, next_ptr, addr, key, value, retdest
    DUP1
    DUP4
    MSTORE_GENERAL // store addr
    // stack: new_ptr, next_ptr, addr, key, value, retdest
    %increment
    DUP1
    DUP5
    MSTORE_GENERAL // store key
    %increment
    DUP1
    DUP6
    MSTORE_GENERAL // store value
    // stack: new_ptr + 2, next_ptr, addr, key, value, retdest
    %increment
    DUP1
    // stack: new_next_ptr, new_next_ptr, next_ptr, addr, key, value, retdest
    SWAP2
    MSTORE_GENERAL
    // stack: new_next_ptr, addr, key, value, retdest
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN)
    // stack: addr, key, value, retdest
    %stack (addr, key, value, retdest) -> (key, value, retdest, 1, value)
    %journal_add_storage_loaded
    JUMP


global insert_accessed_storage_keys_old:
    // stack: addr, key, value, retdest
    %mload_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN)
    // stack: len, addr, key, value, retdest
    PUSH @SEGMENT_ACCESSED_STORAGE_KEYS ADD
    PUSH @SEGMENT_ACCESSED_STORAGE_KEYS
insert_accessed_storage_keys_loop:
    // `i` and `len` are both scaled by SEGMENT_ACCESSED_STORAGE_KEYS
    %stack (i, len, addr, key, value, retdest) -> (i, len, i, len, addr, key, value, retdest)
    EQ %jumpi(insert_storage_key)
    // stack: i, len, addr, key, value, retdest
    DUP1 %increment MLOAD_GENERAL
    // stack: loaded_key, i, len, addr, key, value, retdest
    DUP2 MLOAD_GENERAL
    // stack: loaded_addr, loaded_key, i, len, addr, key, value, retdest
    DUP5 EQ
    // stack: loaded_addr==addr, loaded_key, i, len, addr, key, value, retdest
    SWAP1 DUP6 EQ
    // stack: loaded_key==key, loaded_addr==addr, i, len, addr, key, value, retdest
    MUL // AND
    %jumpi(insert_accessed_storage_keys_found)
    // stack: i, len, addr, key, value, retdest
    %add_const(3)
    %jump(insert_accessed_storage_keys_loop)

insert_storage_key:
    // stack: i, len, addr, key, value, retdest
    DUP4 DUP4 %journal_add_storage_loaded // Add a journal entry for the loaded storage key.
    // stack: i, len, addr, key, value, retdest

    %stack(dst, len, addr, key, value) -> (addr, dst, dst, key, dst, value, dst, @SEGMENT_ACCESSED_STORAGE_KEYS, value)
    MSTORE_GENERAL // Store new address at the end of the array.
    // stack: dst, key, dst, value, dst, segment, value, retdest
    %increment SWAP1
    MSTORE_GENERAL // Store new key after that
    // stack: dst, value, dst, segment, value, retdest
    %add_const(2) SWAP1
    MSTORE_GENERAL // Store new value after that
    // stack: dst, segment, value, retdest
    %add_const(3)
    SUB // unscale dst
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN) // Store new length.
    %stack (value, retdest) -> (retdest, 1, value) // Return 1 to indicate that the storage key was inserted.
    JUMP

insert_accessed_storage_keys_found:
    // stack: i, len, addr, key, value, retdest
    %add_const(2)
    MLOAD_GENERAL
    %stack (original_value, len, addr, key, value, retdest) -> (retdest, 0, original_value) // Return 0 to indicate that the storage key was already present.
    JUMP

/// Remove the storage key and its value from the access list.
/// Panics if the key is not in the list.
global remove_accessed_storage_keys:
    // stack: addr, key, retdest
    %mload_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN)
    // stack: len, addr, key, retdest
    PUSH @SEGMENT_ACCESSED_STORAGE_KEYS ADD
    PUSH @SEGMENT_ACCESSED_STORAGE_KEYS
remove_accessed_storage_keys_loop:
    // `i` and `len` are both scaled by SEGMENT_ACCESSED_STORAGE_KEYS
    %stack (i, len, addr, key, retdest) -> (i, len, i, len, addr, key, retdest)
    EQ %jumpi(panic)
    // stack: i, len, addr, key, retdest
    DUP1 %increment MLOAD_GENERAL
    // stack: loaded_key, i, len, addr, key, retdest
    DUP2 MLOAD_GENERAL
    // stack: loaded_addr, loaded_key, i, len, addr, key, retdest
    DUP5 EQ
    // stack: loaded_addr==addr, loaded_key, i, len, addr, key, retdest
    SWAP1 DUP6 EQ
    // stack: loaded_key==key, loaded_addr==addr, i, len, addr, key, retdest
    MUL // AND
    %jumpi(remove_accessed_storage_keys_found)
    // stack: i, len, addr, key, retdest
    %add_const(3)
    %jump(remove_accessed_storage_keys_loop)

remove_accessed_storage_keys_found:
    %stack (i, len, addr, key, retdest) -> (len, 3, i, retdest)
    SUB 
    PUSH @SEGMENT_ACCESSED_STORAGE_KEYS
    DUP2 SUB // unscale
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN) // Decrease the access list length.
    // stack: len-3, i, retdest
    DUP1 %add_const(2) MLOAD_GENERAL
    // stack: last_value, len-3, i, retdest
    DUP2 %add_const(1) MLOAD_GENERAL
    // stack: last_key, last_value, len-3, i, retdest
    DUP3 MLOAD_GENERAL
    // stack: last_addr, last_key, last_value, len-3, i, retdest
    DUP5 %swap_mstore // Move the last tuple to the position of the removed tuple.
    // stack: last_key, last_value, len-3, i, retdest
    DUP4 %add_const(1) %swap_mstore
    // stack: last_value, len-3, i, retdest
    DUP3 %add_const(2) %swap_mstore
    // stack: len-3, i, retdest
    %pop2 JUMP
