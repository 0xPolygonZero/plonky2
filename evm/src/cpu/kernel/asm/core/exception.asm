// These exception codes are arbitrary and assigned by us.
// Note that exceptions can only be triggered in user mode. Triggering an exception
// in kernel mode wwill fail the constraints.
global exception_jumptable:
    // exception 0: out of gas
    JUMPTABLE exc_out_of_gas

    // exception 1: invalid opcode
    JUMPTABLE exc_invalid_opcode

    // exception 2: stack underflow
    JUMPTABLE exc_stack_underflow

    // exception 3: invalid jump destination
    JUMPTABLE exc_invalid_jump_destination

    // exception 4: invalid jumpi destination
    JUMPTABLE exc_invalid_jumpi_destination

    // exception 5: stack overflow
    JUMPTABLE exc_stack_overflow

    // exceptions 6 and 7: unused
    JUMPTABLE panic
    JUMPTABLE panic


global exc_out_of_gas:
    // stack: trap_info
    %ctx_gas_limit
    // stack: gas_limit, trap_info
    DUP2 %shr_const(192)
    // stack: gas_used, gas_limit, trap_info
    DUP2 DUP2
    // stack: gas_used, gas_limit, gas_used, gas_limit, trap_info
    // If gas_used is already over the limit, panic. The exception should have
    // been raised earlier.
    GT %jumpi(panic)
    // stack: gas_used, gas_limit, trap_info
    DUP3 %opcode_from_exp_trap_info
    // stack: opcode, gas_used, gas_limit, trap_info
    %add_const(gas_cost_for_opcode)
    %mload_kernel_code
    // stack: gas_cost, gas_used, gas_limit, trap_info
    ADD
    // stack: new_gas_used, gas_limit, trap_info
    GT
    // stack: is_oog, trap_info
    SWAP1 POP
    // stack: is_oog
    %jumpi(fault_exception)
    // If we didn't jump, we shouldn't have raised the exception.
    PANIC


global exc_invalid_opcode:
    // stack: trap_info
    // check if the opcode that triggered this trap is _actually_ invalid
    %opcode_from_exp_trap_info
    PUSH @INVALID_OPCODES_USER
    // stack: invalid_opcodes_user, opcode
    SWAP1
    // stack: opcode, invalid_opcodes_user
    SHR
    %mod_const(2)
    // stack: opcode_is_invalid
    // if the opcode is indeed invalid, then perform an exceptional exit
    %jumpi(fault_exception)
    // otherwise, panic because this trap should not have been entered
    PANIC


global exc_stack_underflow:
    // stack: trap_info
    %opcode_from_exp_trap_info
    // stack: opcode
    %add_const(min_stack_len_for_opcode)
    %mload_kernel_code
    // stack: min_stack_length
    %stack_length
    // stack: user_stack_length + 1, min_stack_length
    GT
    // stack: user_stack_length >= min_stack_length
    %jumpi(panic)
    %jump(fault_exception)


// Debugging note: this will underflow if entered without at least one item on the stack (in
// addition to trap_info). This is expected; it means that the exc_stack_underflow handler should
// have been used instead.
global exc_invalid_jump_destination:
    // stack: trap_info, jump_dest
    // check that the triggering opcode is indeed JUMP
    %opcode_from_exp_trap_info
    // stack: opcode, jump_dest
    %eq_const(0x56)
    // if it's JUMP, then verify that we're actually jumping to an invalid address
    %jumpi(invalid_jump_jumpi_destination_common)
    // otherwise, panic
    PANIC


// Debugging note: this will underflow if entered without at least two items on the stack (in
// addition to trap_info). This is expected; it means that the exc_stack_underflow handler should
// have been used instead.
global exc_invalid_jumpi_destination:
    // stack: trap_info, jump_dest, condition
    // check that the triggering opcode is indeed JUMPI
    %opcode_from_exp_trap_info
    // stack: opcode, jump_dest, condition
    %sub_const(0x57)
    // if it's not JUMPI, then panic
    %jumpi(panic)
    // otherwise, verify that the condition is nonzero
    // stack: jump_dest, condition
    SWAP1
    // if it's nonzero, then verify that we're actually jumping to an invalid address
    %jumpi(invalid_jump_jumpi_destination_common)
    // otherwise, panic
    PANIC


