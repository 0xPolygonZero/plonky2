global sys_mload:
    // stack: kexit_info, offset
    PUSH 0 // acc = 0
    // stack: acc, kexit_info, offset
    DUP3 %add_const( 0) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xf8) ADD
    DUP3 %add_const( 1) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xf0) ADD
    DUP3 %add_const( 2) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xe8) ADD
    DUP3 %add_const( 3) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xe0) ADD
    DUP3 %add_const( 4) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xd8) ADD
    DUP3 %add_const( 5) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xd0) ADD
    DUP3 %add_const( 6) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xc8) ADD
    DUP3 %add_const( 7) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xc0) ADD
    DUP3 %add_const( 8) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xb8) ADD
    DUP3 %add_const( 9) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xb0) ADD
    DUP3 %add_const(10) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xa8) ADD
    DUP3 %add_const(11) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0xa0) ADD
    DUP3 %add_const(12) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x98) ADD
    DUP3 %add_const(13) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x90) ADD
    DUP3 %add_const(14) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x88) ADD
    DUP3 %add_const(15) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x80) ADD
    DUP3 %add_const(16) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x78) ADD
    DUP3 %add_const(17) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x70) ADD
    DUP3 %add_const(18) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x68) ADD
    DUP3 %add_const(19) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x60) ADD
    DUP3 %add_const(20) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x58) ADD
    DUP3 %add_const(21) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x50) ADD
    DUP3 %add_const(22) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x48) ADD
    DUP3 %add_const(23) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x40) ADD
    DUP3 %add_const(24) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x38) ADD
    DUP3 %add_const(25) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x30) ADD
    DUP3 %add_const(26) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x28) ADD
    DUP3 %add_const(27) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x20) ADD
    DUP3 %add_const(28) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x18) ADD
    DUP3 %add_const(29) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x10) ADD
    DUP3 %add_const(30) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x08) ADD
    DUP3 %add_const(31) %mload_current(@SEGMENT_MAIN_MEMORY) %shl_const(0x00) ADD
    %stack (acc, kexit_info, offset) -> (kexit_info, acc)
    EXIT_KERNEL

global sys_mstore:
    // stack: kexit_info, offset, value
    DUP3 PUSH  0 BYTE DUP3 %add_const( 0) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH  1 BYTE DUP3 %add_const( 1) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH  2 BYTE DUP3 %add_const( 2) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH  3 BYTE DUP3 %add_const( 3) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH  4 BYTE DUP3 %add_const( 4) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH  5 BYTE DUP3 %add_const( 5) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH  6 BYTE DUP3 %add_const( 6) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH  7 BYTE DUP3 %add_const( 7) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH  8 BYTE DUP3 %add_const( 8) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH  9 BYTE DUP3 %add_const( 9) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 10 BYTE DUP3 %add_const(10) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 11 BYTE DUP3 %add_const(11) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 12 BYTE DUP3 %add_const(12) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 13 BYTE DUP3 %add_const(13) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 14 BYTE DUP3 %add_const(14) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 15 BYTE DUP3 %add_const(15) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 16 BYTE DUP3 %add_const(16) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 17 BYTE DUP3 %add_const(17) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 18 BYTE DUP3 %add_const(18) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 19 BYTE DUP3 %add_const(19) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 20 BYTE DUP3 %add_const(20) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 21 BYTE DUP3 %add_const(21) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 22 BYTE DUP3 %add_const(22) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 23 BYTE DUP3 %add_const(23) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 24 BYTE DUP3 %add_const(24) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 25 BYTE DUP3 %add_const(25) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 26 BYTE DUP3 %add_const(26) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 27 BYTE DUP3 %add_const(27) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 28 BYTE DUP3 %add_const(28) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 29 BYTE DUP3 %add_const(29) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 30 BYTE DUP3 %add_const(30) %mstore_current(@SEGMENT_MAIN_MEMORY)
    DUP3 PUSH 31 BYTE DUP3 %add_const(31) %mstore_current(@SEGMENT_MAIN_MEMORY)
    %stack (kexit_info, offset, value) -> (kexit_info)
    EXIT_KERNEL

global sys_mstore8:
    // stack: kexit_info, offset, value
    %stack (kexit_info, offset, value) -> (offset, value, kexit_info)
    %mstore_current(@SEGMENT_MAIN_MEMORY)
    // stack: kexit_info
    EXIT_KERNEL
