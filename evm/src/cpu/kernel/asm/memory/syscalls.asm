global sys_mload:
    // stack: kexit_info, offset
    DUP2 %ensure_reasonable_offset
    // stack: kexit_info, offset
    %charge_gas_const(@GAS_VERYLOW)
    // stack: kexit_info, offset
    DUP2 %add_const(32)
    // stack: expanded_num_bytes, kexit_info, offset
    %update_mem_bytes
    // stack: kexit_info, offset
    %stack(kexit_info, offset) -> (offset, 32, kexit_info)
    PUSH @SEGMENT_MAIN_MEMORY
    GET_CONTEXT
    %build_address
    // stack: addr, len, kexit_info
    MLOAD_32BYTES
    %stack (value, kexit_info) -> (kexit_info, value)
    EXIT_KERNEL

global sys_mstore:
    // stack: kexit_info, offset, value
    DUP2 %ensure_reasonable_offset
    // stack: kexit_info, offset, value
    %charge_gas_const(@GAS_VERYLOW)
    // stack: kexit_info, offset, value
    DUP2 %add_const(32)
    // stack: expanded_num_bytes, kexit_info, offset, value
    %update_mem_bytes
    // stack: kexit_info, offset, value
    %stack(kexit_info, offset, value) -> (offset, value, kexit_info)
    PUSH @SEGMENT_MAIN_MEMORY
    GET_CONTEXT
    %build_address
    // stack: addr, value, kexit_info
    MSTORE_32BYTES_32
    POP
    // stack: kexit_info
    EXIT_KERNEL

global sys_mstore8:
    // stack: kexit_info, offset, value
    DUP2 %ensure_reasonable_offset
    // stack: kexit_info, offset, value
    %charge_gas_const(@GAS_VERYLOW)
    // stack: kexit_info, offset, value
    DUP2 %increment
    // stack: expanded_num_bytes, kexit_info, offset, value
    %update_mem_bytes
    // stack: kexit_info, offset, value
    %stack (kexit_info, offset, value) -> (value, 0x100, offset, kexit_info)
    MOD SWAP1
    %mstore_current(@SEGMENT_MAIN_MEMORY)
    // stack: kexit_info
    EXIT_KERNEL

global sys_calldataload:
    // stack: kexit_info, i
    %charge_gas_const(@GAS_VERYLOW)
    // stack: kexit_info, i
    %mload_context_metadata(@CTX_METADATA_CALLDATA_SIZE)
    %stack (calldata_size, kexit_info, i) -> (calldata_size, i, kexit_info, i)
    LT %jumpi(calldataload_large_offset)
    %stack (kexit_info, i) -> (@SEGMENT_CALLDATA, i, 32, sys_calldataload_after_mload_packing, kexit_info)
    GET_CONTEXT
    %build_address
    // stack: addr, 32, sys_calldataload_after_mload_packing, kexit_info
    %jump(mload_packing)
sys_calldataload_after_mload_packing:
    // stack: value, kexit_info
    SWAP1
    EXIT_KERNEL
    PANIC
calldataload_large_offset:
    %stack (kexit_info, i) -> (kexit_info, 0)
    EXIT_KERNEL

// Macro for {CALLDATA, RETURNDATA}COPY (W_copy in Yellow Paper).
%macro wcopy(segment, context_metadata_size)
    // stack: kexit_info, dest_offset, offset, size
    %wcopy_charge_gas

    %stack (kexit_info, dest_offset, offset, size) -> (dest_offset, size, kexit_info, dest_offset, offset, size)
    %add_or_fault
    // stack: expanded_num_bytes, kexit_info, dest_offset, offset, size, kexit_info
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes

    %mload_context_metadata($context_metadata_size)
    // stack: total_size, kexit_info, dest_offset, offset, size
    DUP4
    // stack: offset, total_size, kexit_info, dest_offset, offset, size
    GT %jumpi(wcopy_large_offset)

    // stack: kexit_info, dest_offset, offset, size
    GET_CONTEXT
    PUSH $segment
    // stack: segment, context, kexit_info, dest_offset, offset, size
    %jump(wcopy_within_bounds)
%endmacro

%macro wcopy_charge_gas
    // stack: kexit_info, dest_offset, offset, size
    PUSH @GAS_VERYLOW
    DUP5
    // stack: size, Gverylow, kexit_info, dest_offset, offset, size
    ISZERO %jumpi(wcopy_empty)
    // stack: Gverylow, kexit_info, dest_offset, offset, size
    DUP5 %num_bytes_to_num_words %mul_const(@GAS_COPY) ADD %charge_gas
%endmacro


codecopy_within_bounds:
    // stack: total_size, segment, src_ctx, kexit_info, dest_offset, offset, size
    POP
wcopy_within_bounds:
    // stack: segment, src_ctx, kexit_info, dest_offset, offset, size
    GET_CONTEXT
    %stack (context, segment, src_ctx, kexit_info, dest_offset, offset, size) ->
        (src_ctx, segment, offset, @SEGMENT_MAIN_MEMORY, dest_offset, context, size, wcopy_after, kexit_info)
    %build_address
    SWAP3 %build_address
    // stack: DST, SRC, size, wcopy_after, kexit_info
    %jump(memcpy_bytes)

wcopy_empty:
    // stack: Gverylow, kexit_info, dest_offset, offset, size
    %charge_gas
    %stack (kexit_info, dest_offset, offset, size) -> (kexit_info)
    EXIT_KERNEL


codecopy_large_offset:
    // stack: total_size, src_ctx, kexit_info, dest_offset, offset, size
    %pop2
