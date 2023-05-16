/// Self-destruct list.
/// Implemented as an array, with the length stored in the global metadata.
/// Note: This array allows duplicates.

%macro insert_selfdestruct_list
    // stack: addr
    %mload_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN)
    %stack (len, addr) -> (len, addr, len)
    %mstore_kernel(@SEGMENT_SELFDESTRUCT_LIST) // Store new address at the end of the array.
    // stack: len
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN) // Store new length.
%endmacro

/// Remove one occurrence of the address from the list.
/// Panics if the address is not in the list.
global remove_selfdestruct_list:
    // stack: addr, retdest
    %mload_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN)
    // stack: len, addr, retdest
    PUSH 0
remove_selfdestruct_list_loop:
    %stack (i, len, addr, retdest) -> (i, len, i, len, addr, retdest)
    EQ %jumpi(panic)
    // stack: i, len, addr, retdest
    DUP1 %mload_kernel(@SEGMENT_SELFDESTRUCT_LIST)
    // stack: loaded_addr, i, len, addr, retdest
    DUP4
    // stack: addr, loaded_addr, i, len, addr, retdest
    EQ %jumpi(remove_selfdestruct_list_found)
    // stack: i, len, addr, retdest
    %increment
    %jump(remove_selfdestruct_list_loop)
remove_selfdestruct_list_found:
    %stack (i, len, addr, retdest) -> (len, 1, i, retdest)
    SUB DUP1 %mstore_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN) // Decrement the list length.
    // stack: len-1, i, retdest
    %mload_kernel(@SEGMENT_SELFDESTRUCT_LIST) // Load the last address in the list.
    // stack: last_addr, i, retdest
    SWAP1
    %mstore_kernel(@SEGMENT_SELFDESTRUCT_LIST) // Store the last address at the position of the removed address.
    JUMP

global delete_all_selfdestructed_addresses:
    // stack: retdest
    %mload_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN)
    // stack: len, retdest
    PUSH 0
delete_all_selfdestructed_addresses_loop:
    // stack: i, len, retdest
    DUP2 DUP2 EQ %jumpi(delete_all_selfdestructed_addresses_done)
    // stack: i, len, retdest
    DUP1 %mload_kernel(@SEGMENT_SELFDESTRUCT_LIST)
    // stack: loaded_addr, i, len, retdest
    DUP1 %is_non_existent ISZERO %jumpi(bingo)
    // stack: loaded_addr, i, len, retdest
    POP %increment %jump(delete_all_selfdestructed_addresses_loop)
bingo:
    // stack: loaded_addr, i, len, retdest
    %delete_account
    %increment %jump(delete_all_selfdestructed_addresses_loop)
delete_all_selfdestructed_addresses_done:
    // stack: i, len, retdest
    %pop2 JUMP

%macro delete_all_selfdestructed_addresses
    %stack () -> (%%after)
    %jump(delete_all_selfdestructed_addresses)
%%after:
    // stack: (empty)
%endmacro
