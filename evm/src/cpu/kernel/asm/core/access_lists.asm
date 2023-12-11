/// Access lists for addresses and storage keys.
/// The access list is stored in an array. The length of the array is stored in the global metadata.
/// For storage keys, the address and key are stored as two consecutive elements.
/// The array is stored in the SEGMENT_ACCESSED_ADDRESSES segment for addresses and in the SEGMENT_ACCESSED_STORAGE_KEYS segment for storage keys.
/// Both arrays are stored in the kernel memory (context=0).
/// Searching and inserting is done by doing a linear search through the array.
/// If the address/storage key isn't found in the array, it is inserted at the end.
/// TODO: Look into using a more efficient data structure for the access lists.

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
    %mload_global_metadata(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN)
    // stack: len, addr, retdest
    PUSH 0
insert_accessed_addresses_loop:
    %stack (i, len, addr, retdest) -> (i, len, i, len, addr, retdest)
    EQ %jumpi(insert_address)
    // stack: i, len, addr, retdest
    DUP1 %mload_kernel(@SEGMENT_ACCESSED_ADDRESSES)
    // stack: loaded_addr, i, len, addr, retdest
    DUP4
    // stack: addr, loaded_addr, i, len, addr, retdest
    EQ %jumpi(insert_accessed_addresses_found)
    // stack: i, len, addr, retdest
    %increment
    %jump(insert_accessed_addresses_loop)

insert_address:
    %stack (i, len, addr, retdest) -> (i, addr, len, retdest)
    DUP2 %journal_add_account_loaded // Add a journal entry for the loaded account.
    %mstore_kernel(@SEGMENT_ACCESSED_ADDRESSES) // Store new address at the end of the array.
    // stack: len, retdest
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN) // Store new length.
    PUSH 1 // Return 1 to indicate that the address was inserted.
    SWAP1 JUMP

insert_accessed_addresses_found:
    %stack (i, len, addr, retdest) -> (retdest, 0) // Return 0 to indicate that the address was already present.
    JUMP

/// Remove the address from the access list.
/// Panics if the address is not in the access list.
global remove_accessed_addresses:
    // stack: addr, retdest
    %mload_global_metadata(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN)
    // stack: len, addr, retdest
    PUSH 0
remove_accessed_addresses_loop:
    %stack (i, len, addr, retdest) -> (i, len, i, len, addr, retdest)
    EQ %jumpi(panic)
    // stack: i, len, addr, retdest
    DUP1 %mload_kernel(@SEGMENT_ACCESSED_ADDRESSES)
    // stack: loaded_addr, i, len, addr, retdest
    DUP4
    // stack: addr, loaded_addr, i, len, addr, retdest
    EQ %jumpi(remove_accessed_addresses_found)
    // stack: i, len, addr, retdest
    %increment
    %jump(remove_accessed_addresses_loop)
remove_accessed_addresses_found:
    %stack (i, len, addr, retdest) -> (len, 1, i, retdest)
    SUB DUP1 %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN) // Decrement the access list length.
    // stack: len-1, i, retdest
    %mload_kernel(@SEGMENT_ACCESSED_ADDRESSES) // Load the last address in the access list.
    // stack: last_addr, i, retdest
    SWAP1
    %mstore_kernel(@SEGMENT_ACCESSED_ADDRESSES) // Store the last address at the position of the removed address.
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
    %mload_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN)
    // stack: len, addr, key, value, retdest
    PUSH 0
insert_accessed_storage_keys_loop:
    %stack (i, len, addr, key, value, retdest) -> (i, len, i, len, addr, key, value, retdest)
    EQ %jumpi(insert_storage_key)
    // stack: i, len, addr, key, value, retdest
    DUP1 %increment %mload_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: loaded_key, i, len, addr, key, value, retdest
    DUP2 %mload_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
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
    DUP1
    PUSH @SEGMENT_ACCESSED_STORAGE_KEYS
    %build_kernel_address

    %stack(dst, i, len, addr, key, value) -> (addr, dst, dst, key, dst, value, i, value)
    MSTORE_GENERAL // Store new address at the end of the array.
    // stack: dst, key, dst, value, i, value, retdest
    %increment SWAP1
    MSTORE_GENERAL // Store new key after that
    // stack: dst, value, i, value, retdest
    %add_const(2) SWAP1
    MSTORE_GENERAL // Store new value after that
    // stack: i, value, retdest
    %add_const(3)
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN) // Store new length.
    %stack (value, retdest) -> (retdest, 1, value) // Return 1 to indicate that the storage key was inserted.
    JUMP

insert_accessed_storage_keys_found:
    // stack: i, len, addr, key, value, retdest
    %add_const(2)
    %mload_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
    %stack (original_value, len, addr, key, value, retdest) -> (retdest, 0, original_value) // Return 0 to indicate that the storage key was already present.
    JUMP

/// Remove the storage key and its value from the access list.
/// Panics if the key is not in the list.
global remove_accessed_storage_keys:
    // stack: addr, key, retdest
    %mload_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN)
    // stack: len, addr, key, retdest
    PUSH 0
remove_accessed_storage_keys_loop:
    %stack (i, len, addr, key, retdest) -> (i, len, i, len, addr, key, retdest)
    EQ %jumpi(panic)
    // stack: i, len, addr, key, retdest
    DUP1 %increment %mload_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: loaded_key, i, len, addr, key, retdest
    DUP2 %mload_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
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
    SUB DUP1 %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN) // Decrease the access list length.
    // stack: len-3, i, retdest
    DUP1 %add_const(2) %mload_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: last_value, len-3, i, retdest
    DUP2 %add_const(1) %mload_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: last_key, last_value, len-3, i, retdest
    DUP3 %mload_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: last_addr, last_key, last_value, len-3, i, retdest
    DUP5 %mstore_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS) // Move the last tuple to the position of the removed tuple.
    // stack: last_key, last_value, len-3, i, retdest
    DUP4 %add_const(1) %mstore_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: last_value, len-3, i, retdest
    DUP3 %add_const(2) %mstore_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: len-3, i, retdest
    %pop2 JUMP
