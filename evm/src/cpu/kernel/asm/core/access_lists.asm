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

// Multiply the ptr a the top of the stack by 2
// and abort if 2*ptr - @SEGMENT_ACCESSED_ADDRESSES >= @GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN
%macro get_valid_addr_ptr
    // stack: ptr
    %mul_const(2)
    DUP1
    %sub_const(@SEGMENT_ACCESSED_ADDRESSES)
    %assert_lt_const(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN)
    // sack: 2*ptr
%endmacro


/// Inserts the address into the access list if it is not already present.
/// Return 1 if the address was inserted, 0 if it was already present.
global insert_accessed_addresses:
    // stack: addr, retdest
    PROVER_INPUT(access_lists::address_insert)
    // stack: pred_ptr/2, addr, retdest
    %get_valid_addr_ptr
    // stack: pred_ptr, addr, retdest
    DUP1
    MLOAD_GENERAL
    // stack: pred_addr, pred_ptr, addr, retdest
    // If pred_add < addr OR pred_ptr == @SEGMENT_ACCESSED_ADDRESSES
    DUP2
    %eq_const(@SEGMENT_ACCESSED_ADDRESSES)
    DUP2 DUP5 GT
    OR 
    %jumpi(insert_new_address)
    // addr shouldn't be > pred_addr
    // stack: pred_addr, pred_ptr, addr, retdest
    DUP3
    // If addr >= pred_addr then addr == pred_addr
    %assert_eq
    
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
    // stack: pred_addr, red_ptr, addr, retdest
    POP
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
    // Since the pred_addr < addr or pred == @SEGMENT_ACCESSED_STORAGE_KEYS and addr < next_val, the new value
    // can be inserted between pred and next
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
/// Otherwise it guesses the node before the address (pred)
/// such that (pred)->(next)->(next_next), where the (next) node
/// stores the address. It writes the link (pred)->(next_next)
/// and (next) is marked as deleted by writting U256_MAX in its 
/// next node pointer
global remove_accessed_addresses:
    // stack: addr, retdest
    PROVER_INPUT(access_lists::address_remove)
    // stack: pred_ptr/2, addr, retdest
    %get_valid_addr_ptr
    // stack: pred_ptr, addr, retdest
    %increment
    // stack: next_ptr_ptr, addr, retdest
    DUP1
    MLOAD_GENERAL
    // stack: next_ptr, next_ptr_ptr, addr, retdest
    DUP1
    MLOAD_GENERAL
    // stack: next_val, next_ptr, next_ptr_ptr, addr, retdest
    DUP4
    %assert_eq
    // stack: next_ptr, next_ptr_ptr, addr, retdest
    %increment
    // stack: next_next_ptr_ptr, next_ptr_ptr, addr, retdest
    DUP1
    MLOAD_GENERAL
    // stack: next_next_ptr, next_next_ptr_ptr, next_ptr_ptr, addr, retdest
    SWAP1
    PUSH @U256_MAX
    MSTORE_GENERAL
    // stack: next_next_ptr, next_ptr_ptr, addr, retdest
    MSTORE_GENERAL
    POP
    JUMP


%macro insert_accessed_storage_keys
    %stack (addr, key, value) -> (addr, key, value, %%after)
    %jump(insert_accessed_storage_keys)
%%after:
    // stack: cold_access, original_value
%endmacro

// Multiply the ptr a the top of the stack by 4
// and abort if 4*ptr - SEGMENT_ACCESSED_STORAGE_KEYS >= @GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN
%macro get_valid_storage_ptr
    // stack: ptr
    %mul_const(4)
    DUP1
    %sub_const(@SEGMENT_ACCESSED_STORAGE_KEYS)
    %assert_lt_const(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN)
    // sack: 2*ptr
%endmacro

