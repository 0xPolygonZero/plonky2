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
    PUSH @INVALID_OPCODES_USER
    // stack: invalid_opcodes_user, opcode
    SWAP1
    // stack: opcode, invalid_opcodes_user
    SHR
    %and_const(1)
    // stack: opcode_is_invalid
    // if the opcode is indeed invalid, then perform an exceptional exit
    %jumpi(fault_exception)
    // otherwise, panic because this trap should not have been entered
    PANIC
