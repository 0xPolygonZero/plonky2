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
    // stack: addr: 3, len, kexit_info
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
    %stack(kexit_info, offset, value) -> (offset, value, 32, kexit_info)
    PUSH @SEGMENT_MAIN_MEMORY
    GET_CONTEXT
    // stack: addr: 3, value, len, kexit_info
    MSTORE_32BYTES
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
    // stack: ADDR: 3, 32, sys_calldataload_after_mload_packing, kexit_info
    %jump(mload_packing)
sys_calldataload_after_mload_packing:
    // stack: value, kexit_info
    SWAP1
    EXIT_KERNEL
    PANIC
calldataload_large_offset:
    %stack (kexit_info, i) -> (kexit_info, 0)
    EXIT_KERNEL

// Macro for {CALLDATA,CODE,RETURNDATA}COPY (W_copy in Yellow Paper).
%macro wcopy(segment, context_metadata_size)
    // stack: kexit_info, dest_offset, offset, size
    PUSH @GAS_VERYLOW
    DUP5
    // stack: size, Gverylow, kexit_info, dest_offset, offset, size
    ISZERO %jumpi(wcopy_empty)
    // stack: Gverylow, kexit_info, dest_offset, offset, size
    DUP5 %num_bytes_to_num_words %mul_const(@GAS_COPY) ADD %charge_gas

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

    PUSH $segment
    %mload_context_metadata($context_metadata_size)
    // stack: total_size, segment, kexit_info, dest_offset, offset, size
    DUP6 DUP6 ADD
    // stack: offset + size, total_size, segment, kexit_info, dest_offset, offset, size
    LT %jumpi(wcopy_within_bounds)

    %mload_context_metadata($context_metadata_size)
    // stack: total_size, segment, kexit_info, dest_offset, offset, size
    DUP6 DUP6 ADD
    // stack: offset + size, total_size, segment, kexit_info, dest_offset, offset, size
    SUB // extra_size = offset + size - total_size
    // stack: extra_size, segment, kexit_info, dest_offset, offset, size
    DUP1 DUP7 SUB
    // stack: copy_size = size - extra_size, extra_size, segment, kexit_info, dest_offset, offset, size

    // Compute the new dest_offset after actual copies, at which we will start padding with zeroes.
    DUP1 DUP6 ADD
    // stack: new_dest_offset, copy_size, extra_size, segment, kexit_info, dest_offset, offset, size

    GET_CONTEXT
    %stack (context, new_dest_offset, copy_size, extra_size, segment, kexit_info, dest_offset, offset, size) ->
        (context, @SEGMENT_MAIN_MEMORY, dest_offset, context, segment, offset, copy_size, wcopy_over_range, new_dest_offset, extra_size, kexit_info)
    %jump(memcpy_bytes)
%endmacro

wcopy_within_bounds:
    // stack: segment, kexit_info, dest_offset, offset, size
    GET_CONTEXT
    %stack (context, segment, kexit_info, dest_offset, offset, size) ->
        (context, @SEGMENT_MAIN_MEMORY, dest_offset, context, segment, offset, size, wcopy_after, kexit_info)
    %jump(memcpy_bytes)


// Same as wcopy_large_offset, but without `offset` in the stack.
wcopy_over_range:
    // stack: dest_offset, size, kexit_info
    GET_CONTEXT
    %stack (context, dest_offset, size, kexit_info) ->
        (context, @SEGMENT_MAIN_MEMORY, dest_offset, size, wcopy_after, kexit_info)
    %jump(memset)

wcopy_empty:
    // stack: Gverylow, kexit_info, dest_offset, offset, size
    %charge_gas
    %stack (kexit_info, dest_offset, offset, size) -> (kexit_info)
    EXIT_KERNEL

wcopy_large_offset:
    // offset is larger than the size of the {CALLDATA,CODE,RETURNDATA}. So we just have to write zeros.
    // stack: kexit_info, dest_offset, offset, size
    GET_CONTEXT
    %stack (context, kexit_info, dest_offset, offset, size) ->
        (context, @SEGMENT_MAIN_MEMORY, dest_offset, size, wcopy_after, kexit_info)
    %jump(memset)

wcopy_after:
    // stack: kexit_info
    EXIT_KERNEL

global sys_calldatacopy:
    %wcopy(@SEGMENT_CALLDATA, @CTX_METADATA_CALLDATA_SIZE)

global sys_codecopy:
    %wcopy(@SEGMENT_CODE, @CTX_METADATA_CODE_SIZE)

