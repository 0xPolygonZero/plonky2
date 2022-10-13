// Methods for encoding integers as bytes in memory, as well as the reverse,
// decoding bytes as integers. All big-endian.

// Given a pointer to some bytes in memory, pack them into a word. Assumes 0 < len <= 32.
// Pre stack: addr: 3, len, retdest
// Post stack: packed_value
// NOTE: addr: 3 denotes a (context, segment, virtual) tuple
global mload_packing:
    // stack: addr: 3, len, retdest
    DUP3                DUP3 DUP3 MLOAD_GENERAL     DUP5 %eq_const(1)  %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(1)  DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(2)  %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(2)  DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(3)  %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(3)  DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(4)  %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(4)  DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(5)  %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(5)  DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(6)  %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(6)  DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(7)  %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(7)  DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(8)  %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(8)  DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(9)  %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(9)  DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(10) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(10) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(11) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(11) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(12) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(12) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(13) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(13) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(14) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(14) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(15) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(15) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(16) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(16) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(17) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(17) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(18) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(18) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(19) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(19) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(20) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(20) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(21) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(21) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(22) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(22) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(23) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(23) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(24) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(24) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(25) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(25) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(26) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(26) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(27) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(27) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(28) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(28) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(29) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(29) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(30) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(30) DUP4 DUP4 MLOAD_GENERAL ADD DUP5 %eq_const(31) %jumpi(mload_packing_return) %shl_const(8)
    DUP4 %add_const(31) DUP4 DUP4 MLOAD_GENERAL ADD
mload_packing_return:
    %stack (packed_value, addr: 3, len, retdest) -> (retdest, packed_value)
    JUMP

// Pre stack: context, segment, offset, value, len, retdest
// Post stack: offset'
global mstore_unpacking:
    // stack: context, segment, offset, value, len, retdest
    // We will enumerate i in (32 - len)..32.
    // That way BYTE(i, value) will give us the bytes we want.
    DUP5 // len
    PUSH 32
    SUB

mstore_unpacking_loop:
    // stack: i, context, segment, offset, value, len, retdest
    // If i == 32, finish.
    DUP1
    %eq_const(32)
    %jumpi(mstore_unpacking_finish)

    // stack: i, context, segment, offset, value, len, retdest
    DUP5 // value
    DUP2 // i
    BYTE
    // stack: value[i], i, context, segment, offset, value, len, retdest
    DUP5 DUP5 DUP5 // context, segment, offset
    // stack: context, segment, offset, value[i], i, context, segment, offset, value, len, retdest
    MSTORE_GENERAL
    // stack: i, context, segment, offset, value, len, retdest

    // Increment offset.
    SWAP3 %increment SWAP3
    // Increment i.
    %increment

    %jump(mstore_unpacking_loop)

mstore_unpacking_finish:
    // stack: i, context, segment, offset, value, len, retdest
    %pop3
    %stack (offset, value, len, retdest) -> (retdest, offset)
    JUMP
