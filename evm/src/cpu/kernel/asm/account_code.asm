retzero:
    %stack (account_ptr, retdest) -> (retdest, 0)
    JUMP

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

%macro extcodehash
    %stack (address) -> (address, %%after)
    %jump(extcodehash)
%%after:
%endmacro

%macro ext_code_empty
    %extcodehash
    %eq_const(@EMPTY_STRING_HASH)
%endmacro

%macro codesize
    // stack: (empty)
    %address
    %extcodesize
%endmacro

%macro extcodesize
    %stack (address) -> (address, 0, @SEGMENT_KERNEL_ACCOUNT_CODE, %%after)
    %jump(load_code)
%%after:
%endmacro

global extcodesize:
    // stack: address, retdest
    %extcodesize
    // stack: extcodesize(address), retdest
    SWAP1 JUMP


%macro codecopy
    // stack: dest_offset, offset, size, retdest
    %address
    // stack: address, dest_offset, offset, size, retdest
    %jump(extcodecopy)
%endmacro

// Pre stack: address, dest_offset, offset, size, retdest
// Post stack: (empty)
global extcodecopy:
    // stack: address, dest_offset, offset, size, retdest
    %stack (address, dest_offset, offset, size, retdest)
        -> (address, 0, @SEGMENT_KERNEL_ACCOUNT_CODE, extcodecopy_contd, size, offset, dest_offset, retdest)
    %jump(load_code)

extcodecopy_contd:
    // stack: code_length, size, offset, dest_offset, retdest
    SWAP1
    // stack: size, code_length, offset, dest_offset, retdest
    PUSH 0

// Loop copying the `code[offset]` to `memory[dest_offset]` until `i==size`.
// Each iteration increments `offset, dest_offset, i`.
// TODO: Consider implementing this with memcpy.
extcodecopy_loop:
    // stack: i, size, code_length, offset, dest_offset, retdest
    DUP2 DUP2 EQ
    // stack: i == size, i, size, code_length, offset, dest_offset, retdest
    %jumpi(extcodecopy_end)
    %stack (i, size, code_length, offset, dest_offset, retdest)
        -> (offset, code_length, offset, code_length, dest_offset, i, size, retdest)
    LT
    // stack: offset < code_length, offset, code_length, dest_offset, i, size, retdest
    DUP2
    // stack: offset, offset < code_length, offset, code_length, dest_offset, i, size, retdest
    %mload_current(@SEGMENT_KERNEL_ACCOUNT_CODE)
    // stack: opcode, offset < code_length, offset, code_length, dest_offset, i, size, retdest
    %stack (opcode, offset_lt_code_length, offset, code_length, dest_offset, i, size, retdest)
        -> (offset_lt_code_length, 0, opcode, offset, code_length, dest_offset, i, size, retdest)
    // If `offset >= code_length`, use `opcode=0`. Necessary since `SEGMENT_KERNEL_ACCOUNT_CODE` might be clobbered from previous calls.
    %select_bool
    // stack: opcode, offset, code_length, dest_offset, i, size, retdest
    DUP4
    // stack: dest_offset, opcode, offset, code_length, dest_offset, i, size, retdest
    %mstore_main
    // stack: offset, code_length, dest_offset, i, size, retdest
    %increment
    // stack: offset+1, code_length, dest_offset, i, size, retdest
    SWAP2
    // stack: dest_offset, code_length, offset+1, i, size, retdest
    %increment
    // stack: dest_offset+1, code_length, offset+1, i, size, retdest
    SWAP3
    // stack: i, code_length, offset+1, dest_offset+1, size, retdest
    %increment
    // stack: i+1, code_length, offset+1, dest_offset+1, size, retdest
    %stack (i, code_length, offset, dest_offset, size, retdest) -> (i, size, code_length, offset, dest_offset, retdest)
    %jump(extcodecopy_loop)

extcodecopy_end:
    %stack (i, size, code_length, offset, dest_offset, retdest) -> (retdest)
    JUMP


// Loads the code at `address` into memory, at the given context and segment, starting at offset 0.
// Checks that the hash of the loaded code corresponds to the `codehash` in the state trie.
// Pre stack: address, ctx, segment, retdest
// Post stack: code_len
global load_code:
    %stack (address, ctx, segment, retdest) -> (extcodehash, address, load_code_ctd, ctx, segment, retdest)
    JUMP
load_code_ctd:
    // stack: codehash, ctx, segment, retdest
    PROVER_INPUT(account_code::length)
    // stack: code_length, codehash, ctx, segment, retdest
    PUSH 0

// Loop non-deterministically querying `code[i]` and storing it in `SEGMENT_KERNEL_ACCOUNT_CODE` at offset `i`, until `i==code_length`.
load_code_loop:
    // stack: i, code_length, codehash, ctx, segment, retdest
    DUP2 DUP2 EQ
    // stack: i == code_length, i, code_length, codehash, ctx, segment, retdest
    %jumpi(load_code_check)
    PROVER_INPUT(account_code::get)
    // stack: opcode, i, code_length, codehash, ctx, segment, retdest
    DUP2
    // stack: i, opcode, i, code_length, codehash, ctx, segment, retdest
    DUP7 // segment
    DUP7 // context
    MSTORE_GENERAL
    // stack: i, code_length, codehash, ctx, segment, retdest
    %increment
    // stack: i+1, code_length, codehash, ctx, segment, retdest
    %jump(load_code_loop)

// Check that the hash of the loaded code equals `codehash`.
load_code_check:
    // stack: i, code_length, codehash, ctx, segment, retdest
    %stack (i, code_length, codehash, ctx, segment, retdest)
        -> (ctx, segment, 0, code_length, codehash, retdest, code_length)
    KECCAK_GENERAL
    // stack: shouldbecodehash, codehash, retdest, code_length
    %assert_eq
    JUMP
