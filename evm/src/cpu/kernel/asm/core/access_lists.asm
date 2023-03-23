global insert_accessed_addresses:
    // stack: addr, retdest
    PUSH 0 %mload_current(@SEGMENT_ACCESSED_ADDRESSES)
    // stack: len, addr, retdest
    %increment
    PUSH 1
global insert_accessed_addresses_loop:
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

global insert_address:
    %stack (i, len, addr, retdest) -> (i, addr, len, retdest)
    %mstore_current(@SEGMENT_ACCESSED_ADDRESSES) // Store new address at the end of the array.
    // stack: len, retdest
    PUSH 0 %mstore_current(@SEGMENT_ACCESSED_ADDRESSES) // Store new length in front of the array.
    PUSH 1 // Return 1 to indicate that the address was inserted.
    SWAP1 JUMP

insert_accessed_addresses_found:
    %stack (i, len, addr, retdest) -> (retdest, 0) // Return 0 to indicate that the address was already present.
    JUMP
