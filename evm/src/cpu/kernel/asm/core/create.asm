// The CREATE syscall. Address will be
//     address = KEC(RLP(sender, nonce))[12:]
//
// Pre stack: kexit_info, value, code_offset, code_len
// Post stack: address
global sys_create:
    %check_static

    %stack (kexit_info, value, code_offset, code_len) -> (code_len, code_offset, kexit_info, value, code_offset, code_len)
    %checked_mem_expansion
    // stack: kexit_info, value, code_offset, code_len
    %charge_gas_const(@GAS_CREATE)
    // stack: kexit_info, value, code_offset, code_len
    DUP4
    // stack: code_len, kexit_info, value, code_offset, code_len
    %check_initcode_size

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
    %check_static

    // stack: kexit_info, value, code_offset, code_len, salt
    %stack (kexit_info, value, code_offset, code_len) -> (code_len, code_offset, kexit_info, value, code_offset, code_len)
    %checked_mem_expansion
    // stack: kexit_info, value, code_offset, code_len, salt
    DUP4 %num_bytes_to_num_words
    %mul_const(@GAS_KECCAK256WORD) %add_const(@GAS_CREATE) %charge_gas
    // stack: kexit_info, value, code_offset, code_len, salt
    DUP4
    // stack: code_len, kexit_info, value, code_offset, code_len, salt
    %check_initcode_size


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
    DUP2 DUP2 %address %journal_add_balance_transfer // Add journal entry for the balance transfer.

    %create_context
    // stack: new_ctx, address, value, code_offset, code_len, kexit_info
    GET_CONTEXT
    // stack: src_ctx, new_ctx, address, value, code_offset, code_len, kexit_info

    %stack (src_ctx, new_ctx, address, value, code_offset, code_len) ->
        (code_len, new_ctx, src_ctx, new_ctx, address, value, code_offset, code_len)
    %set_new_ctx_code_size POP
    // Copy the code from memory to the new context's code segment.
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
    POP


    // TODO: Skip blocks below if success is false.
    // EIP-3541: Reject new contract code starting with the 0xEF byte
    PUSH 0 %mload_current(@SEGMENT_RETURNDATA) %eq_const(0xEF) %jumpi(fault_exception)

    // Charge gas for the code size.
    SWAP3
    // stack: kexit_info, success, address, leftover_gas
    %returndatasize // Size of the code.
    // stack: code_size, kexit_info, success, address, leftover_gas
    DUP1 %gt_const(@MAX_CODE_SIZE)
    %jumpi(fault_exception)
    // stack: code_size, kexit_info, success, address, leftover_gas
    %mul_const(@GAS_CODEDEPOSIT) %charge_gas
    SWAP3

    // Store the code hash of the new contract.
    GET_CONTEXT
    %returndatasize
    %stack (size, ctx) -> (ctx, @SEGMENT_RETURNDATA, 0, size) // context, segment, offset, len
    KECCAK_GENERAL
    // stack: codehash, leftover_gas, success, address, kexit_info
    %observe_new_contract
    DUP4
    // stack: address, codehash, leftover_gas, success, address, kexit_info
    %set_codehash

    // Set the return data size to 0.
    %mstore_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 0)

    // stack: leftover_gas, success, address, kexit_info
    %shl_const(192)
    // stack: leftover_gas << 192, success, address, kexit_info
    SWAP2
    // stack: address, success, leftover_gas << 192, kexit_info
    MUL
    // stack: address_if_success, leftover_gas << 192, kexit_info
    SWAP2
    // stack: kexit_info, leftover_gas << 192, address_if_success
    SUB
    // stack: kexit_info, address_if_success
    EXIT_KERNEL

%macro set_codehash
    %stack (addr, codehash) -> (addr, codehash, %%after)
    %jump(set_codehash)
%%after:
    // stack: (empty)
%endmacro

// Pre stack: addr, codehash, redest
// Post stack: (empty)
// TODO: Should it be copy-on-write (with make_account_copy) instead of mutating the trie?
global set_codehash:
    // stack: addr, codehash, retdest
    %mpt_read_state_trie
    // stack: account_ptr, codehash, retdest
    %add_const(3)
    // stack: codehash_ptr, codehash, retdest
    %mstore_trie_data
    // stack: retdest
    JUMP

// Check and charge gas cost for initcode size. See EIP-3860.
// Pre stack: code_size, kexit_info
// Post stack: kexit_info
%macro check_initcode_size
    DUP1 %gt_const(@MAX_INITCODE_SIZE) %jumpi(fault_exception)
    // stack: code_size, kexit_info
    %num_bytes_to_num_words %mul_const(@INITCODE_WORD_COST)
    %charge_gas
%endmacro


// This should be called whenever a new contract is created.
// It does nothing, but just provides a single hook where code can react to newly created contracts.
// When called, the code corresponding to `codehash` should be stored in the return data.
// Pre stack: codehash, retdest
// Post stack: codehash
global observe_new_contract:
    // stack codehash, retdest
    SWAP1 JUMP

%macro observe_new_contract
    %stack (codehash) -> (codehash, %%after)
    %jump(observe_new_contract)
%%after:
    // stack: codehash
%endmacro
