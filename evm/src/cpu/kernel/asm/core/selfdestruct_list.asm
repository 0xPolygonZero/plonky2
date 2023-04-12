/// Self-destruct list.
/// Implemented as an append-only array, with the length stored in the global metadata.

%macro insert_selfdestruct_list
    // stack: addr
    %mload_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN)
    %stack (len, addr) -> (len, addr, len)
    %mstore_kernel(@SEGMENT_SELFDESTRUCT_LIST) // Store new address at the end of the array.
    // stack: len
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN) // Store new length.
%endmacro
