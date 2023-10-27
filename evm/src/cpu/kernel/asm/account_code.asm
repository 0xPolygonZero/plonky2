global sys_extcodehash:
    // stack: kexit_info, address
    SWAP1 %u256_to_addr
    // stack: address, kexit_info
    DUP1 %insert_accessed_addresses
    // stack: cold_access, address, kexit_info
    PUSH @GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS
    MUL
    PUSH @GAS_WARMACCESS
    ADD
    %stack (gas, address, kexit_info) -> (gas, kexit_info, address)
    %charge_gas
    // stack: kexit_info, address

    SWAP1
    DUP1 %is_dead %jumpi(extcodehash_dead)
    %extcodehash
    // stack: hash, kexit_info
    SWAP1
    EXIT_KERNEL
extcodehash_dead:
    %stack (address, kexit_info) -> (kexit_info, 0)
    EXIT_KERNEL

global extcodehash:
    // stack: address, retdest
    %mpt_read_state_trie
    // stack: account_ptr, retdest
    DUP1 ISZERO %jumpi(retzero)
    %add_const(3)
    // stack: codehash_ptr, retdest
    %mload_trie_data
    // stack: codehash, retdest
    SWAP1 JUMP
retzero:
    %stack (account_ptr, retdest) -> (retdest, 0)
    JUMP

%macro extcodehash
    %stack (address) -> (address, %%after)
    %jump(extcodehash)
%%after:
%endmacro

%macro ext_code_empty
    %extcodehash
    %eq_const(@EMPTY_STRING_HASH)
%endmacro

%macro extcodesize
    %stack (address) -> (address, 0, @SEGMENT_KERNEL_ACCOUNT_CODE, %%after)
    %jump(load_code)
%%after:
%endmacro

global sys_extcodesize:
    // stack: kexit_info, address
    SWAP1 %u256_to_addr
    // stack: address, kexit_info
    DUP1 %insert_accessed_addresses
    // stack: cold_access, address, kexit_info
    PUSH @GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS
    MUL
    PUSH @GAS_WARMACCESS
    ADD
    %stack (gas, address, kexit_info) -> (gas, kexit_info, address)
    %charge_gas
    // stack: kexit_info, address

    SWAP1
    // stack: address, kexit_info
    %extcodesize
    // stack: code_size, kexit_info
    SWAP1
    EXIT_KERNEL

global extcodesize:
    // stack: address, retdest
    %extcodesize
    // stack: extcodesize(address), retdest
    SWAP1 JUMP

%macro extcodecopy
    // stack: address, dest_offset, offset, size
    %stack (address, dest_offset, offset, size) -> (address, dest_offset, offset, size, %%after)
    %jump(extcodecopy)
%%after:
%endmacro

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

    %stack (kexit_info, address, dest_offset, offset, size) -> (address, dest_offset, offset, size, kexit_info)
    %extcodecopy
    // stack: kexit_info
    EXIT_KERNEL

sys_extcodecopy_empty:
    %stack (Gaccess, address, dest_offset, offset, size, kexit_info) -> (Gaccess, kexit_info)
    %charge_gas
    EXIT_KERNEL


// Pre stack: address, dest_offset, offset, size, retdest
// Post stack: (empty)
global extcodecopy:
    // stack: address, dest_offset, offset, size, retdest
    %stack (address, dest_offset, offset, size, retdest)
        -> (address, 0, @SEGMENT_KERNEL_ACCOUNT_CODE, extcodecopy_contd, size, offset, dest_offset, retdest)
    %jump(load_code)