// Same as %wcopy but with overflow checks.
global sys_returndatacopy:
    // stack: kexit_info, dest_offset, offset, size
    PUSH @GAS_VERYLOW
    // stack: Gverylow, kexit_info, dest_offset, offset, size
    DUP5 %num_bytes_to_num_words %mul_const(@GAS_COPY) ADD %charge_gas

    %stack (kexit_info, dest_offset, offset, size) -> (dest_offset, size, kexit_info, dest_offset, offset, size)
    %add_or_fault
    // stack: expanded_num_bytes, kexit_info, dest_offset, offset, size, kexit_info
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
    // stack: kexit_info, dest_offset, offset, size, kexit_info
    DUP4 DUP4 %add_or_fault // Overflow check
    %mload_context_metadata(@CTX_METADATA_RETURNDATA_SIZE) LT %jumpi(fault_exception) // Data len check

    // stack:  kexit_info, dest_offset, offset, size
    DUP4
    // stack:  size, kexit_info, dest_offset, offset, size
    ISZERO %jumpi(returndatacopy_empty)

    %mload_context_metadata(@CTX_METADATA_RETURNDATA_SIZE)
    // stack: total_size, kexit_info, dest_offset, offset, size
    DUP4
    // stack: offset, total_size, kexit_info, dest_offset, offset, size
    GT %jumpi(wcopy_large_offset)

    PUSH @SEGMENT_RETURNDATA
    %mload_context_metadata(@CTX_METADATA_RETURNDATA_SIZE)
    // stack: total_size, returndata_segment, kexit_info, dest_offset, offset, size
    DUP6 DUP6 ADD
    // stack: offset + size, total_size, returndata_segment, kexit_info, dest_offset, offset, size
    LT %jumpi(wcopy_within_bounds)

    %mload_context_metadata(@CTX_METADATA_RETURNDATA_SIZE)
    // stack: total_size, returndata_segment, kexit_info, dest_offset, offset, size
    DUP6 DUP6 ADD
    // stack: offset + size, total_size, returndata_segment, kexit_info, dest_offset, offset, size
    SUB // extra_size = offset + size - total_size
    // stack: extra_size, returndata_segment, kexit_info, dest_offset, offset, size
    DUP1 DUP7 SUB
    // stack: copy_size = size - extra_size, extra_size, returndata_segment, kexit_info, dest_offset, offset, size

    // Compute the new dest_offset after actual copies, at which we will start padding with zeroes.
    DUP1 DUP6 ADD
    // stack: new_dest_offset, copy_size, extra_size, returndata_segment, kexit_info, dest_offset, offset, size

    GET_CONTEXT
    %stack (context, new_dest_offset, copy_size, extra_size, returndata_segment, kexit_info, dest_offset, offset, size) ->
        (context, @SEGMENT_MAIN_MEMORY, dest_offset, context, returndata_segment, offset, copy_size, wcopy_over_range, new_dest_offset, extra_size, kexit_info)
    %jump(memcpy_bytes)

returndatacopy_empty:
    %stack (kexit_info, dest_offset, offset, size) -> (kexit_info)
    EXIT_KERNEL

// Same as %wcopy but with special handling in case of overlapping ranges.
global sys_mcopy:
    // stack: kexit_info, dest_offset, offset, size
    PUSH @GAS_VERYLOW
    // stack: Gverylow, kexit_info, dest_offset, offset, size
    DUP5 %num_bytes_to_num_words %mul_const(@GAS_COPY) ADD %charge_gas

    %stack (kexit_info, dest_offset, offset, size) -> (dest_offset, size, kexit_info, dest_offset, offset, size)
    %add_or_fault
    // stack: expanded_num_bytes, kexit_info, dest_offset, offset, size, kexit_info
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes

    // stack:  kexit_info, dest_offset, offset, size
    DUP4
    // stack:  size, kexit_info, dest_offset, offset, size
    ISZERO %jumpi(returndatacopy_empty) // If size is empty, just pop the stack and exit the kernel

    // stack:  kexit_info, dest_offset, offset, size
    DUP3 DUP3 EQ
    // stack:  dest_offset = offset, kexit_info, dest_offset, offset, size
    %jumpi(returndatacopy_empty) // If SRC == DST, just pop the stack and exit the kernel

    // stack: kexit_info, dest_offset, offset, size
    PUSH @SEGMENT_MAIN_MEMORY
    DUP5 DUP5 ADD
    // stack: offset + size, segment, kexit_info, dest_offset, offset, size
    DUP4 LT
    // stack: dest_offset < offset + size, segment, kexit_info, dest_offset, offset, size
    DUP5 DUP5 GT
    // stack: dest_offset > offset, dest_offset < offset + size, segment, kexit_info, dest_offset, offset, size
    AND
    // stack: (dest_offset > offset) && (dest_offset < offset + size), segment, kexit_info, dest_offset, offset, size

    // If both conditions are satisfied, that means we will get an overlap, in which case we need to process the copy
    // in two chunks to prevent overwriting memory data before reading it.
    %jumpi(mcopy_with_overlap)

    // stack: segment, kexit_info, dest_offset, offset, size
    PUSH wcopy_within_bounds
    JUMP

mcopy_with_overlap:
    // We do have an overlap between the SRC and DST ranges. We will first copy the overlapping segment
    // (i.e. end of the copy portion), then copy the remaining (i.e. beginning) portion. 

    // stack: segment, kexit_info, dest_offset, offset, size
    DUP4 DUP4 SUB
    // stack: remaining_size = dest_offset - offset, segment, kexit_info, dest_offset, offset, size
    DUP1 DUP7
    SUB // overlapping_size = size - remaining_size
    // stack: overlapping_size, remaining_size, segment, kexit_info, dest_offset, offset, size

    // Shift the initial offsets to copy the overlapping segment first.
    DUP2 DUP7 ADD
    // stack: offset_first_copy, overlapping_size, remaining_size, segment, kexit_info, dest_offset, offset, size
    DUP3 DUP7 ADD
    // stack: dest_offset_first_copy, offset_first_copy, overlapping_size, remaining_size, segment, kexit_info, dest_offset, offset, size

    GET_CONTEXT
    %stack (context, dest_offset_first_copy, offset_first_copy, overlapping_size, remaining_size, segment, kexit_info, dest_offset, offset, size) ->
        (context, segment, dest_offset_first_copy, context, segment, offset_first_copy, overlapping_size, wcopy_within_bounds, segment, kexit_info, dest_offset, offset, remaining_size)
    %jump(memcpy_bytes)
