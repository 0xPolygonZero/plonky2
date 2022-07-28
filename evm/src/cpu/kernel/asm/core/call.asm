// Handlers for call-like operations, namely CALL, CALLCODE, STATICCALL and DELEGATECALL.

// Creates a new sub context and executes the code of the given account.
global call:
    // stack: gas, address, value, args_offset, args_size, ret_offset, ret_size
    %address
    %stack (self, gas, address, value)
           // These are (should_transfer_value, value, static, gas, sender, storage, code_addr)
        -> (1, value, 0, gas, self, address, address)
    %jump(call_common)

// Creates a new sub context as if calling itself, but with the code of the
// given account. In particular the storage remains the same.
global call_code:
    // stack: gas, address, value, args_offset, args_size, ret_offset, ret_size
    %address
    %stack (self, gas, address, value)
           // These are (should_transfer_value, value, static, gas, sender, storage, code_addr)
        -> (1, value, 0, gas, self, self, address)
    %jump(call_common)

// Creates a new sub context and executes the code of the given account.
// Equivalent to CALL, except that it does not allow any state modifying
// instructions or sending ETH in the sub context. The disallowed instructions
// are CREATE, CREATE2, LOG0, LOG1, LOG2, LOG3, LOG4, SSTORE, SELFDESTRUCT and
// CALL if the value sent is not 0.
global static_all:
    // stack: gas, address, args_offset, args_size, ret_offset, ret_size
    %address
    %stack (self, gas, address)
           // These are (should_transfer_value, value, static, gas, sender, storage, code_addr)
        -> (0, 0, 1, gas, self, address, address)
    %jump(call_common)

// Creates a new sub context as if calling itself, but with the code of the
// given account. In particular the storage, the current sender and the current
// value remain the same.
global delegate_call:
    // stack: gas, address, args_offset, args_size, ret_offset, ret_size
    %address
    %sender
    %callvalue
    %stack (self, sender, value, gas, address)
           // These are (should_transfer_value, value, static, gas, sender, storage, code_addr)
        -> (0, value, 0, gas, sender, self, address)
    %jump(call_common)

call_common:
    // stack: should_transfer_value, value, static, gas, sender, storage, code_addr, args_offset, args_size, ret_offset, ret_size
    // TODO: Set all the appropriate metadata fields...
    %create_context
    // stack: new_ctx, after_call
    // Now, switch to the new context and go to usermode with PC=0.
    SET_CONTEXT
    PUSH 0
    EXIT_KERNEL

after_call:
    // TODO: Set RETURNDATA etc.
