// Methods for encoding integers as bytes in memory, as well as the reverse,
// decoding bytes as integers. All big-endian.

// Given a pointer to some bytes in memory, pack them into a word. Assumes 0 < len <= 32.
// Pre stack: addr, len, retdest
// Post stack: packed_value
global mload_packing:
    // stack: addr, len, retdest
    MLOAD_32BYTES
    // stack: packed_value, retdest
    SWAP1
    // stack: retdest, packed_value
    JUMP

%macro mload_packing
    %stack (addr, len) -> (addr, len, %%after)
    %jump(mload_packing)
%%after:
%endmacro

global mload_packing_u64_LE:
    // stack: addr, retdest
    DUP1                MLOAD_GENERAL
    DUP2 %add_const(1)  MLOAD_GENERAL %shl_const( 8) ADD
    DUP2 %add_const(2)  MLOAD_GENERAL %shl_const(16) ADD
    DUP2 %add_const(3)  MLOAD_GENERAL %shl_const(24) ADD
    DUP2 %add_const(4)  MLOAD_GENERAL %shl_const(32) ADD
    DUP2 %add_const(5)  MLOAD_GENERAL %shl_const(40) ADD
    DUP2 %add_const(6)  MLOAD_GENERAL %shl_const(48) ADD
    DUP2 %add_const(7)  MLOAD_GENERAL %shl_const(56) ADD
    %stack (value, addr, retdest) -> (retdest, value)
    JUMP

%macro mload_packing_u64_LE
    %stack (addr) -> (addr, %%after)
    %jump(mload_packing_u64_LE)
%%after:
%endmacro

// Pre stack: addr, value, len, retdest
// Post stack: addr'
global mstore_unpacking:
    // stack: addr, value, len, retdest
    %stack(addr, value, len, retdest) -> (addr, value, len, addr, len, retdest)
    // stack: addr, value, len, addr, len, retdest
    MSTORE_32BYTES
    // stack: addr, len, retdest
    ADD SWAP1
    // stack: retdest, addr'
    JUMP

%macro mstore_unpacking
    %stack (addr, value, len) -> (addr, value, len, %%after)
    %jump(mstore_unpacking)
%%after:
%endmacro

// Pre stack: addr, value, retdest
// Post stack: addr'
global mstore_unpacking_u64_LE:
    %stack (addr, value) -> (0xff, value, addr, value)
    AND
    DUP2 MSTORE_GENERAL // First byte
    %stack (addr, value) -> (0xff00, value, addr, value)
    AND %shr_const(8)
    DUP2 %add_const(1) MSTORE_GENERAL // Second byte
    %stack (addr, value) -> (0xff0000, value, addr, value)
    AND %shr_const(16)
    DUP2 %add_const(2) MSTORE_GENERAL // Third byte
    %stack (addr, value) -> (0xff000000, value, addr, value)
    AND %shr_const(24)
    DUP2 %add_const(3) MSTORE_GENERAL // Fourth byte
    %stack (addr, value) -> (0xff00000000, value, addr, value)
    AND %shr_const(32)
    DUP2 %add_const(4) MSTORE_GENERAL // Fifth byte
    %stack (addr, value) -> (0xff0000000000, value, addr, value)
    AND %shr_const(40)
    DUP2 %add_const(5) MSTORE_GENERAL // Sixth byte
    %stack (addr, value) -> (0xff000000000000, value, addr, value)
    AND %shr_const(48)
    DUP2 %add_const(6) MSTORE_GENERAL // Seventh byte
    %stack (addr, value) -> (0xff00000000000000, value, addr, value)
    AND %shr_const(56)
    DUP2 %add_const(7) MSTORE_GENERAL // Eighth byte
    %pop2 JUMP

%macro mstore_unpacking_u64_LE
    %stack (addr, value) -> (addr, value, %%after)
    %jump(mstore_unpacking_u64_LE)
%%after:
%endmacro
