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
    %mstore_kernel(@SEGMENT_ACCESSED_ADDRESSES) // Store new address at the end of the array.
    // stack: len, retdest
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN) // Store new length.
    PUSH 1 // Return 1 to indicate that the address was inserted.
    SWAP1 JUMP

insert_accessed_addresses_found:
    %stack (i, len, addr, retdest) -> (retdest, 0) // Return 0 to indicate that the address was already present.
    JUMP


%macro insert_accessed_storage_keys
    %stack (addr, key, value) -> (addr, key, value, %%after)
    %jump(insert_accessed_storage_keys)
%%after:
    // stack: cold_access
%endmacro

/// Inserts the storage key into the access list if it is not already present.
/// Return 1 if the storage key was inserted, 0 if it was already present.
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
    DUP1 %increment
    DUP1 %increment
    %stack (i_plus_2, i_plus_1, i, len, addr, key, value) -> (i, addr, i_plus_1, key, i_plus_2, value, i_plus_2, value)
    %mstore_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS) // Store new address at the end of the array.
    %mstore_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS) // Store new key after that
    %mstore_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS) // Store new value after that
    // stack: i_plus_2, value, retdest
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN) // Store new length.
    %stack (value, retdest) -> (retdest, 1, value) // Return 1 to indicate that the storage key was inserted.
    JUMP

insert_accessed_storage_keys_found:
    // stack: i, len, addr, key, value, retdest
    %add_const(2)
    %mload_kernel(@SEGMENT_ACCESSED_STORAGE_KEYS)
    %stack (original_value, len, addr, key, value, retdest) -> (retdest, 0, original_value) // Return 0 to indicate that the storage key was already present.
    JUMP