global invalid_jump_jumpi_destination_common:
    // We have a jump destination on the stack. We want to `PANIC` if it is valid, and jump to
    // `fault_exception` if it is not. An address is a valid jump destination if it points to a
    // `JUMPDEST` instruction. In practice, since in this implementation memory addresses are
    // limited to 32 bits, we check two things:
    //  1. the address is no more than 32 bits long, and
    //  2. it points to a `JUMPDEST` instruction.
    // stack: jump_dest
    DUP1
    %shr_const(32)
    %jumpi(fault_exception) // This keeps one copy of jump_dest on the stack, but that's fine.
    // jump_dest is a valid address; check if it points to a `JUMP_DEST`.
    %mload_current(@SEGMENT_JUMPDEST_BITS)
    // stack: is_valid_jumpdest
    %jumpi(panic) // Trap should never have been entered.
    %jump(fault_exception)


global exc_stack_overflow:
    // stack: trap_info
    // check that the triggering opcode _can_ overflow (i.e., it increases the stack size by 1)
    %opcode_from_exp_trap_info
    PUSH @STACK_LENGTH_INCREASING_OPCODES_USER
    // stack: stack_length_increasing_opcodes_user, opcode
    SWAP1
    // stack: opcode, stack_length_increasing_opcodes_user
    SHR
    %mod_const(2)
    // stack: opcode_increases_stack_length
    // if the opcode indeed increases the stack length, then check whether the stack size is at its
    // maximum value
    %jumpi(exc_stack_overflow_check_stack_length)
    // otherwise, panic because this trap should not have been entered
    PANIC
global exc_stack_overflow_check_stack_length:
    // stack: (empty)
    %stack_length
    %eq_const(1024)
    // if true, stack length is at its maximum allowed value, so the instruction would indeed cause
    // an overflow.
    %jumpi(fault_exception)
    PANIC


// Given the exception trap info, load the opcode that caused the exception
%macro opcode_from_exp_trap_info
    %mod_const(0x100000000) // get program counter from low 32 bits of trap_info
    %mload_current_code
%endmacro


