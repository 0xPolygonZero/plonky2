global insert_accessed_addresses:
    // stack: addr, retdest
    PUSH 0 %mload_current(@SEGMENT_ACCESSED_ADDRESSES)
    // stack: len, addr, retdest
    %increment
    PUSH 1
insert_accessed_addresses_loop:
    %stack (i, len, addr, retdest) -> (i, len, i, len, addr, retdest)
    EQ %jumpi(insert_address)
    // stack: i, len, addr, retdest
    DUP1 %mload_current(@SEGMENT_ACCESSED_ADDRESSES)
    // stack: loaded_addr, i, len, addr, retdest
    DUP4
    // stack: addr, loaded_addr, i, len, addr, retdest
    EQ %jumpi(insert_accessed_addresses_found)
    // stack: i, len, addr, retdest
    %increment
    %jump(insert_accessed_addresses_loop)

insert_address:
    %stack (i, len, addr, retdest) -> (i, addr, len, retdest)
    %mstore_current(@SEGMENT_ACCESSED_ADDRESSES) // Store new address at the end of the array.
    // stack: len, retdest
    PUSH 0 %mstore_current(@SEGMENT_ACCESSED_ADDRESSES) // Store new length in front of the array.
    PUSH 1 // Return 1 to indicate that the address was inserted.
    SWAP1 JUMP

insert_accessed_addresses_found:
    %stack (i, len, addr, retdest) -> (retdest, 0) // Return 0 to indicate that the address was already present.
    JUMP


global insert_accessed_storage_keys:
    // stack: addr, key, retdest
    PUSH 0 %mload_current(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: len, addr, key, retdest
    %increment
    PUSH 1
insert_accessed_storage_keys_loop:
    %stack (i, len, addr, key, retdest) -> (i, len, i, len, addr, key, retdest)
    EQ %jumpi(insert_storage_key)
    // stack: i, len, addr, key, retdest
    DUP1 %increment %mload_current(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: loaded_key, i, len, addr, key, retdest
    DUP2 %mload_current(@SEGMENT_ACCESSED_STORAGE_KEYS)
    // stack: loaded_addr, loaded_key, i, len, addr, key, retdest
    DUP5 EQ
    // stack: loaded_addr==addr, loaded_key, i, len, addr, key, retdest
    SWAP1 DUP6 EQ
    // stack: loaded_key==key, loaded_addr==addr, i, len, addr, key, retdest
    MUL // AND
    %jumpi(insert_accessed_storage_keys_found)
    // stack: i, len, addr, key, retdest
    %add_const(2)
    %jump(insert_accessed_storage_keys_loop)

insert_storage_key:
    // stack: i, len, addr, key, retdest
    DUP1 %increment
    %stack (i_plus_1, i, len, addr, key, retdest) -> (i, addr, i_plus_1, key, i_plus_1, retdest)
    %mstore_current(@SEGMENT_ACCESSED_STORAGE_KEYS) // Store new address at the end of the array.
    %mstore_current(@SEGMENT_ACCESSED_STORAGE_KEYS) // Store new key after that
    // stack: i_plus_1, retdest
    PUSH 0 %mstore_current(@SEGMENT_ACCESSED_STORAGE_KEYS) // Store new length in front of the array.
    PUSH 1 // Return 1 to indicate that the storage key was inserted.
    SWAP1 JUMP

insert_accessed_storage_keys_found:
    %stack (i, len, addr, key, retdest) -> (retdest, 0) // Return 0 to indicate that the storage key was already present.
    JUMP
