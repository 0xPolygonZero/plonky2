/// See `smt_utils_hermez:keys.rs` for documentation.

// addr = sum_{0<=i<5} a_i << (32i)
%macro key_balance
    // stack: addr
    DUP1 %and_const(0xffffffff)
    // stack: a_0, addr
    DUP2 %shr_const(32) %and_const(0xffffffff) %shl_const(64) ADD
    // stack: a_0 + a_1<<64, addr
    DUP2 %shr_const(64) %and_const(0xffffffff) %shl_const(128) ADD
    // stack: a_0 + a_1<<64 + a_2<<128, addr
    DUP2 %shr_const(96) %and_const(0xffffffff) %shl_const(192) ADD
    // stack: a_0 + a_1<<64 + a_2<<128 + a_3<<192, addr
    SWAP1 %shr_const(128) %and_const(0xffffffff)
    // stack: a_4, a_0 + a_1<<64 + a_2<<128 + a_3<<192
    %stack (y, x) -> (x, y, @POSEIDON_HASH_ZEROS)
    POSEIDON
%endmacro

// addr = sum_{0<=i<5} a_i << (32i)
%macro key_nonce
    // stack: addr
    DUP1 %and_const(0xffffffff)
    // stack: a_0, addr
    DUP2 %shr_const(32) %and_const(0xffffffff) %shl_const(64) ADD
    // stack: a_0 + a_1<<64, addr
    DUP2 %shr_const(64) %and_const(0xffffffff) %shl_const(128) ADD
    // stack: a_0 + a_1<<64 + a_2<<128, addr
    DUP2 %shr_const(96) %and_const(0xffffffff) %shl_const(192) ADD
    // stack: a_0 + a_1<<64 + a_2<<128 + a_3<<192, addr
    SWAP1 %shr_const(128) %and_const(0xffffffff)
    // stack: a_4, a_0 + a_1<<64 + a_2<<128 + a_3<<192
    %add_const(0x100000000000000000000000000000000) // SMT_KEY_NONCE (=1) << 128
    %stack (y, x) -> (x, y, @POSEIDON_HASH_ZEROS)
    POSEIDON
%endmacro

// addr = sum_{0<=i<5} a_i << (32i)
%macro key_code
    // stack: addr
    DUP1 %and_const(0xffffffff)
    // stack: a_0, addr
    DUP2 %shr_const(32) %and_const(0xffffffff) %shl_const(64) ADD
    // stack: a_0 + a_1<<64, addr
    DUP2 %shr_const(64) %and_const(0xffffffff) %shl_const(128) ADD
    // stack: a_0 + a_1<<64 + a_2<<128, addr
    DUP2 %shr_const(96) %and_const(0xffffffff) %shl_const(192) ADD
    // stack: a_0 + a_1<<64 + a_2<<128 + a_3<<192, addr
    SWAP1 %shr_const(128) %and_const(0xffffffff)
    // stack: a_4, a_0 + a_1<<64 + a_2<<128 + a_3<<192
    %add_const(0x200000000000000000000000000000000) // SMT_KEY_CODE (=2) << 128
    %stack (y, x) -> (x, y, @POSEIDON_HASH_ZEROS)
    POSEIDON
%endmacro

// addr = sum_{0<=i<5} a_i << (32i)
%macro key_code_length
    // stack: addr
    DUP1 %and_const(0xffffffff)
    // stack: a_0, addr
    DUP2 %shr_const(32) %and_const(0xffffffff) %shl_const(64) ADD
    // stack: a_0 + a_1<<64, addr
    DUP2 %shr_const(64) %and_const(0xffffffff) %shl_const(128) ADD
    // stack: a_0 + a_1<<64 + a_2<<128, addr
    DUP2 %shr_const(96) %and_const(0xffffffff) %shl_const(192) ADD
    // stack: a_0 + a_1<<64 + a_2<<128 + a_3<<192, addr
    SWAP1 %shr_const(128) %and_const(0xffffffff)
    // stack: a_4, a_0 + a_1<<64 + a_2<<128 + a_3<<192
    %add_const(0x400000000000000000000000000000000) // SMT_KEY_CODE_LENGTH (=4) << 128
    %stack (y, x) -> (x, y, @POSEIDON_HASH_ZEROS)
    POSEIDON
%endmacro

// addr = sum_{0<=i<5} a_i << (32i)
%macro key_storage
    %stack (addr, slot) -> (slot, %%after, addr)
    %jump(hash_limbs)
%%after:
    // stack: capacity, addr
    SWAP1
    // stack:  addr, capacity
    DUP1 %and_const(0xffffffff)
    // stack: a_0, addr
    DUP2 %shr_const(32) %and_const(0xffffffff) %shl_const(64) ADD
    // stack: a_0 + a_1<<64, addr
    DUP2 %shr_const(64) %and_const(0xffffffff) %shl_const(128) ADD
    // stack: a_0 + a_1<<64 + a_2<<128, addr
    DUP2 %shr_const(96) %and_const(0xffffffff) %shl_const(192) ADD
    // stack: a_0 + a_1<<64 + a_2<<128 + a_3<<192, addr
    SWAP1 %shr_const(128) %and_const(0xffffffff)
    // stack: a_4, a_0 + a_1<<64 + a_2<<128 + a_3<<192
    %add_const(0x300000000000000000000000000000000) // SMT_KEY_STORAGE (=3) << 128
    %stack (y, x, capacity) -> (x, y, capacity)
    POSEIDON
%endmacro

// slot = sum_{0<=i<8} s_i << (32i)
global hash_limbs:
    // stack: slot, retdest
    DUP1 %and_const(0xffffffff)
    // stack: s_0, slot
    DUP2 %shr_const(32) %and_const(0xffffffff) %shl_const(64) ADD
    // stack: s_0 + s_1<<64, slot
    DUP2 %shr_const(64) %and_const(0xffffffff) %shl_const(128) ADD
    // stack: s_0 + s_1<<64 + s_2<<128, slot
    DUP2 %shr_const(96) %and_const(0xffffffff) %shl_const(192) ADD
    // stack: s_0 + s_1<<64 + s_2<<128 + s_3<<192, slot
    DUP2 %shr_const(128) %and_const(0xffffffff)
    // stack: s_4, s_0 + s_1<<64 + s_2<<128 + s_3<<192, slot
    DUP3 %shr_const(160) %and_const(0xffffffff) %shl_const(64) ADD
    // stack: s_4 + s_5<<64, s_0 + s_1<<64 + s_2<<128 + s_3<<192, slot
    DUP3 %shr_const(192) %and_const(0xffffffff) %shl_const(128) ADD
    // stack: s_4 + s_5<<64 + s_6<<128, s_0 + s_1<<64 + s_2<<128 + s_3<<192, slot
    DUP3 %shr_const(224) %and_const(0xffffffff) %shl_const(192) ADD
    // stack: s_4 + s_5<<64 + s_6<<128 + s_7<<192, s_0 + s_1<<64 + s_2<<128 + s_3<<192, slot
    %stack (b, a, slot) -> (a, b, 0)
    POSEIDON
    // stack: hash, retdest
    SWAP1 JUMP