wcopy_large_offset:
    // offset is larger than the size of the {CALLDATA,CODE,RETURNDATA}. So we just have to write zeros.
    // stack: kexit_info, dest_offset, offset, size
    GET_CONTEXT
    %stack (context, kexit_info, dest_offset, offset, size) ->
        (context, @SEGMENT_MAIN_MEMORY, dest_offset, size, wcopy_after, kexit_info)
    %build_address
    %jump(memset)

wcopy_after:
    // stack: kexit_info
    EXIT_KERNEL

// Pre stack: kexit_info, dest_offset, offset, size
// Post stack: (empty)
global sys_calldatacopy:
    %wcopy(@SEGMENT_CALLDATA, @CTX_METADATA_CALLDATA_SIZE)

// Pre stack: kexit_info, dest_offset, offset, size
// Post stack: (empty)
global sys_returndatacopy:
    DUP4 DUP4 %add_or_fault // Overflow check
    %mload_context_metadata(@CTX_METADATA_RETURNDATA_SIZE) LT %jumpi(fault_exception) // Data len check

    %wcopy(@SEGMENT_RETURNDATA, @CTX_METADATA_RETURNDATA_SIZE)

// Pre stack: kexit_info, dest_offset, offset, size
// Post stack: (empty)
global sys_codecopy:
    // stack: kexit_info, dest_offset, offset, size
    %wcopy_charge_gas

    %stack (kexit_info, dest_offset, offset, size) -> (dest_offset, size, kexit_info, dest_offset, offset, size)
    %add_or_fault
    // stack: expanded_num_bytes, kexit_info, dest_offset, offset, size, kexit_info
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes

    GET_CONTEXT
    %mload_context_metadata(@CTX_METADATA_CODE_SIZE)
    // stack: code_size, ctx, kexit_info, dest_offset, offset, size
    %codecopy_after_checks(@SEGMENT_CODE)


// Pre stack: kexit_info, address, dest_offset, offset, size
// Post stack: (empty)
global sys_extcodecopy:
    %stack (kexit_info, address, dest_offset, offset, size)
        -> (address, dest_offset, offset, size, kexit_info)
    %u256_to_addr DUP1 %insert_accessed_addresses
    // stack: cold_access, address, dest_offset, offset, size, kexit_info
    PUSH @GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS
    MUL
    PUSH @GAS_WARMACCESS
    ADD
    // stack: Gaccess, address, dest_offset, offset, size, kexit_info

    DUP5
    // stack: size, Gaccess, address, dest_offset, offset, size, kexit_info
    ISZERO %jumpi(sys_extcodecopy_empty)

    // stack: Gaccess, address, dest_offset, offset, size, kexit_info
    DUP5 %num_bytes_to_num_words %mul_const(@GAS_COPY) ADD
    %stack (gas, address, dest_offset, offset, size, kexit_info) -> (gas, kexit_info, address, dest_offset, offset, size)
    %charge_gas

    %stack (kexit_info, address, dest_offset, offset, size) -> (dest_offset, size, kexit_info, address, dest_offset, offset, size)
    %add_or_fault
    // stack: expanded_num_bytes, kexit_info, address, dest_offset, offset, size
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes

    %next_context_id

    %stack (ctx, kexit_info, address, dest_offset, offset, size) ->
        (address, ctx, extcodecopy_contd, ctx, kexit_info, dest_offset, offset, size)
    %jump(load_code)

sys_extcodecopy_empty:
    %stack (Gaccess, address, dest_offset, offset, size, kexit_info) -> (Gaccess, kexit_info)
    %charge_gas
    EXIT_KERNEL

extcodecopy_contd:
    // stack: code_size, ctx, kexit_info, dest_offset, offset, size
    %codecopy_after_checks(@SEGMENT_CODE)


// The internal logic is similar to wcopy, but handles range overflow differently.
// It is used for both CODECOPY and EXTCODECOPY.
%macro codecopy_after_checks(segment)
    // stack: total_size, src_ctx, kexit_info, dest_offset, offset, size
    DUP1 DUP6
    // stack: offset, total_size, total_size, src_ctx, kexit_info, dest_offset, offset, size
    GT %jumpi(codecopy_large_offset)

    PUSH $segment SWAP1
    // stack: total_size, segment, src_ctx, kexit_info, dest_offset, offset, size
    DUP1 DUP8 DUP8 ADD
    // stack: offset + size, total_size, total_size, segment, src_ctx, kexit_info, dest_offset, offset, size
    LT %jumpi(codecopy_within_bounds)

    // stack: total_size, segment, src_ctx, kexit_info, dest_offset, offset, size
    DUP7 DUP7 ADD
    // stack: offset + size, total_size, segment, src_ctx, kexit_info, dest_offset, offset, size
    SUB // extra_size = offset + size - total_size
    // stack: extra_size, segment, src_ctx, kexit_info, dest_offset, offset, size
    DUP1 DUP8 SUB
    // stack: copy_size = size - extra_size, extra_size, segment, src_ctx, kexit_info, dest_offset, offset, size

    // Compute the new dest_offset after actual copies, at which we will start padding with zeroes.
    DUP1 DUP7 ADD
    // stack: new_dest_offset, copy_size, extra_size, segment, src_ctx, kexit_info, dest_offset, offset, size

    GET_CONTEXT
    %stack (context, new_dest_offset, copy_size, extra_size, segment, src_ctx, kexit_info, dest_offset, offset, size) ->
        (src_ctx, segment, offset, @SEGMENT_MAIN_MEMORY, dest_offset, context, copy_size, wcopy_large_offset, kexit_info, new_dest_offset, offset, extra_size)
    %build_address
    SWAP3 %build_address
    // stack: DST, SRC, copy_size, wcopy_large_offset, kexit_info, new_dest_offset, offset, extra_size
    %jump(memcpy_bytes)
%endmacro
