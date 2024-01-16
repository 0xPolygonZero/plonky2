%macro insert_touched_addresses
    %stack (addr) -> (addr, %%after)
    %jump(insert_touched_addresses)
%%after:
    // stack: (empty)
%endmacro

%macro insert_touched_addresses_no_return
    %insert_touched_addresses
    POP
%endmacro

/// Inserts the address into the list if it is not already present.
global insert_touched_addresses:
    // stack: addr, retdest
    %mload_global_metadata(@GLOBAL_METADATA_TOUCHED_ADDRESSES_LEN)
    // stack: len, addr, retdest
    PUSH @SEGMENT_TOUCHED_ADDRESSES ADD
    PUSH @SEGMENT_TOUCHED_ADDRESSES
insert_touched_addresses_loop:
    // `i` and `len` are both scaled by SEGMENT_TOUCHED_ADDRESSES
    %stack (i, len, addr, retdest) -> (i, len, i, len, addr, retdest)
    EQ %jumpi(insert_address)
    // stack: i, len, addr, retdest
    DUP1 MLOAD_GENERAL
    // stack: loaded_addr, i, len, addr, retdest
    DUP4
    // stack: addr, loaded_addr, i, len, addr, retdest
    EQ %jumpi(insert_touched_addresses_found)
    // stack: i, len, addr, retdest
    %increment
    %jump(insert_touched_addresses_loop)

insert_address:
    %stack (i, len, addr, retdest) -> (i, addr, len, @SEGMENT_TOUCHED_ADDRESSES, retdest)
    DUP2 %journal_add_account_touched // Add a journal entry for the touched account.
    %swap_mstore // Store new address at the end of the array.
    // stack: len, segment, retdest
    SUB // unscale
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_TOUCHED_ADDRESSES_LEN) // Store new length.
    JUMP

insert_touched_addresses_found:
    %stack (i, len, addr, retdest) -> (retdest)
    JUMP

/// Remove the address from the list.
/// Panics if the address is not in the list.
/// TODO: Unused?
global remove_touched_addresses:
    // stack: addr, retdest
    %mload_global_metadata(@GLOBAL_METADATA_TOUCHED_ADDRESSES_LEN)
    // stack: len, addr, retdest
    PUSH @SEGMENT_TOUCHED_ADDRESSES ADD
    PUSH @SEGMENT_TOUCHED_ADDRESSES
remove_touched_addresses_loop:
    // `i` and `len` are both scaled by SEGMENT_TOUCHED_ADDRESSES
    %stack (i, len, addr, retdest) -> (i, len, i, len, addr, retdest)
    EQ %jumpi(panic)
    // stack: i, len, addr, retdest
    DUP1 MLOAD_GENERAL
    // stack: loaded_addr, i, len, addr, retdest
    DUP4
    // stack: addr, loaded_addr, i, len, addr, retdest
    EQ %jumpi(remove_touched_addresses_found)
    // stack: i, len, addr, retdest
    %increment
    %jump(remove_touched_addresses_loop)
remove_touched_addresses_found:
    %stack (i, len, addr, retdest) -> (len, 1, i, retdest)
    SUB
    PUSH @SEGMENT_TOUCHED_ADDRESSES DUP2
    SUB // unscale
    %mstore_global_metadata(@GLOBAL_METADATA_TOUCHED_ADDRESSES_LEN) // Decrement the list length.
    // stack: len-1, i, retdest
    MLOAD_GENERAL // Load the last address in the list.
    // stack: last_addr, i, retdest
    SWAP1
    MLOAD_GENERAL // Store the last address at the position of the removed address.
    JUMP


global delete_all_touched_addresses:
    // stack: retdest
    %mload_global_metadata(@GLOBAL_METADATA_TOUCHED_ADDRESSES_LEN)
    // stack: len, retdest
    PUSH @SEGMENT_TOUCHED_ADDRESSES ADD
    PUSH @SEGMENT_TOUCHED_ADDRESSES
delete_all_touched_addresses_loop:
    // `i` and `len` are both scaled by SEGMENT_TOUCHED_ADDRESSES
    // stack: i, len, retdest
    DUP2 DUP2 EQ %jumpi(delete_all_touched_addresses_done)
    // stack: i, len, retdest
    DUP1 MLOAD_GENERAL
    // stack: loaded_addr, i, len, retdest
    DUP1 %is_empty %jumpi(bingo)
    // stack: loaded_addr, i, len, retdest
    POP %increment %jump(delete_all_touched_addresses_loop)
bingo:
    // stack: loaded_addr, i, len, retdest
    %delete_account
    %increment %jump(delete_all_touched_addresses_loop)
delete_all_touched_addresses_done:
    // stack: i, len, retdest
    %pop2 JUMP

%macro delete_all_touched_addresses
    %stack () -> (%%after)
    %jump(delete_all_touched_addresses)
%%after:
    // stack: (empty)
%endmacro