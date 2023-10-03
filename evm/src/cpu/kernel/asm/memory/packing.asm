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
    DUP4 %add_const(1)  DUP4 DUP4 MLOAD_GENERAL %shl_const( 8) ADD
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
    %stack(context, segment, offset, value, len, retdest) -> (context, segment, offset, value, len, offset, len, retdest)
    // stack: context, segment, offset, value, len, offset, len, retdest
    MSTORE_32BYTES
    // stack: offset, len, retdest
    ADD SWAP1
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
    %stack (context, segment, offset, value) -> (0xff, value, context, segment, offset, value)
    AND
    DUP4 DUP4 DUP4 MSTORE_GENERAL // First byte
    %stack (context, segment, offset, value) -> (0xff00, value, context, segment, offset, value)
    AND %shr_const(8)
    DUP4 %add_const(1) DUP4 DUP4 MSTORE_GENERAL // Second byte
    %stack (context, segment, offset, value) -> (0xff0000, value, context, segment, offset, value)
    AND %shr_const(16)
    DUP4 %add_const(2) DUP4 DUP4 MSTORE_GENERAL // Third byte
    %stack (context, segment, offset, value) -> (0xff000000, value, context, segment, offset, value)
    AND %shr_const(24)
    DUP4 %add_const(3) DUP4 DUP4 MSTORE_GENERAL // Fourth byte
    %stack (context, segment, offset, value) -> (0xff00000000, value, context, segment, offset, value)
    AND %shr_const(32)
    DUP4 %add_const(4) DUP4 DUP4 MSTORE_GENERAL // Fifth byte
    %stack (context, segment, offset, value) -> (0xff0000000000, value, context, segment, offset, value)
    AND %shr_const(40)
    DUP4 %add_const(5) DUP4 DUP4 MSTORE_GENERAL // Sixth byte
    %stack (context, segment, offset, value) -> (0xff000000000000, value, context, segment, offset, value)
    AND %shr_const(48)
    DUP4 %add_const(6) DUP4 DUP4 MSTORE_GENERAL // Seventh byte
    %stack (context, segment, offset, value) -> (0xff00000000000000, value, context, segment, offset, value)
    AND %shr_const(56)
    DUP4 %add_const(7) DUP4 DUP4 MSTORE_GENERAL // Eighth byte
    %pop4 JUMP

%macro mstore_unpacking_u64_LE
    %stack (addr: 3, value) -> (addr, value, %%after)
    %jump(mstore_unpacking_u64_LE)
%%after:
%endmacro
