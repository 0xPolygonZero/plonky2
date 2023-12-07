// Methods for encoding integers as bytes in memory, as well as the reverse,
// decoding bytes as integers. All big-endian.

// Given a pointer to some bytes in memory, pack them into a word. Assumes 0 < len <= 32.
// Pre stack: addr: 3, len, retdest
// Post stack: packed_value
// NOTE: addr: 3 denotes a (context, segment, virtual) tuple
global mload_packing:
    // stack: addr: 3, len, retdest
    MLOAD_32BYTES
    // stack: packed_value, retdest
    SWAP1
    // stack: retdest, packed_value
    JUMP

%macro mload_packing
    %stack (addr: 3, len) -> (addr, len, %%after)
    %jump(mload_packing)
%%after:
%endmacro

global mload_packing_u64_LE:
    // stack: context, segment, offset, retdest
    DUP3                DUP3 DUP3 MLOAD_GENERAL
    DUP4 INCREMENT  DUP4 DUP4 MLOAD_GENERAL %shl_const( 8) ADD
    DUP4 %add_const(2)  DUP4 DUP4 MLOAD_GENERAL %shl_const(16) ADD
    DUP4 %add_const(3)  DUP4 DUP4 MLOAD_GENERAL %shl_const(24) ADD
    DUP4 %add_const(4)  DUP4 DUP4 MLOAD_GENERAL %shl_const(32) ADD
    DUP4 %add_const(5)  DUP4 DUP4 MLOAD_GENERAL %shl_const(40) ADD
    DUP4 %add_const(6)  DUP4 DUP4 MLOAD_GENERAL %shl_const(48) ADD
    DUP4 %add_const(7)  DUP4 DUP4 MLOAD_GENERAL %shl_const(56) ADD
    %stack (value, context, segment, offset, retdest) -> (retdest, value)
    JUMP

%macro mload_packing_u64_LE
    %stack (addr: 3) -> (addr, %%after)
    %jump(mload_packing_u64_LE)
%%after:
%endmacro

// Pre stack: context, segment, offset, value, len, retdest
// Post stack: offset'
global mstore_unpacking:
    // stack: context, segment, offset, value, len, retdest
    DUP5 ISZERO
    // stack: len == 0, context, segment, offset, value, len, retdest
    %jumpi(mstore_unpacking_empty)
    %stack(context, segment, offset, value, len, retdest) -> (len, context, segment, offset, value, retdest)
    PUSH 3
    // stack: BYTES_PER_JUMP, len, context, segment, offset, value, retdest
    MUL
    // stack: jump_offset, context, segment, offset, value, retdest
    PUSH mstore_unpacking_0
    // stack: mstore_unpacking_0, jump_offset, context, segment, offset, value, retdest
    ADD
    // stack: address_unpacking, context, segment, offset, value, retdest
    JUMP

mstore_unpacking_empty:
    %stack(context, segment, offset, value, len, retdest) -> (retdest, offset)
    JUMP

// This case can never be reached. It's only here to offset the table correctly.
mstore_unpacking_0:
    %rep 3
        PANIC
    %endrep
mstore_unpacking_1:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_1
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_2:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_2
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_3:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_3
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_4:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_4
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_5:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_5
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_6:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_6
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_7:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_7
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_8:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_8
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_9:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_9
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_10:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_10
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_11:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_11
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_12:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_12
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_13:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_13
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_14:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_14
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_15:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_15
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_16:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_16
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_17:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_17
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_18:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_18
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_19:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_19
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_20:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_20
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_21:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_21
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_22:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_22
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_23:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_23
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_24:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_24
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_25:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_25
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_26:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_26
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_27:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_27
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_28:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_28
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_29:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_29
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_30:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_30
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_31:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_31
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP
mstore_unpacking_32:
    // stack: context, segment, offset, value, retdest
    MSTORE_32BYTES_32
    // stack: offset', retdest
    SWAP1
    // stack: retdest, offset'
    JUMP

%macro mstore_unpacking
    %stack (addr: 3, value, len) -> (addr, value, len, %%after)
    %jump(mstore_unpacking)
%%after:
%endmacro

// Pre stack: context, segment, offset, value, retdest
// Post stack: offset'
global mstore_unpacking_u64_LE:
    %stack (context, segment, offset, value) -> (0xff, value, context, segment, offset, context, segment, offset, value)
    AND
    MSTORE_GENERAL // First byte
    DUP3 INCREMENT
    %stack (new_offset, context, segment, offset, value) -> (0xff00, value, context, segment, new_offset, context, segment, offset, value)
    AND %shr_const(8)
    MSTORE_GENERAL // Second byte
    DUP3 %add_const(2)
    %stack (new_offset, context, segment, offset, value) -> (0xff0000, value, context, segment, new_offset, context, segment, offset, value)
    AND %shr_const(16)
    MSTORE_GENERAL // Third byte
    DUP3 %add_const(3)
    %stack (new_offset, context, segment, offset, value) -> (0xff000000, value, context, segment, new_offset, context, segment, offset, value)
    AND %shr_const(24)
    MSTORE_GENERAL // Fourth byte
    DUP3 %add_const(4)
    %stack (new_offset, context, segment, offset, value) -> (0xff00000000, value, context, segment, new_offset, context, segment, offset, value)
    AND %shr_const(32)
    MSTORE_GENERAL // Fifth byte
    DUP3 %add_const(5)
    %stack (new_offset, context, segment, offset, value) -> (0xff0000000000, value, context, segment, new_offset, context, segment, offset, value)
    AND %shr_const(40)
    MSTORE_GENERAL // Sixth byte
    DUP3 %add_const(6)
    %stack (new_offset, context, segment, offset, value) -> (0xff000000000000, value, context, segment, new_offset, context, segment, offset, value)
    AND %shr_const(48)
    MSTORE_GENERAL // Seventh byte
    DUP3 %add_const(7)
    %stack (new_offset, context, segment, offset, value) -> (0xff00000000000000, value, context, segment, new_offset, context, segment, offset, value)
    AND %shr_const(56)
    MSTORE_GENERAL // Eighth byte
    %pop4 JUMP

%macro mstore_unpacking_u64_LE
    %stack (addr: 3, value) -> (addr, value, %%after)
    %jump(mstore_unpacking_u64_LE)
%%after:
%endmacro
