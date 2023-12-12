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
    DUP3 ISZERO
    // stack: len == 0, addr, value, len, retdest
    %jumpi(mstore_unpacking_empty)
    %stack(addr, value, len, retdest) -> (len, addr, value, retdest)
    PUSH 3
    // stack: BYTES_PER_JUMP, len, addr, value, retdest
    MUL
    // stack: jump_offset, addr, value, retdest
    PUSH mstore_unpacking_0
    // stack: mstore_unpacking_0, jump_offset, addr, value, retdest
    ADD
    // stack: address_unpacking, addr, value, retdest
    JUMP

mstore_unpacking_empty:
    %stack(addr, value, len, retdest) -> (retdest, addr)
    JUMP

// This case can never be reached. It's only here to offset the table correctly.
mstore_unpacking_0:
    %rep 3
        PANIC
    %endrep
mstore_unpacking_1:
    // stack: addr, value, retdest
    MSTORE_32BYTES_1
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_2:
    // stack: addr, value, retdest
    MSTORE_32BYTES_2
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_3:
    // stack: addr, value, retdest
    MSTORE_32BYTES_3
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_4:
    // stack: addr, value, retdest
    MSTORE_32BYTES_4
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_5:
    // stack: addr, value, retdest
    MSTORE_32BYTES_5
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_6:
    // stack: addr, value, retdest
    MSTORE_32BYTES_6
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_7:
    // stack: addr, value, retdest
    MSTORE_32BYTES_7
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_8:
    // stack: addr, value, retdest
    MSTORE_32BYTES_8
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_9:
    // stack: addr, value, retdest
    MSTORE_32BYTES_9
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_10:
    // stack: addr, value, retdest
    MSTORE_32BYTES_10
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_11:
    // stack: addr, value, retdest
    MSTORE_32BYTES_11
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_12:
    // stack: addr, value, retdest
    MSTORE_32BYTES_12
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_13:
    // stack: addr, value, retdest
    MSTORE_32BYTES_13
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_14:
    // stack: addr, value, retdest
    MSTORE_32BYTES_14
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_15:
    // stack: addr, value, retdest
    MSTORE_32BYTES_15
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_16:
    // stack: addr, value, retdest
    MSTORE_32BYTES_16
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_17:
    // stack: addr, value, retdest
    MSTORE_32BYTES_17
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_18:
    // stack: addr, value, retdest
    MSTORE_32BYTES_18
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_19:
    // stack: addr, value, retdest
    MSTORE_32BYTES_19
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_20:
    // stack: addr, value, retdest
    MSTORE_32BYTES_20
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_21:
    // stack: addr, value, retdest
    MSTORE_32BYTES_21
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_22:
    // stack: addr, value, retdest
    MSTORE_32BYTES_22
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_23:
    // stack: addr, value, retdest
    MSTORE_32BYTES_23
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_24:
    // stack: addr, value, retdest
    MSTORE_32BYTES_24
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_25:
    // stack: addr, value, retdest
    MSTORE_32BYTES_25
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_26:
    // stack: addr, value, retdest
    MSTORE_32BYTES_26
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_27:
    // stack: addr, value, retdest
    MSTORE_32BYTES_27
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_28:
    // stack: addr, value, retdest
    MSTORE_32BYTES_28
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_29:
    // stack: addr, value, retdest
    MSTORE_32BYTES_29
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_30:
    // stack: addr, value, retdest
    MSTORE_32BYTES_30
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_31:
    // stack: addr, value, retdest
    MSTORE_32BYTES_31
    // stack: addr', retdest
    SWAP1
    // stack: retdest, addr'
    JUMP
mstore_unpacking_32:
    // stack: addr, value, retdest
    MSTORE_32BYTES_32
    // stack: addr', retdest
    SWAP1
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
    %stack (addr, value) -> (0xff, value, addr, addr, value)
    AND
    MSTORE_GENERAL // First byte
    DUP1 %add_const(1)
    %stack (new_addr, addr, value) -> (0xff00, value, new_addr, addr, value)
    AND %shr_const(8)
    MSTORE_GENERAL // Second byte
    DUP1 %add_const(2)
    %stack (new_addr, addr, value) -> (0xff0000, value, new_addr, addr, value)
    AND %shr_const(16)
    MSTORE_GENERAL // Third byte
    DUP1 %add_const(3)
    %stack (new_addr, addr, value) -> (0xff000000, value, new_addr, addr, value)
    AND %shr_const(24)
    MSTORE_GENERAL // Fourth byte
    DUP1 %add_const(4)
    %stack (new_addr, addr, value) -> (0xff00000000, value, new_addr, addr, value)
    AND %shr_const(32)
    MSTORE_GENERAL // Fifth byte
    DUP1 %add_const(5)
    %stack (new_addr, addr, value) -> (0xff0000000000, value, new_addr, addr, value)
    AND %shr_const(40)
    MSTORE_GENERAL // Sixth byte
    DUP1 %add_const(6)
    %stack (new_addr, addr, value) -> (0xff000000000000, value, new_addr, addr, value)
    AND %shr_const(48)
    MSTORE_GENERAL // Seventh byte
    DUP1 %add_const(7)
    %stack (new_addr, addr, value) -> (0xff00000000000000, value, new_addr, addr, value)
    AND %shr_const(56)
    MSTORE_GENERAL // Eighth byte
    %pop2 JUMP

%macro mstore_unpacking_u64_LE
    %stack (addr, value) -> (addr, value, %%after)
    %jump(mstore_unpacking_u64_LE)
%%after:
%endmacro
