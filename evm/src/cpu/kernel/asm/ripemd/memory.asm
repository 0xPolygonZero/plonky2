global ripemd_storage: // starts by initializing buffer
    // stack: i [init: 64]
    %store_zeros(64, ripemd_storage)
    // stack: (empty)
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
    %mstore_kernel_general(71)
    // stack:           0x80    // padding has 0x80 in first position and zeros elsewhere
    %mstore_kernel_general(72)  // store first padding term here so as to avoid extra label
    %jump(store_padding)

store_padding:
    // stack: i [init 63], length
    %store_zeros(136, store_padding)
    // stack:              length
    DUP1
    %jumpi(store_input_stack)
    POP
    %jump(ripemd_init)

store_input_stack:
    // stack:               rem, length, REM_INP
    %stack (rem, length, head) -> (length, rem, 136, head, rem, length)
    SUB
    ADD
    // stack: offset, byte, rem, length, REM_INP
    %mstore_kernel_general
    // stack:               rem, length, REM_INP
    %decrement
    DUP1
    // stack:  rem - 1, rem - 1, length, REM_INP
    %jumpi(store_input_stack)
    // stack:                 0, length
    POP
    %jump(ripemd_init)

store_input:
    // stack:               rem  , ADDR  , length
    DUP4
    DUP4
    DUP4
    MLOAD_GENERAL
    // stack:         byte, rem  , ADDR  , length 
    DUP2
    DUP7
    SUB
    %add_const(136)
    // stack: offset, byte, rem  , ADDR  , length 
    %mstore_kernel_general
    // stack:               rem  , ADDR  , length 
    %decrement
    // stack:               rem-1, ADDR  , length
    SWAP3
    %increment
    SWAP3
    // stack:               rem-1, ADDR+1, length
    DUP1
    %jumpi(store_input)
    // stack:               0    , ADDR  , length
    %pop4
    // stack:                              length
    %jump(ripemd_init)

/// def buffer_update(get, set, times):
///     for i in range(times):
///         buffer[set+i] = bytestring[get+i]

global buffer_update:
    // stack:           get  , set  , times  , retdest
    DUP2
    DUP2
    // stack: get, set, get  , set  , times  , retdest
    %mupdate_kernel_general
    // stack:           get  , set  , times  , retdest
    %increment
    SWAP1 
    %increment
    SWAP1
    SWAP2
    %decrement
    SWAP2
    // stack:           get+1, set+1, times-1, retdest
    DUP3
    %jumpi(buffer_update)
    // stack:           get  , set  , 0      , retdest
    %pop3
    JUMP


%macro store_zeros(N, label)
    // stack: i
    %stack (i) -> ($N, i, 0, i)
    SUB
    // stack: offset = N-i, 0, i
    %mstore_kernel_general
    // stack: i
    %decrement
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
    %mstore_kernel_general($offset)
    // stack: xs
%endmacro 
