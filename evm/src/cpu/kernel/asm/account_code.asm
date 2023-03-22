global sys_extcodehash:
    // stack: kexit_info, address
    // TODO: Charge gas.
    SWAP1
    // stack: address, kexit_info
    %extcodehash
    // stack: hash, kexit_info
    SWAP1
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
    // TODO: Charge gas.
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
    // TODO: Call %update_mem_bytes to expand memory.
    // TODO: Charge other gas.
    %stack (kexit_info, address, dest_offset, offset, size)
        -> (address, dest_offset, offset, size, kexit_info)
    %extcodecopy
    // stack: kexit_info
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
    SWAP1
    // stack: size, code_size, offset, dest_offset, retdest
    PUSH 0

// Loop copying the `code[offset]` to `memory[dest_offset]` until `i==size`.
// Each iteration increments `offset, dest_offset, i`.
// TODO: Consider implementing this with memcpy.
extcodecopy_loop:
    // stack: i, size, code_size, offset, dest_offset, retdest
    DUP2 DUP2 EQ
    // stack: i == size, i, size, code_size, offset, dest_offset, retdest
    %jumpi(extcodecopy_end)
    %stack (i, size, code_size, offset, dest_offset, retdest)
        -> (offset, code_size, offset, code_size, dest_offset, i, size, retdest)
    LT
    // stack: offset < code_size, offset, code_size, dest_offset, i, size, retdest
    DUP2
    // stack: offset, offset < code_size, offset, code_size, dest_offset, i, size, retdest
    %mload_current(@SEGMENT_KERNEL_ACCOUNT_CODE)
    // stack: opcode, offset < code_size, offset, code_size, dest_offset, i, size, retdest
    %stack (opcode, offset_lt_code_size, offset, code_size, dest_offset, i, size, retdest)
        -> (offset_lt_code_size, 0, opcode, offset, code_size, dest_offset, i, size, retdest)
    // If `offset >= code_size`, use `opcode=0`. Necessary since `SEGMENT_KERNEL_ACCOUNT_CODE` might be clobbered from previous calls.
    %select_bool
    // stack: opcode, offset, code_size, dest_offset, i, size, retdest
    DUP4
    // stack: dest_offset, opcode, offset, code_size, dest_offset, i, size, retdest
    %mstore_current(@SEGMENT_MAIN_MEMORY)
    // stack: offset, code_size, dest_offset, i, size, retdest
    %increment
    // stack: offset+1, code_size, dest_offset, i, size, retdest
    SWAP2
    // stack: dest_offset, code_size, offset+1, i, size, retdest
    %increment
    // stack: dest_offset+1, code_size, offset+1, i, size, retdest
    SWAP3
    // stack: i, code_size, offset+1, dest_offset+1, size, retdest
    %increment
    // stack: i+1, code_size, offset+1, dest_offset+1, size, retdest
    %stack (i, code_size, offset, dest_offset, size, retdest) -> (i, size, code_size, offset, dest_offset, retdest)
    %jump(extcodecopy_loop)

extcodecopy_end:
    %stack (i, size, code_size, offset, dest_offset, retdest) -> (retdest)
    JUMP


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