min_stack_len_for_opcode:
    BYTES 0  // 0x00, STOP
    BYTES 2  // 0x01, ADD
    BYTES 2  // 0x02, MUL
    BYTES 2  // 0x03, SUB
    BYTES 2  // 0x04, DIV
    BYTES 2  // 0x05, SDIV
    BYTES 2  // 0x06, MOD
    BYTES 2  // 0x07, SMOD
    BYTES 3  // 0x08, ADDMOD
    BYTES 3  // 0x09, MULMOD
    BYTES 2  // 0x0a, EXP
    BYTES 2  // 0x0b, SIGNEXTEND
    %rep 4  // 0x0c-0x0f, invalid
        BYTES 0
    %endrep

    BYTES 2  // 0x10, LT
    BYTES 2  // 0x11, GT
    BYTES 2  // 0x12, SLT
    BYTES 2  // 0x13, SGT
    BYTES 2  // 0x14, EQ
    BYTES 1  // 0x15, ISZERO
    BYTES 2  // 0x16, AND
    BYTES 2  // 0x17, OR
    BYTES 2  // 0x18, XOR
    BYTES 1  // 0x19, NOT
    BYTES 2  // 0x1a, BYTE
    BYTES 2  // 0x1b, SHL
    BYTES 2  // 0x1c, SHR
    BYTES 2  // 0x1d, SAR
    BYTES 0  // 0x1e, invalid
    BYTES 0  // 0x1f, invalid

    BYTES 2  // 0x20, KECCAK256
    %rep 15 // 0x21-0x2f, invalid
        BYTES 0
    %endrep

    BYTES 0  // 0x30, ADDRESS
    BYTES 1  // 0x31, BALANCE
    BYTES 0  // 0x32, ORIGIN
    BYTES 0  // 0x33, CALLER
    BYTES 0  // 0x34, CALLVALUE
    BYTES 1  // 0x35, CALLDATALOAD
    BYTES 0  // 0x36, CALLDATASIZE
    BYTES 3  // 0x37, CALLDATACOPY
    BYTES 0  // 0x38, CODESIZE
    BYTES 3  // 0x39, CODECOPY
    BYTES 0  // 0x3a, GASPRICE
    BYTES 1  // 0x3b, EXTCODESIZE
    BYTES 4  // 0x3c, EXTCODECOPY
    BYTES 0  // 0x3d, RETURNDATASIZE
    BYTES 3  // 0x3e, RETURNDATACOPY
    BYTES 1  // 0x3f, EXTCODEHASH

    BYTES 1  // 0x40, BLOCKHASH
    BYTES 0  // 0x41, COINBASE
    BYTES 0  // 0x42, TIMESTAMP
    BYTES 0  // 0x43, NUMBER
    BYTES 0  // 0x44, DIFFICULTY
    BYTES 0  // 0x45, GASLIMIT
    BYTES 0  // 0x46, CHAINID
    BYTES 0  // 0x47, SELFBALANCE
    BYTES 0  // 0x48, BASEFEE
    %rep 7  // 0x49-0x4f, invalid
        BYTES 0
    %endrep

    BYTES 1  // 0x50, POP
    BYTES 1  // 0x51, MLOAD
    BYTES 2  // 0x52, MSTORE
    BYTES 2  // 0x53, MSTORE8
    BYTES 1  // 0x54, SLOAD
    BYTES 2  // 0x55, SSTORE
    BYTES 1  // 0x56, JUMP
    BYTES 2  // 0x57, JUMPI
    BYTES 0  // 0x58, PC
    BYTES 0  // 0x59, MSIZE
    BYTES 0  // 0x5a, GAS
    BYTES 0  // 0x5b, JUMPDEST
    %rep 3  // 0x5c-0x5e, invalid
        BYTES 0
    %endrep

    %rep 33 // 0x5f-0x7f, PUSH0-PUSH32
        BYTES 0
    %endrep

    BYTES 1  // 0x80, DUP1
    BYTES 2  // 0x81, DUP2
    BYTES 3  // 0x82, DUP3
    BYTES 4  // 0x83, DUP4
    BYTES 5  // 0x84, DUP5
    BYTES 6  // 0x85, DUP6
    BYTES 7  // 0x86, DUP7
    BYTES 8  // 0x87, DUP8
    BYTES 9  // 0x88, DUP9
    BYTES 10 // 0x89, DUP10
    BYTES 11 // 0x8a, DUP11
    BYTES 12 // 0x8b, DUP12
    BYTES 13 // 0x8c, DUP13
    BYTES 14 // 0x8d, DUP14
    BYTES 15 // 0x8e, DUP15
    BYTES 16 // 0x8f, DUP16

    BYTES 2  // 0x90, SWAP1
    BYTES 3  // 0x91, SWAP2
    BYTES 4  // 0x92, SWAP3
    BYTES 5  // 0x93, SWAP4
    BYTES 6  // 0x94, SWAP5
    BYTES 7  // 0x95, SWAP6
    BYTES 8  // 0x96, SWAP7
    BYTES 9  // 0x97, SWAP8
    BYTES 10 // 0x98, SWAP9
    BYTES 11 // 0x99, SWAP10
    BYTES 12 // 0x9a, SWAP11
    BYTES 13 // 0x9b, SWAP12
    BYTES 14 // 0x9c, SWAP13
    BYTES 15 // 0x9d, SWAP14
    BYTES 16 // 0x9e, SWAP15
    BYTES 17 // 0x9f, SWAP16

    BYTES 2  // 0xa0, LOG0
    BYTES 3  // 0xa1, LOG1
    BYTES 4  // 0xa2, LOG2
    BYTES 5  // 0xa3, LOG3
    BYTES 6  // 0xa4, LOG4

    %rep 27 // 0xa5-0xbf, invalid
        BYTES 0
    %endrep

    %rep 32 // 0xc0-0xdf, MSTORE_32BYTES
        BYTES 4
    %endrep
    
    %rep 16 // 0xe0-0xef, invalid
        BYTES 0
    %endrep

    BYTES 3  // 0xf0, CREATE
    BYTES 7  // 0xf1, CALL
    BYTES 7  // 0xf2, CALLCODE
    BYTES 2  // 0xf3, RETURN
    BYTES 6  // 0xf4, DELEGATECALL
    BYTES 4  // 0xf5, CREATE2
    %rep 4  // 0xf6-0xf9, invalid
        BYTES 0
    %endrep
    BYTES 6  // 0xfa, STATICCALL
    BYTES 0  // 0xfb, invalid
    BYTES 0  // 0xfc, invalid
    BYTES 2  // 0xfd, REVERT
    BYTES 0  // 0xfe, invalid
    BYTES 1  // 0xff, SELFDESTRUCT

