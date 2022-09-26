global ripemd_storage: // starts by initializing buffer
    // stack: i [init: 64]
    %store_zeros(64, ripemd_storage)
    // stack:
    %jump(store_size)

store_size:
    // stack: length
    %shl_const(3)
    // stack: abcdefgh
    %extract_and_store_byte(64)
    // stack: abcdefg
    %extract_and_store_byte(65)
    // stack: abcdef
    %extract_and_store_byte(66)
    // stack: abcde
    %extract_and_store_byte(67)
    // stack: abcd
    %extract_and_store_byte(68)
    // stack: abc
    %extract_and_store_byte(69)
    // stack: ab
    %extract_and_store_byte(70)
    // stack: a
    %mstore_ripemd_offset(71)
    // stack:           0x80     // padding has 0x80 in first position and zeros elsewhere
    %mstore_ripemd_offset(72)    // store first padding term here so as to avoid extra label
    %jump(store_padding)

store_padding:
    // stack: i (init 63)
    %store_zeros(136, store_padding)
    %jump(store_input_alt)
    %jump(ripemd_init)

store_input_alt:
    // stack:               rem, length, REM_INP
    %stack (rem, length, head) -> (length, rem, 136, head, rem, length)
    SUB
    ADD
    // stack: offset, byte, rem, length, REM_INP
    %mstore_ripemd
    // stack:               rem, length, REM_INP
    %sub_const(1)
    DUP1
    // stack:  rem - 1, rem - 1, length, REM_INP
    %jumpi(store_input_alt)
    // stack:                 0, length
    POP
    %jump(ripemd_init)


store_input:
    // stack:               ADDR    , rem    , length
    DUP3
    DUP3
    DUP3
    MLOAD_GENERAL
    // stack:         byte, ADDR    , rem    , length 
    DUP5
    DUP7
    SUB
    %add_const(136)
    // stack: offset, byte, ADDR    , rem    , length 
    %mstore_ripemd
    // stack:               ADDR    , rem    , length 
    SWAP2
    %add_const(1)
    SWAP2
    // stack:               ADDR + 1, rem    , length
    SWAP3
    %sub_const(1)
    SWAP3
    // stack:               ADDR + 1, rem - 1, length 
    DUP4
    %jumpi(store_input)
    // stack:               ADDR    , 0      , length
    %pop4
    // stack:                                  length
    %jump(ripemd_init)


%macro store_zeros(N, label)
    // stack: i
    %stack (i) -> ($N, i, 0, i)
    SUB
    // stack: offset = N-i, 0, i
    %mstore_ripemd
    // stack: i
    %sub_const(1)
    DUP1
    // stack: i-1, i-1
    %jumpi($label)
    // stack: 0
    POP
%endmacro 

%macro extract_and_store_byte(offset)
    // stack: xsy
    PUSH 0x100
    DUP2
    MOD
    // stack: y, xsy
    %stack (y, xsy) -> (xsy, y, 0x100, y)
    // stack:           xsy, y, 0x100, y
    SUB
    DIV
    SWAP1
    // stack: y, xs
    %mstore_ripemd_offset($offset)
    // stack: xs
%endmacro 

%macro mstore_ripemd_offset(offset)
    // stack:         value 
    PUSH $offset
    // stack: offset, value 
    %mstore_kernel(@SEGMENT_RIPEMD)
    // stack: 
%endmacro

%macro mstore_ripemd
    // stack: offset, value 
    %mstore_kernel(@SEGMENT_RIPEMD)
    // stack: 
%endmacro

%macro mload_ripemd
    %mload_kernel(@SEGMENT_RIPEMD)
%endmacro

// Load LE u32 from 4 contiguous bytes a, b, c, d in SEGMENT_RIPEMD
%macro load_u32_from_block
    // stack: offset
    DUP1
    %mload_ripemd
    // stack: a                       , offset
    DUP2
    %add_const(1)
    %mload_ripemd
    %shl_const(8)
    OR
    // stack: a | (b << 8)            , offset
    DUP2
    %add_const(2)
    %mload_ripemd
    %shl_const(16)
    OR
    // stack: a | (b << 8) | (c << 16), offset
    SWAP1
    %add_const(3)
    %mload_ripemd
    %shl_const(24)
    OR
    // stack: a | (b << 8) | (c << 16) | (d << 24)
%endmacro


// set offset i to offset j in SEGMENT_RIPEMD
%macro mupdate_ripemd
    // stack: j, i
    %mload_ripemd
    // stack: x, i
    SWAP1
    %mstore_ripemd
    // stack:
%endmacro