/// Inserts the storage key and value into the access list if it is not already present.
/// `value` should be the current storage value at the slot `(addr, key)`.
/// Return `1, original_value` if the storage key was inserted, `0, original_value` if it was already present.
global insert_accessed_storage_keys:
    // stack: addr, key, value, retdest
    PROVER_INPUT(access_lists::storage_insert)
    // stack: pred_ptr/4, addr, key, value, retdest
    %get_valid_storage_ptr
    // stack: pred_ptr, addr, key, value, retdest
    DUP1
    MLOAD_GENERAL
    DUP1
    // stack: pred_addr, pred_addr, pred_ptr, addr, key, value, retdest
    DUP4 GT
    DUP3 %eq_const(@SEGMENT_ACCESSED_STORAGE_KEYS)
    ADD
    %jumpi(insert_storage_key)
    // stack: pred_addr, pred_ptr, addr, key, value, retdest
    // It must hold that pred_addr == addr
    DUP3
    %assert_eq
    // stack: pred_ptr, addr, key, value, retdest
    DUP1
    %increment
    MLOAD_GENERAL
    // stack: pred_key, pred_ptr, addr, key, value, retdest
    DUP1 DUP5
    GT
    %jumpi(insert_storage_key)
    // stack: pred_key, pred_ptr, addr, key, value, retdest
    DUP4
    // It  must hold that pred_key == key
    %assert_eq
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
    // stack: pred_ptr, addr, key, value, retdest
    %add_const(2)
    MLOAD_GENERAL
    %stack (original_value, addr, key, value, retdest) -> (retdest, 0, original_value) // Return 0 to indicate that the address was already present.
    JUMP

insert_storage_key:
    // stack: pred_addr or pred_key, pred_ptr, addr, key, value, retdest
    POP
    // Insert a new storage key
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
    // Check that next_val > addr OR (next_val == addr AND next_key > key)
    DUP2 DUP2
    LT
    // stack: addr < next_val, addr, next_val, next_ptr, new_ptr, next_ptr_ptr, addr, key, value, retdest
    SWAP2
    EQ
    // stack: next_val == addr, addr < next_val, next_ptr, new_ptr, next_ptr_ptr, addr, key, value, retdest
    DUP3 %increment
    MLOAD_GENERAL
    DUP8
    LT
    AND
    OR
    %assert_nonzero
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
    %stack (addr, key, value, retdest) -> (addr, key, retdest, 1, value)
    %journal_add_storage_loaded
    JUMP


/// Remove the storage key and its value from the access list.
/// Panics if the key is not in the list.
global remove_accessed_storage_keys:
    // stack: addr, key, retdest
    PROVER_INPUT(access_lists::storage_remove)
    // stack: pred_ptr/4, addr, key, retdest
    %get_valid_storage_ptr
    // stack: pred_ptr, addr, key, retdest
    %add_const(3)
    // stack: next_ptr_ptr, addr, key, retdest
    DUP1
    MLOAD_GENERAL
    // stack: next_ptr, next_ptr_ptr, addr, key, retdest
    DUP1
    %increment
    MLOAD_GENERAL
    // stack: next_key, next_ptr, next_ptr_ptr, addr, key, retdest
    DUP5
    EQ
    DUP2
    MLOAD_GENERAL
    // stack: next_addr, next_key == key, next_ptr, next_ptr_ptr, addr, key, retdest
    DUP5
    EQ
    MUL
    // stack: next_addr  == addr AND next_key == key, next_ptr, next_ptr_ptr, addr, key, retdest
    %assert_nonzero
    // stack: next_ptr, next_ptr_ptr, addr, key, retdest
    %add_const(3)
    // stack: next_next_ptr_ptr, next_ptr_ptr, addr, key, retdest
    DUP1
    MLOAD_GENERAL
    // stack: next_next_ptr, next_next_ptr_ptr, next_ptr_ptr, addr, key, retdest
    SWAP1
    PUSH @U256_MAX
    MSTORE_GENERAL
    // stack: next_next_ptr, next_ptr_ptr, addr, key, retdest
    MSTORE_GENERAL
    %pop2
    JUMP