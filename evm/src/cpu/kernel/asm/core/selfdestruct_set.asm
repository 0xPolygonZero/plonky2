/// Self-destruct set.
/// Essentially the same code as in `access_lists.asm`, with the exception that the insert function doesn't return anything.
/// TODO: Would it make sense to merge this with `access_lists.asm`?
/// TODO: Look into using a more efficient data structure.

%macro insert_selfdestruct_set
    %stack (addr) -> (addr, %%after)
    %jump(insert_selfdestruct_set)
%%after:
    // stack: (empty)
%endmacro

/// Inserts the address into the self-destruct set if it is not already present.
global insert_selfdestruct_set:
    // stack: addr, retdest
    %mload_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_SET_LEN)
    // stack: len, addr, retdest
    PUSH 0
insert_selfdestruct_set_loop:
    %stack (i, len, addr, retdest) -> (i, len, i, len, addr, retdest)
    EQ %jumpi(insert_address)
    // stack: i, len, addr, retdest
    DUP1 %mload_kernel(@SEGMENT_SELFDESTRUCT_SET)
    // stack: loaded_addr, i, len, addr, retdest
    DUP4
    // stack: addr, loaded_addr, i, len, addr, retdest
    EQ %jumpi(insert_address_found)
    // stack: i, len, addr, retdest
    %increment
    %jump(insert_selfdestruct_set_loop)

insert_address:
    %stack (i, len, addr, retdest) -> (i, addr, len, retdest)
    %mstore_kernel(@SEGMENT_SELFDESTRUCT_SET) // Store new address at the end of the array.
    // stack: len, retdest
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_SET_LEN) // Store new length.
    JUMP

insert_address_found:
    // stack: i, len, addr, retdest
    %pop3
    JUMP
