%extcodehash
    // stack: address
    %mpt_read_state_trie
    // stack: account_ptr
    %add_const(3)
    // stack: codehash_ptr
    %mload_trie_data
    // stack: codehash
%endmacro

%extcodesize
    // stack: address
    %stack (address) -> (address, %after)
    %jump(load_code)
%%after:
%endmacro

%codesize
    ADDRESS
    %extcodesize
%endmacro

global extcodecopy:
    // stack: address, dest_offset, offset, size, retdest
    %stack (address, dest_offset, offset, size, retdest) -> (address, extcodecopy_contd, size, offset, dest_offset, retdest)
    %jump(load_code)
extcodecopy_contd:
    // stack: code_length, size, offset, dest_offset, retdest
    SWAP1
    // stack: size, code_length, offset, dest_offset, retdest
    PUSH 0
extcodecopy_loop:
    // stack: i, size, code_length, offset, dest_offset, retdest
    DUP2 DUP2 EQ
    // stack: i == size, i, size, code_length, offset, dest_offset, retdest
    %jumpi(extcodecopy_end)
    %stack: (i, size, code_length, offset, dest_offset, retdest) -> (offset, code_length, offset, code_length, dest_offset, i, size, retdest)
    LT
    // stack: offset < code_length, offset, code_length, dest_offset, i, size, retdest
    DUP2
    // stack: offset, offset < code_length, offset, code_length, dest_offset, i, size, retdest
    %mload_current(@SEGMENT_KERNEL_ACCOUNT_CODE)
    // stack: opcode, offset < code_length, offset, code_length, dest_offset, i, size, retdest
    &stack (opcode, offset < code_length, offset, code_length, dest_offset, i, size, retdest) -> (offset < code_length, 0, opcode, offset, code_length, dest_offset, i, size, retdest)
    %select_bool
    // stack: opcode, offset, code_length, dest_offset, i, size, retdest
    DUP4
    // stack: dest_offset, opcode, offset, code_length, dest_offset, i, size, retdest


extcodecopy_end:
    %stack: (i, size, code_length, offset, dest_offset, size, retdest) -> (retdest)
    JUMP


load_code:
    // stack: address, retdest
    %extcodehash
    // stack: codehash, retdest
    PROVER_INPUT(account_code::length)
    // stack: code_length, codehash, retdest
    PUSH 0
load_code_loop:
    // stack: i, code_length, codehash, retdest
    DUP2 DUP2 EQ
    // stack: i == code_length, i, code_length, codehash, retdest
    %jumpi(load_code_check)
    PROVER_INPUT(account_code::get)
    // stack: opcode, i, code_length, codehash, retdest
    DUP2
    // stack: i, opcode, i, code_length, codehash, retdest
    %mstore_current(@SEGMENT_KERNEL_ACCOUNT_CODE)
    // stack: i, code_length, codehash, retdest
    %increment
    // stack: i+1, code_length, codehash, retdest
    %jump(load_code_loop)

load_code_check:
    // stack: i, code_length, codehash, retdest
    POP
    // stack: code_length, codehash, retdest
    %stack (code_length, codehash, retdest) -> (code_length, codehash, retdest, code_length)
    PUSH 0
    // stack: 0, code_length, codehash, retdest, code_length
    PUSH @SEGMENT_KERNEL_ACCOUNT_CODE
    // stack: segment, 0, code_length, codehash, retdest, code_length
    GET_CONTEXT
    // stack: context, segment, 0, code_length, codehash, retdest, code_length
    KECCAK_GENERAL
    // stack: shouldbecodehash, codehash, retdest, code_length
    %assert_eq
    JUMP

