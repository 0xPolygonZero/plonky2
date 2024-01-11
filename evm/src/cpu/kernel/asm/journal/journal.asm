%macro journal_size
    %mload_global_metadata(@GLOBAL_METADATA_JOURNAL_LEN)
%endmacro

%macro mstore_journal
    // stack: virtual, value
    %mstore_kernel(@SEGMENT_JOURNAL)
    // stack: (empty)
%endmacro

%macro mload_journal
    // stack: virtual
    %mload_kernel(@SEGMENT_JOURNAL)
    // stack: value
%endmacro

%macro append_journal
    // stack: pointer
    %journal_size
    // stack: journal_size, pointer
    SWAP1 DUP2
    // stack: journal_size, pointer, journal_size
    %mstore_journal
    // stack: journal_size
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_JOURNAL_LEN)
%endmacro

%macro journal_data_size
    %mload_global_metadata(@GLOBAL_METADATA_JOURNAL_DATA_LEN)
%endmacro

%macro mstore_journal_data
    // stack: virtual, value
    %mstore_kernel(@SEGMENT_JOURNAL_DATA)
    // stack: (empty)
%endmacro

%macro mload_journal_data
    // stack: virtual
    %mload_kernel(@SEGMENT_JOURNAL_DATA)
    // stack: value
%endmacro

%macro append_journal_data
    // stack: value
    %journal_data_size
    // stack: size, value
    SWAP1 DUP2
    // stack: size, value, size
    %mstore_journal_data
    // stack: size
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_JOURNAL_DATA_LEN)
%endmacro

%macro journal_add_1(type)
    // stack: w
    %journal_data_size
    // stack: ptr, w
    PUSH $type %append_journal_data
    // stack: ptr, w
    SWAP1
    // stack: w, ptr
    %append_journal_data
    // stack: ptr
    %append_journal
%endmacro

%macro journal_add_2(type)
    // stack: w, x
    %journal_data_size
    // stack: ptr, w, x
    PUSH $type %append_journal_data
    // stack: ptr, w, x
    SWAP1 %append_journal_data
    // stack: ptr, x
    SWAP1 %append_journal_data
    // stack: ptr
    %append_journal
%endmacro

%macro journal_add_3(type)
    // stack: w, x, y
    %journal_data_size
    // stack: ptr, w, x, y
    PUSH $type %append_journal_data
    // stack: ptr, w, x, y
    SWAP1 %append_journal_data
    // stack: ptr, x, y
    SWAP1 %append_journal_data
    // stack: ptr, y
    SWAP1 %append_journal_data
    // stack: ptr
    %append_journal
%endmacro

%macro journal_add_4(type)
    // stack: w, x, y, z
    %journal_data_size
    // stack: ptr, w, x, y, z
    PUSH $type %append_journal_data
    // stack: ptr, w, x, y, z
    SWAP1 %append_journal_data
    // stack: ptr, x, y, z
    SWAP1 %append_journal_data
    // stack: ptr, y, z
    SWAP1 %append_journal_data
    // stack: ptr, z
    SWAP1 %append_journal_data
    // stack: ptr
    %append_journal
%endmacro

%macro journal_load_1
    // ptr
    %add_const(1)
    %mload_journal_data
    // w
%endmacro

%macro journal_load_2
    // ptr
    DUP1
    %add_const(2)
    %mload_journal_data
    // x, ptr
    SWAP1
    %add_const(1)
    %mload_journal_data
    // w, x
%endmacro

%macro journal_load_3
    // ptr
    DUP1
    %add_const(3)
    %mload_journal_data
    // y, ptr
    SWAP1
    DUP1
    // ptr, ptr, y
    %add_const(2)
    %mload_journal_data
    // x, ptr, y
    SWAP1
    %add_const(1)
    %mload_journal_data
    // w, x, y
%endmacro

%macro journal_load_4
    // ptr
    DUP1
    %add_const(4)
    %mload_journal_data
    // z, ptr
    SWAP1
    DUP1
    // ptr, ptr, z
    %add_const(3)
    %mload_journal_data
    // y, ptr, z
    SWAP1
    DUP1
    // ptr, ptr, y, z
    %add_const(2)
    %mload_journal_data
    // x, ptr, y, z
    SWAP1
    %add_const(1)
    %mload_journal_data
    // w, x, y, z
%endmacro

%macro current_checkpoint
    %mload_global_metadata(@GLOBAL_METADATA_CURRENT_CHECKPOINT)
%endmacro


%macro checkpoint
    // stack: (empty)
    %current_checkpoint
    // stack: current_checkpoint
    %journal_size
    // stack: journal_size, current_checkpoint
    DUP2 %mstore_kernel(@SEGMENT_JOURNAL_CHECKPOINTS)
    // stack: current_checkpoint
    %mload_context_metadata(@CTX_METADATA_CHECKPOINTS_LEN)
    // stack: i, current_checkpoint
    DUP2 DUP2 %mstore_current(@SEGMENT_CONTEXT_CHECKPOINTS)
    // stack: i, current_checkpoint
    %increment
    %mstore_context_metadata(@CTX_METADATA_CHECKPOINTS_LEN)
    // stack: current_checkpoint
    %increment
    %mstore_global_metadata(@GLOBAL_METADATA_CURRENT_CHECKPOINT)
    // stack: (empty)
%endmacro

%macro pop_checkpoint
    PUSH 1
    %mload_context_metadata(@CTX_METADATA_CHECKPOINTS_LEN)
    // stack: i
    SUB
    %mstore_context_metadata(@CTX_METADATA_CHECKPOINTS_LEN)
%endmacro
