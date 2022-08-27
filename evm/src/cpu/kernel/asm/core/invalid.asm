global handle_invalid:
    // stack: trap_info

    // if the kernel is trying to execute an invalid instruction, then we've already screwed up and
    // there's no chance of getting a useful proof, so we just panic
    DUP1
    // stack: trap_info, trap_info
    %shr_const(32)
    // stack: is_kernel, trap_info
    %jumpi(panic)

    // check if the opcode that triggered this trap is _actually_ invalid
    // stack: program_counter (is_kernel == 0, so trap_info == program_counter)
    %mload_current_code
    // stack: opcode
    // Python:
    //   >>> invalid_ranges_inclusive = [(0x0c, 0x0f), (0x1e, 0x1f), (0x21, 0x2f), (0x49, 0x4f),
    //   ...                             (0x5c, 0x5f), (0xa5, 0xef), (0xf6, 0xf9), (0xfb, 0xfc),
    //   ...                             (0xfe, 0xfe)]
    //   >>> inclusive_range_bits = lambda start, end: sum(1 << i for i in range(start, end + 1))
    //   >>> hex(sum(inclusive_range_bits(start, end) for start, end in invalid_ranges_inclusive))
    //   '0x5bc0ffffffffffffffffffe00000000000000000f000fe000000fffec000f000'
    PUSH 0x5bc0ffffffffffffffffffe00000000000000000f000fe000000fffec000f000
    // stack: invalid_opcodes_bitmap, opcode
    SWAP1
    // stack: opcode, invalid_opcodes_bitmap
    SHR
    %and_const(1)
    // stack: opcode_is_invalid
    // if the opcode is indeed invalid, then perform an exceptional exit
    %jumpi(fault_exception)
    // otherwise, panic because this trap should not have been entered
panic:
    PANIC
