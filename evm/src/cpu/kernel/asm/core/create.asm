// TODO: This file needs to be cleaned up.
// `create` is no longer being used for contract-creation txns,
// so it can be inlined. Also need to set metadata on new ctx.

// The CREATE syscall.
//
// Pre stack: kexit_info, value, code_offset, code_len
// Post stack: address
global sys_create:
    // TODO: Charge gas.
    %stack (kexit_info, value, code_offset, code_len)
        -> (value, 0, @SEGMENT_MAIN_MEMORY, code_offset, code_len)
    %address
    // stack: sender, value, CODE_ADDR: 3, code_len, sys_create_finish, kexit_info
    %jump(create)
sys_create_finish:
    // stack: address, kexit_info
    SWAP1
    EXIT_KERNEL

// Create a new contract account with the traditional address scheme, i.e.
//     address = KEC(RLP(sender, nonce))[12:]
// This can be used both for the CREATE instruction and for contract-creation
// transactions.
//
// Pre stack: sender, endowment, CODE_ADDR: 3, code_len, retdest
// Post stack: address
// Note: CODE_ADDR refers to a (context, segment, offset) tuple.
global create:
    // stack: sender, endowment, CODE_ADDR, code_len, retdest
    DUP1 %nonce

    // stack: nonce, sender, endowment, CODE_ADDR, code_len, retdest
    // Call get_create_address and have it return to create_inner.
    %stack (nonce, sender)
        -> (sender, nonce, create_inner, sender)
    %jump(get_create_address)

// CREATE2; see EIP-1014. Address will be
//     address = KEC(0xff || sender || salt || code_hash)[12:]
//
// Pre stack: kexit_info, value, code_offset, code_len, salt
// Post stack: address
global sys_create2:
    // stack: kexit_info, value, code_offset, code_len, salt
    // TODO: Charge gas.
    SWAP4
    %stack (salt) -> (salt, sys_create2_got_address)
    // stack: salt, sys_create2_got_address, value, code_offset, code_len, kexit_info
    DUP5 // code_len
    DUP5 // code_offset
    PUSH @SEGMENT_MAIN_MEMORY
    GET_CONTEXT
    KECCAK_GENERAL
    // stack: hash, salt, sys_create2_got_address, value, code_offset, code_len, kexit_info
    %address
    // stack: sender, hash, salt, sys_create2_got_address, value, code_offset, code_len, kexit_info
    %jump(get_create2_address)
sys_create2_got_address:
    // stack: address, value, code_offset, code_len, kexit_info
    %address
    %stack (sender, address, value, code_offset, code_len, kexit_info)
        -> (address, sender, value, 0, @SEGMENT_MAIN_MEMORY, code_offset, code_len,
            sys_create2_finish, kexit_info)
    %jump(create_inner)
sys_create2_finish:
    // stack: address, kexit_info
    SWAP1
    EXIT_KERNEL

// Pre stack: address, sender, endowment, CODE_ADDR, code_len, retdest
// Post stack: address
// Note: CODE_ADDR refers to a (context, segment, offset) tuple.
global create_inner:
    // stack: address, sender, endowment, CODE_ADDR, code_len, retdest
    DUP1 %insert_accessed_addresses_no_return
    %stack (address, sender, endowment)
        -> (sender, address, endowment, sender, address)

    %transfer_eth
    // stack: transfer_eth_status, sender, address, CODE_ADDR, code_len, retdest
    %jumpi(fault_exception)
    // stack: sender, address, CODE_ADDR, code_len, retdest

    %increment_nonce
    // stack: address, CODE_ADDR, code_len, retdest

    %create_context
    // stack: new_ctx, address, CODE_ADDR, code_len, retdest
    %stack (new_ctx, address, src_ctx, src_segment, src_offset, code_len)
        -> (new_ctx, @SEGMENT_CODE, 0,
            src_ctx, src_segment, src_offset,
            code_len, run_constructor,
            new_ctx, address)
    %jump(memcpy)

run_constructor:
    // stack: new_ctx, address, retdest
    // At this point, the initialization code has been loaded.
    // Save our return address in memory, so we'll be in `after_constructor`
    // after the new context returns.
    // Note: We can't use %mstore_context_metadata because we're writing to
    // memory owned by the new context, not the current one.
    %stack (new_ctx) -> (new_ctx, @SEGMENT_CONTEXT_METADATA,
                         @CTX_METADATA_PARENT_PC, after_constructor, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx, address, retdest

    // Now, switch to the new context and go to usermode with PC=0.
    SET_CONTEXT
    // stack: (empty, since we're in the new context)
    PUSH 0
    EXIT_KERNEL

after_constructor:
    // stack: address, retdest
    // TODO: If code was returned, store it in the account.
    SWAP1
    JUMP
