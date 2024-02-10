/// Self-destruct list.
/// Implemented as an array, with the length stored in the global metadata.
/// Note: This array allows duplicates.

%macro insert_selfdestruct_list
    // stack: addr
    %mload_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN)
    DUP1 PUSH @SEGMENT_SELFDESTRUCT_LIST %build_kernel_address
    %stack (write_addr, len, addr) -> (addr, write_addr, len)
    MSTORE_GENERAL // Store new address at the end of the array.
    // stack: len
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN) // Store new length.
%endmacro

/// Remove one occurrence of the address from the list.
/// No effect if the address is not in the list.
global remove_selfdestruct_list:
    // stack: addr, retdest
    %mload_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN)
    // stack: len, addr, retdest
    PUSH @SEGMENT_SELFDESTRUCT_LIST ADD
    PUSH @SEGMENT_SELFDESTRUCT_LIST
remove_selfdestruct_list_loop:
    // `i` and `len` are both scaled by SEGMENT_SELFDESTRUCT_LIST
    %stack (i, len, addr, retdest) -> (i, len, i, len, addr, retdest)
    EQ %jumpi(remove_selfdestruct_not_found)
    // stack: i, len, addr, retdest
    DUP1 MLOAD_GENERAL
    // stack: loaded_addr, i, len, addr, retdest
    DUP4
    // stack: addr, loaded_addr, i, len, addr, retdest
    EQ %jumpi(remove_selfdestruct_list_found)
    // stack: i, len, addr, retdest
    %increment
    %jump(remove_selfdestruct_list_loop)
remove_selfdestruct_list_found:
    %stack (i, len, addr, retdest) -> (len, 1, i, retdest)
    SUB
    PUSH @SEGMENT_SELFDESTRUCT_LIST
    DUP2 SUB // unscale
    %mstore_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN) // Decrement the list length.
    // stack: len-1, i, retdest
    MLOAD_GENERAL // Load the last address in the list.
    // stack: last_addr, i, retdest
    MSTORE_GENERAL // Store the last address at the position of the removed address.
    JUMP
remove_selfdestruct_not_found:
    // stack: i, len, addr, retdest
    %pop3
    JUMP

global delete_all_selfdestructed_addresses:
    // stack: retdest
    %mload_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN)
    // stack: len, retdest
    PUSH @SEGMENT_SELFDESTRUCT_LIST ADD
    PUSH @SEGMENT_SELFDESTRUCT_LIST
delete_all_selfdestructed_addresses_loop:
    // `i` and `len` are both scaled by SEGMENT_SELFDESTRUCT_LIST
    // stack: i, len, retdest
    DUP2 DUP2 EQ %jumpi(delete_all_selfdestructed_addresses_done)
    // stack: i, len, retdest
    DUP1 MLOAD_GENERAL
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
