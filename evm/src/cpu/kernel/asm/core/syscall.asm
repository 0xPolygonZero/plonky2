global syscall_jumptable:
    // 0x00-0x0f
    JUMPTABLE sys_stop
    JUMPTABLE panic // add is implemented natively
    JUMPTABLE panic // mul is implemented natively
    JUMPTABLE panic // sub is implemented natively
    JUMPTABLE panic // div is implemented natively
    JUMPTABLE sys_sdiv
    JUMPTABLE panic // mod is implemented natively
    JUMPTABLE sys_smod
    JUMPTABLE panic // addmod is implemented natively
    JUMPTABLE panic // mulmod is implemented natively
    JUMPTABLE sys_exp
    JUMPTABLE sys_signextend
    JUMPTABLE panic // 0x0c is an invalid opcode
    JUMPTABLE panic // 0x0d is an invalid opcode
    JUMPTABLE panic // 0x0e is an invalid opcode
    JUMPTABLE panic // 0x0f is an invalid opcode

    // 0x10-0x1f
    JUMPTABLE panic // lt is implemented natively
    JUMPTABLE panic // gt is implemented natively
    JUMPTABLE sys_slt
    JUMPTABLE sys_sgt
    JUMPTABLE panic // eq is implemented natively
    JUMPTABLE panic // iszero is implemented natively
    JUMPTABLE panic // and is implemented natively
    JUMPTABLE panic // or is implemented natively
    JUMPTABLE panic // xor is implemented natively
    JUMPTABLE panic // not is implemented natively
    JUMPTABLE panic // byte is implemented natively
    JUMPTABLE panic // shl is implemented natively
    JUMPTABLE panic // shr is implemented natively
    JUMPTABLE sys_sar
    JUMPTABLE panic // 0x1e is an invalid opcode
    JUMPTABLE panic // 0x1f is an invalid opcode

    // 0x20-0x2f
    JUMPTABLE sys_keccak256
    %rep 15
        JUMPTABLE panic // 0x21-0x2f are invalid opcodes
    %endrep

    // 0x30-0x3f
    JUMPTABLE sys_address
    JUMPTABLE sys_balance
    JUMPTABLE sys_origin
    JUMPTABLE sys_caller
    JUMPTABLE sys_callvalue
    JUMPTABLE sys_calldataload
    JUMPTABLE sys_calldatasize
    JUMPTABLE sys_calldatacopy
    JUMPTABLE sys_codesize
    JUMPTABLE sys_codecopy
    JUMPTABLE sys_gasprice
    JUMPTABLE sys_extcodesize
    JUMPTABLE sys_extcodecopy
    JUMPTABLE sys_returndatasize
    JUMPTABLE sys_returndatacopy
    JUMPTABLE sys_extcodehash

    // 0x40-0x4f
    JUMPTABLE sys_blockhash
    JUMPTABLE sys_coinbase
    JUMPTABLE sys_timestamp
    JUMPTABLE sys_number
    JUMPTABLE sys_prevrandao
    JUMPTABLE sys_gaslimit
    JUMPTABLE sys_chainid
    JUMPTABLE sys_selfbalance
    JUMPTABLE sys_basefee
    %rep 7
        JUMPTABLE panic // 0x49-0x4f are invalid opcodes
    %endrep

    // 0x50-0x5f
    JUMPTABLE panic // pop is implemented natively
    JUMPTABLE sys_mload
    JUMPTABLE sys_mstore
    JUMPTABLE sys_mstore8
    JUMPTABLE sys_sload
    JUMPTABLE sys_sstore
    JUMPTABLE panic // jump is implemented natively
    JUMPTABLE panic // jumpi is implemented natively
    JUMPTABLE panic // pc is implemented natively
    JUMPTABLE sys_msize
    JUMPTABLE sys_gas
    JUMPTABLE panic // jumpdest is implemented natively
    JUMPTABLE panic // 0x5c is an invalid opcode
    JUMPTABLE panic // 0x5d is an invalid opcode
    JUMPTABLE panic // 0x5e is an invalid opcode
    JUMPTABLE panic // 0x5f is an invalid opcode

    // 0x60-0x6f
    %rep 16
        JUMPTABLE panic // push1-push16 are implemented natively
    %endrep

    // 0x70-0x7f
    %rep 16
        JUMPTABLE panic // push17-push32 are implemented natively
    %endrep

    // 0x80-0x8f
    %rep 16
        JUMPTABLE panic // dup1-dup16 are implemented natively
    %endrep

    // 0x90-0x9f
    %rep 16
        JUMPTABLE panic // swap1-swap16 are implemented natively
    %endrep

    // 0xa0-0xaf
    JUMPTABLE sys_log0
    JUMPTABLE sys_log1
    JUMPTABLE sys_log2
    JUMPTABLE sys_log3
    JUMPTABLE sys_log4
    %rep 11
        JUMPTABLE panic // 0xa5-0xaf are invalid opcodes
    %endrep

    // 0xb0-0xbf
    %rep 16
        JUMPTABLE panic // 0xb0-0xbf are invalid opcodes
    %endrep

    // 0xc0-0xcf
    %rep 16
        JUMPTABLE panic // 0xc0-0xcf are invalid opcodes
    %endrep

    // 0xd0-0xdf
    %rep 16
        JUMPTABLE panic // 0xd0-0xdf are invalid opcodes
    %endrep

    // 0xe0-0xef
    %rep 16
        JUMPTABLE panic // 0xe0-0xef are invalid opcodes
    %endrep

    // 0xf0-0xff
    JUMPTABLE sys_create
    JUMPTABLE sys_call
    JUMPTABLE sys_callcode
    JUMPTABLE sys_return
    JUMPTABLE sys_delegatecall
    JUMPTABLE sys_create2
    JUMPTABLE panic // 0xf6 is an invalid opcode
    JUMPTABLE panic // 0xf7 is an invalid opcode
    JUMPTABLE panic // 0xf8 is an invalid opcode
    JUMPTABLE panic // 0xf9 is an invalid opcode
    JUMPTABLE sys_staticcall
    JUMPTABLE panic // 0xfb is an invalid opcode
    JUMPTABLE panic // 0xfc is an invalid opcode
    JUMPTABLE sys_revert
    JUMPTABLE panic // 0xfe is an invalid opcode
    JUMPTABLE sys_selfdestruct
