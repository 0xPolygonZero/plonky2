// Load the given global metadata field from memory.
%macro mload_global_metadata(field)
    // stack: (empty)
    PUSH $field
    // stack: offset
    %mload_kernel(@SEGMENT_GLOBAL_METADATA)
    // stack: value
%endmacro

// Store the given global metadata field to memory.
%macro mstore_global_metadata(field)
    // stack: value
    PUSH $field
    // stack: offset, value
    %mstore_kernel(@SEGMENT_GLOBAL_METADATA)
    // stack: (empty)
%endmacro

// Load the given context metadata field from memory.
%macro mload_context_metadata(field)
    // stack: (empty)
    PUSH $field
    // stack: offset
    %mload_current(@SEGMENT_CONTEXT_METADATA)
    // stack: value
%endmacro

// Store the given context metadata field to memory.
%macro mstore_context_metadata(field)
    // stack: value
    PUSH $field
    // stack: offset, value
    %mstore_current(@SEGMENT_CONTEXT_METADATA)
    // stack: (empty)
%endmacro

// Store the given context metadata field to memory.
%macro mstore_context_metadata(field, value)
    PUSH $value
    PUSH $field
    // stack: offset, value
    %mstore_current(@SEGMENT_CONTEXT_METADATA)
    // stack: (empty)
%endmacro

%macro mstore_parent_context_metadata(field)
    // stack: value
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, value) ->
        (parent_ctx, @SEGMENT_CONTEXT_METADATA, $field, value)
    MSTORE_GENERAL
    // stack: (empty)
%endmacro

%macro mstore_parent_context_metadata(field, value)
    // stack: (empty)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx) ->
        (parent_ctx, @SEGMENT_CONTEXT_METADATA, $field, $value)
    MSTORE_GENERAL
    // stack: (empty)
%endmacro

%macro address
    %mload_context_metadata(@CTX_METADATA_ADDRESS)
%endmacro

global sys_address:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %address
    // stack: address, kexit_info
    SWAP1
    EXIT_KERNEL

%macro caller
    %mload_context_metadata(@CTX_METADATA_CALLER)
%endmacro

global sys_caller:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %caller
    // stack: caller, kexit_info
    SWAP1
    EXIT_KERNEL

%macro callvalue
    %mload_context_metadata(@CTX_METADATA_CALL_VALUE)
%endmacro

%macro codesize
    %mload_context_metadata(@CTX_METADATA_CODE_SIZE)
%endmacro

global sys_codesize:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %codesize
    // stack: codesize, kexit_info
    SWAP1
    EXIT_KERNEL

global sys_callvalue:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %callvalue
    // stack: callvalue, kexit_info
    SWAP1
    EXIT_KERNEL

%macro mem_words
    %mload_context_metadata(@CTX_METADATA_MEM_WORDS)
%endmacro

%macro msize
    %mem_words
    %mul_const(32)
%endmacro

global sys_msize:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %msize
    // stack: msize, kexit_info
    SWAP1
    EXIT_KERNEL

%macro calldatasize
    %mload_context_metadata(@CTX_METADATA_CALLDATA_SIZE)
%endmacro

global sys_calldatasize:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %calldatasize
    // stack: calldatasize, kexit_info
    SWAP1
    EXIT_KERNEL

%macro returndatasize
    %mload_context_metadata(@CTX_METADATA_RETURNDATA_SIZE)
%endmacro

global sys_returndatasize:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %returndatasize
    // stack: returndatasize, kexit_info
    SWAP1
    EXIT_KERNEL

%macro coinbase
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_BENEFICIARY)
%endmacro

global sys_coinbase:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %coinbase
    // stack: coinbase, kexit_info
    SWAP1
    EXIT_KERNEL

%macro timestamp
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_TIMESTAMP)
%endmacro

global sys_timestamp:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %timestamp
    // stack: timestamp, kexit_info
    SWAP1
    EXIT_KERNEL

%macro blocknumber
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_NUMBER)
%endmacro

global sys_number:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %blocknumber
    // stack: blocknumber, kexit_info
    SWAP1
    EXIT_KERNEL

%macro blockgaslimit
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_GAS_LIMIT)
%endmacro

global sys_gaslimit:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %blockgaslimit
    // stack: blockgaslimit, kexit_info
    SWAP1
    EXIT_KERNEL

%macro blockchainid
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_CHAIN_ID)
%endmacro

global sys_chainid:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %blockchainid
    // stack: chain_id, kexit_info
    SWAP1
    EXIT_KERNEL

%macro basefee
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_BASE_FEE)
%endmacro

global sys_basefee:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %basefee
    // stack: basefee, kexit_info
    SWAP1
    EXIT_KERNEL

%macro update_mem_words
    // stack: num_words, kexit_info
    %mem_words
    // stack: old_num_words, num_words, kexit_info
    DUP2 DUP2 GT
    // stack: old_num_words > num_words, old_num_words, num_words, kexit_info
    %jumpi(%%no_update)
    // stack: old_num_words, num_words, kexit_info
    %memory_cost
    // stack: old_cost, num_words, kexit_info
    SWAP1
    // stack: num_words, old_cost, kexit_info
    DUP1 %mstore_context_metadata(@CTX_METADATA_MEM_WORDS)
    // stack: num_words, old_cost, kexit_info
    %memory_cost
    // stack: new_cost, old_cost, kexit_info
    SUB
    // stack: additional_cost, kexit_info
    %charge_gas
    %jump(%%end)
%%no_update:
    // stack: old_num_words, num_words, kexit_info
    %pop2
%%end:
    // stack: kexit_info
%endmacro

%macro update_mem_bytes
    // stack: num_bytes, kexit_info
    %num_bytes_to_num_words
    // stack: num_words, kexit_info
    %update_mem_words
    // stack: kexit_info
%endmacro

%macro num_bytes_to_num_words
    // stack: num_bytes
    %add_const(31)
    // stack: 31 + num_bytes
    %div_const(32)
    // stack: (num_bytes + 31) / 32
%endmacro

%macro memory_cost
    // stack: num_words
    DUP1
    // stack: num_words, msize
    %mul_const(@GAS_MEMORY)
    // stack: num_words * GAS_MEMORY, msize
    SWAP1
    // stack: num_words, num_words * GAS_MEMORY
    %square
    %div_const(512)
    // stack: num_words^2 / 512, num_words * GAS_MEMORY
    ADD
    // stack: cost = num_words^2 / 512 + num_words * GAS_MEMORY
%endmacro

// Faults if the given offset is "unreasonable", i.e. the associated memory expansion cost
// would exceed any reasonable block limit.
// We do this to avoid overflows in future gas-related calculations.
%macro ensure_reasonable_offset
    // stack: offset
    // The memory expansion cost, (50000000 / 32)^2 / 512, is around 2^32 gas,
    // i.e. greater than any reasonable block limit.
    %gt_const(50000000)
    // stack: is_unreasonable
    %jumpi(fault_exception)
    // stack: (empty)
%endmacro

// Convenience macro for checking if the current context is static.
// Called before state-changing opcodes.
%macro check_static
    %mload_context_metadata(@CTX_METADATA_STATIC)
    %jumpi(fault_exception)
%endmacro

%macro add_or_fault
    // stack: offset, size, kexit_info, offset, size
    DUP1
    %ensure_reasonable_offset
    // stack: offset, size, kexit_info, offset, size
    DUP2
    // stack: size, offset, size, kexit_info, offset, size
    %ensure_reasonable_offset
    // stack: offset, size, kexit_info, offset, size
    ADD 
%endmacro