extcodecopy_contd:
    // stack: code_size, size, offset, dest_offset, retdest
    DUP1 DUP4
    // stack: offset, code_size, code_size, size, offset, dest_offset, retdest
    GT %jumpi(extcodecopy_large_offset)

    // stack: code_size, size, offset, dest_offset, retdest
    DUP3 DUP3 ADD
    // stack: offset + size, code_size, size, offset, dest_offset, retdest
    DUP2 GT %jumpi(extcodecopy_within_bounds)

    // stack: code_size, size, offset, dest_offset, retdest
    DUP3 DUP3 ADD
    // stack: offset + size, code_size, size, offset, dest_offset, retdest
    SUB
    // stack: extra_size = offset + size - code_size, size, offset, dest_offset, retdest
    DUP1 DUP3 SUB
    // stack: copy_size = size - extra_size, extra_size, size, offset, dest_offset, retdest

    // Compute the new dest_offset after actual copies, at which we will start padding with zeroes.
    DUP1 DUP6 ADD
    // stack: new_dest_offset, copy_size, extra_size, size, offset, dest_offset, retdest

    GET_CONTEXT
    %stack (context, new_dest_offset, copy_size, extra_size, size, offset, dest_offset, retdest) ->
        (context, @SEGMENT_MAIN_MEMORY, dest_offset, 0, @SEGMENT_KERNEL_ACCOUNT_CODE, offset, copy_size, extcodecopy_end, new_dest_offset, extra_size, retdest)
    %jump(memcpy_bytes)

extcodecopy_within_bounds:
    // stack: code_size, size, offset, dest_offset, retdest
    GET_CONTEXT
    %stack (context, code_size, size, offset, dest_offset, retdest) ->
        (context, @SEGMENT_MAIN_MEMORY, dest_offset, 0, @SEGMENT_KERNEL_ACCOUNT_CODE, offset, size, retdest)
    %jump(memcpy_bytes)

// Same as extcodecopy_large_offset, but without `offset` in the stack.
extcodecopy_end:
    // stack: dest_offset, size, retdest
    GET_CONTEXT
    %stack (context, dest_offset, size, retdest) ->
        (context, @SEGMENT_MAIN_MEMORY, dest_offset, size, retdest)
    %jump(memset)

extcodecopy_large_offset:
    // offset is larger than the code size. So we just have to write zeros.
    // stack: code_size, size, offset, dest_offset, retdest
    GET_CONTEXT
    %stack (context, code_size, size, offset, dest_offset, retdest) -> (context, @SEGMENT_MAIN_MEMORY, dest_offset, size, retdest)
    %jump(memset)

// Loads the code at `address` into memory, at the given context and segment, starting at offset 0.
// Checks that the hash of the loaded code corresponds to the `codehash` in the state trie.
// Pre stack: address, ctx, segment, retdest
// Post stack: code_size
global load_code:
    %stack (address, ctx, segment, retdest) -> (extcodehash, address, load_code_ctd, ctx, segment, retdest)
    JUMP
load_code_ctd:
    // stack: codehash, ctx, segment, retdest
    DUP1 ISZERO %jumpi(load_code_non_existent_account)
    PROVER_INPUT(account_code::length)
    // stack: code_size, codehash, ctx, segment, retdest
    PUSH 0

// Loop non-deterministically querying `code[i]` and storing it in `SEGMENT_KERNEL_ACCOUNT_CODE`
// at offset `i`, until `i==code_size`.
load_code_loop:
    // stack: i, code_size, codehash, ctx, segment, retdest
    DUP2 DUP2 EQ
    // stack: i == code_size, i, code_size, codehash, ctx, segment, retdest
    %jumpi(load_code_check)
    PROVER_INPUT(account_code::get)
    // stack: opcode, i, code_size, codehash, ctx, segment, retdest
    DUP2
    // stack: i, opcode, i, code_size, codehash, ctx, segment, retdest
    DUP7 // segment
    DUP7 // context
    MSTORE_GENERAL
    // stack: i, code_size, codehash, ctx, segment, retdest
    %increment
    // stack: i+1, code_size, codehash, ctx, segment, retdest
    %jump(load_code_loop)

// Check that the hash of the loaded code equals `codehash`.
load_code_check:
    // stack: i, code_size, codehash, ctx, segment, retdest
    %stack (i, code_size, codehash, ctx, segment, retdest)
        -> (ctx, segment, 0, code_size, codehash, retdest, code_size)
    KECCAK_GENERAL
    // stack: shouldbecodehash, codehash, retdest, code_size
    %assert_eq
    JUMP

load_code_non_existent_account:
    %stack (codehash, ctx, segment, retdest) -> (retdest, 0)
    JUMP
