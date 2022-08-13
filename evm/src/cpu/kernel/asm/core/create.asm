// Create a new contract account with the traditional address scheme, i.e.
//     address = KEC(RLP(sender, nonce))[12:]
// This can be used both for the CREATE instruction and for contract-creation
// transactions.
//
// Pre stack: CODE_ADDR, code_len, retdest
// Post stack: address
// Note: CODE_ADDR refers to a (context, segment, offset) tuple.
global create:
    // stack: sender, endowment, CODE_ADDR, code_len, retdest
    DUP1 %get_nonce
    // stack: nonce, sender, endowment, CODE_ADDR, code_len, retdest
    // Call get_create_address and have it return to create_inner.
    %stack (nonce, sender)
        -> (sender, nonce, create_inner, sender)
    %jump(get_create_address)

// CREATE2; see EIP-1014. Address will be
//     address = KEC(0xff || sender || salt || code_hash)[12:]
//
// Pre stack: sender, endowment, salt, CODE_ADDR, code_len, retdest
// Post stack: address
// Note: CODE_ADDR refers to a (context, segment, offset) tuple.
global create2:
    // stack: sender, endowment, salt, CODE_ADDR, code_len, retdest
    // Call get_create2_address and have it return to create_inner.
    %stack (sender, endowment, salt) -> (salt, sender, endowment)
    // stack: salt, sender, endowment, CODE_ADDR, code_len, retdest
    DUP7 DUP7 DUP7 DUP7 // CODE_ADDR and code_len
    // stack: CODE_ADDR, code_len, salt, sender, endowment, CODE_ADDR, code_len, retdest
    PUSH create_inner
    // stack: create_inner, CODE_ADDR, code_len, salt, sender, endowment, CODE_ADDR, code_len, retdest
    SWAP5 // create_inner <-> salt
    // stack: salt, CODE_ADDR, code_len, create_inner, sender, endowment, CODE_ADDR, code_len, retdest
    DUP7 // sender
    // stack: sender, salt, CODE_ADDR, code_len, create_inner, sender, endowment, CODE_ADDR, code_len, retdest
    %jump(get_create2_address)

// Pre stack: address, sender, endowment, CODE_ADDR, code_len, retdest
// Post stack: address
// Note: CODE_ADDR refers to a (context, segment, offset) tuple.
create_inner:
    // stack: address, sender, endowment, CODE_ADDR, code_len, retdest
    %stack (address, sender, endowment)
        -> (sender, address, endowment, sender, address)
    // TODO: Need to handle insufficient balance failure.
    %transfer_eth
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