// A zero indicates either that the opcode is kernel-only,
// or that it's handled with a syscall.
gas_cost_for_opcode:
    BYTES 0  // 0x00, STOP
    BYTES @GAS_VERYLOW  // 0x01, ADD
    BYTES @GAS_LOW  // 0x02, MUL
    BYTES @GAS_VERYLOW  // 0x03, SUB
    BYTES @GAS_LOW  // 0x04, DIV
    BYTES @GAS_LOW  // 0x05, SDIV
    BYTES @GAS_LOW  // 0x06, MOD
    BYTES @GAS_LOW  // 0x07, SMOD
    BYTES @GAS_MID  // 0x08, ADDMOD
    BYTES @GAS_MID  // 0x09, MULMOD
    BYTES 0  // 0x0a, EXP
    BYTES 0  // 0x0b, SIGNEXTEND
    %rep 4  // 0x0c-0x0f, invalid
        BYTES 0
    %endrep

    BYTES @GAS_VERYLOW  // 0x10, LT
    BYTES @GAS_VERYLOW  // 0x11, GT
    BYTES @GAS_VERYLOW  // 0x12, SLT
    BYTES @GAS_VERYLOW  // 0x13, SGT
    BYTES @GAS_VERYLOW  // 0x14, EQ
    BYTES @GAS_VERYLOW  // 0x15, ISZERO
    BYTES @GAS_VERYLOW  // 0x16, AND
    BYTES @GAS_VERYLOW  // 0x17, OR
    BYTES @GAS_VERYLOW  // 0x18, XOR
    BYTES @GAS_VERYLOW  // 0x19, NOT
    BYTES @GAS_VERYLOW  // 0x1a, BYTE
    BYTES @GAS_VERYLOW  // 0x1b, SHL
    BYTES @GAS_VERYLOW  // 0x1c, SHR
    BYTES @GAS_VERYLOW  // 0x1d, SAR
    BYTES 0  // 0x1e, invalid
    BYTES 0  // 0x1f, invalid

    BYTES 0  // 0x20, KECCAK256
    %rep 15 // 0x21-0x2f, invalid
        BYTES 0
    %endrep

    %rep 25 //0x30-0x48, only syscalls
    BYTES 0  
    %endrep

    %rep 7  // 0x49-0x4f, invalid
        BYTES 0
    %endrep

    BYTES @GAS_BASE  // 0x50, POP
    BYTES 0  // 0x51, MLOAD
    BYTES 0  // 0x52, MSTORE
    BYTES 0  // 0x53, MSTORE8
    BYTES 0  // 0x54, SLOAD
    BYTES 0  // 0x55, SSTORE
    BYTES @GAS_MID  // 0x56, JUMP
    BYTES @GAS_HIGH  // 0x57, JUMPI
    BYTES @GAS_BASE  // 0x58, PC
    BYTES 0  // 0x59, MSIZE
    BYTES 0  // 0x5a, GAS
    BYTES @GAS_JUMPDEST  // 0x5b, JUMPDEST
    %rep 3  // 0x5c-0x5e, invalid
        BYTES 0
    %endrep

    BYTES @GAS_BASE // 0x5f, PUSH0
    %rep 32 // 0x60-0x7f, PUSH1-PUSH32
        BYTES @GAS_VERYLOW
    %endrep

    %rep 16 // 0x80-0x8f, DUP1-DUP16
        BYTES @GAS_VERYLOW
    %endrep

    %rep 16 // 0x90-0x9f, SWAP1-SWAP16
        BYTES @GAS_VERYLOW
    %endrep

    BYTES 0  // 0xa0, LOG0
    BYTES 0  // 0xa1, LOG1
    BYTES 0  // 0xa2, LOG2
    BYTES 0  // 0xa3, LOG3
    BYTES 0  // 0xa4, LOG4
    %rep 11 // 0xa5-0xaf, invalid
        BYTES 0
    %endrep

    %rep 64 // 0xb0-0xef, invalid
        BYTES 0
    %endrep

    BYTES 0  // 0xf0, CREATE
    BYTES 0  // 0xf1, CALL
    BYTES 0  // 0xf2, CALLCODE
    BYTES 0  // 0xf3, RETURN
    BYTES 0  // 0xf4, DELEGATECALL
    BYTES 0  // 0xf5, CREATE2
    %rep 4  // 0xf6-0xf9, invalid
        BYTES 0
    %endrep
    BYTES 0  // 0xfa, STATICCALL
    BYTES 0  // 0xfb, invalid
    BYTES 0  // 0xfc, invalid
    BYTES 0  // 0xfd, REVERT
    BYTES 0  // 0xfe, invalid
    BYTES 0  // 0xff, SELFDESTRUCT
