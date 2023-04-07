// The CREATE syscall. Address will be
//     address = KEC(RLP(sender, nonce))[12:]
//
// Pre stack: kexit_info, value, code_offset, code_len
// Post stack: address
global sys_create:
    // stack: kexit_info, value, code_offset, code_len
    // TODO: Charge gas.
    %stack (kexit_info, value, code_offset, code_len)
        -> (sys_create_got_address, value, code_offset, code_len, kexit_info)
    %address
    // stack: sender, sys_create_got_address, value, code_offset, code_len, kexit_info
    DUP1 %nonce
    // stack: nonce, sender, sys_create_got_address, value, code_offset, code_len, kexit_info
    SWAP1
    // stack: sender, nonce, sys_create_got_address, value, code_offset, code_len, kexit_info
    %jump(get_create_address)
sys_create_got_address:
    // stack: address, value, code_offset, code_len, kexit_info
    %jump(create_common)

// The CREATE2 syscall; see EIP-1014. Address will be
//     address = KEC(0xff || sender || salt || code_hash)[12:]
//
// Pre stack: kexit_info, value, code_offset, code_len, salt
// Post stack: address
global sys_create2:
    // stack: kexit_info, value, code_offset, code_len, salt
    // TODO: Charge gas.
    SWAP4
    %stack (salt) -> (salt, create_common)
    // stack: salt, create_common, value, code_offset, code_len, kexit_info

    // Hash the code.
    DUP5 // code_len
    DUP5 // code_offset
    PUSH @SEGMENT_MAIN_MEMORY
    GET_CONTEXT
    KECCAK_GENERAL
    // stack: hash, salt, create_common, value, code_offset, code_len, kexit_info

    %address
    // stack: sender, hash, salt, create_common, value, code_offset, code_len, kexit_info
    %jump(get_create2_address)

// Pre stack: address, value, code_offset, code_len, kexit_info
// Post stack: address
global create_common:
    // stack: address, value, code_offset, code_len, kexit_info
    DUP1 %insert_accessed_addresses_no_return

    // Increment the sender's nonce.
    %address
    %increment_nonce
    // stack: address, value, code_offset, code_len, kexit_info

    // Deduct value from the caller.
    DUP2
    %address
    // stack: sender, value, address, value, code_offset, code_len, kexit_info
    %deduct_eth
    // stack: deduct_eth_status, address, value, code_offset, code_len, kexit_info
    %jumpi(fault_exception)
    // stack: address, value, code_offset, code_len, kexit_info

    // Create the new contract account in the state trie.
    DUP1 DUP3
    // stack: value, address, address, value, code_offset, code_len, kexit_info
    %create_contract_account
    // stack: status, address, value, code_offset, code_len, kexit_info
    %jumpi(fault_exception)
    // stack: address, value, code_offset, code_len, kexit_info

    %create_context
    // stack: new_ctx, address, value, code_offset, code_len, kexit_info
    GET_CONTEXT
    // stack: src_ctx, new_ctx, address, value, code_offset, code_len, kexit_info

    // Copy the code from txdata to the new context's code segment.
    %stack (src_ctx, new_ctx, address, value, code_offset, code_len)
        -> (new_ctx, @SEGMENT_CODE, 0, // DST
            src_ctx, @SEGMENT_MAIN_MEMORY, code_offset, // SRC
            code_len,
            run_constructor,
            new_ctx, value, address)
    %jump(memcpy)

run_constructor:
    // stack: new_ctx, value, address, kexit_info
    SWAP1 %set_new_ctx_value
    // stack: new_ctx, address, kexit_info

    // Each line in the block below does not change the stack.
    DUP2 %set_new_ctx_addr
    %address %set_new_ctx_caller
    %set_new_ctx_parent_ctx
    %set_new_ctx_parent_pc(after_constructor)
    // stack: new_ctx, address, kexit_info

    // All but 1/64 of the sender's remaining gas goes to the constructor.
    SWAP2
    // stack: kexit_info, address, new_ctx
    %drain_all_but_one_64th_gas
    %stack (kexit_info, drained_gas, address, new_ctx) -> (drained_gas, new_ctx, address, kexit_info)
    %set_new_ctx_gas_limit
    // stack: new_ctx, address, kexit_info

    %enter_new_ctx
    // (Old context) stack: new_ctx, address, kexit_info

after_constructor:
    // stack: success, leftover_gas, new_ctx, address, kexit_info
    SWAP2
    // stack: new_ctx, leftover_gas, success, address, kexit_info
    POP // TODO: Ignoring new_ctx for now, but we will need it to store code that was returned, if any.
    // stack: leftover_gas, success, address, kexit_info
    %shl_const(192)
    // stack: leftover_gas << 192, success, address, kexit_info
    SWAP2
    // stack: address, success, leftover_gas << 192, kexit_info
    MUL
    // stack: address_if_success, leftover_gas << 192, kexit_info
    SWAP2
    // stack: kexit_info, leftover_gas << 192, address_if_success
    ADD
    // stack: kexit_info, address_if_success
    EXIT_KERNEL
