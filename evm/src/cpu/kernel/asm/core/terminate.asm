// Handlers for operations which terminate the current context, namely STOP,
// RETURN, SELFDESTRUCT, REVERT, and exceptions such as stack underflow.

global sys_stop:
    // stack: kexit_info
    // Set the parent context's return data size to 0.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 0)

    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

global sys_return:
    // stack: kexit_info, offset, size
    %stack (kexit_info, offset, size) -> (offset, size, kexit_info, offset, size)
    %add_or_fault 
    // stack: offset+size, kexit_info, offset, size
    DUP4 ISZERO %jumpi(return_zero_size)
    // stack: offset+size, kexit_info, offset, size
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
    %jump(return_after_gas)
return_zero_size:
    POP
return_after_gas:
    // Load the parent's context.
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)

    // Store the return data size in the parent context's metadata.
    %stack (parent_ctx, kexit_info, offset, size) ->
        (parent_ctx, @CTX_METADATA_RETURNDATA_SIZE, size, offset, size, parent_ctx, kexit_info)
    ADD // addr (CTX offsets are already scaled by their segment)
    SWAP1
    // stack: size, addr, offset, size, parent_ctx, kexit_info
    MSTORE_GENERAL
    // stack: offset, size, parent_ctx, kexit_info

    // Store the return data in the parent context's returndata segment.
    PUSH @SEGMENT_MAIN_MEMORY
    GET_CONTEXT
    %build_address

    %stack (addr, size, parent_ctx, kexit_info) ->
        (
        parent_ctx, @SEGMENT_RETURNDATA, // DST
        addr, // SRC
        size, sys_return_finish, kexit_info // count, retdest, ...
        )
    %build_address_no_offset
    // stack: DST, SRC, size, sys_return_finish, kexit_info
    %jump(memcpy_bytes)

sys_return_finish:
    // stack: kexit_info
    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

global sys_selfdestruct:
    %check_static
    // stack: kexit_info, recipient
    SWAP1 %u256_to_addr
    %address DUP1 %balance

    // Insert recipient into the accessed addresses list.
    // stack: balance, address, recipient, kexit_info
    DUP3 %insert_accessed_addresses

    // Set the parent context's return data size to 0.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 0)

    // Compute gas.
    // stack: cold_access, balance, address, recipient, kexit_info
    %mul_const(@GAS_COLDACCOUNTACCESS)
    DUP2
    // stack: balance, gas_coldaccess, balance, address, recipient, kexit_info
    ISZERO %not_bit
    // stack: balance!=0, gas_coldaccess, balance, address, recipient, kexit_info
    DUP5 %is_dead MUL %mul_const(@GAS_NEWACCOUNT)
    // stack: gas_newaccount, gas_coldaccess, balance, address, recipient, kexit_info
    ADD %add_const(@GAS_SELFDESTRUCT)
    %stack (gas, balance, address, recipient, kexit_info) -> (gas, kexit_info, balance, address, recipient)
    %charge_gas
    %stack (kexit_info, balance, address, recipient) -> (balance, address, recipient, kexit_info)

    // Set the balance of the address to 0.
    // stack: balance, address, recipient, kexit_info
    PUSH 0
    // stack: 0, balance, address, recipient, kexit_info
    DUP3 %mpt_read_state_trie
    // stack: account_ptr, 0, balance, address, recipient, kexit_info
    %add_const(1)
    // stack: balance_ptr, 0, balance, address, recipient, kexit_info
    %mstore_trie_data


    // EIP-6780: insert address into the selfdestruct set only if contract has been created
    // during the current transaction.
    // stack: balance, address, recipient, kexit_info
    DUP2 %contract_just_created
    // stack: is_just_created, balance, address, recipient, kexit_info
    %jumpi(sys_selfdestruct_just_created)

    // Send the balance to the recipient. 
    %stack (balance, address, recipient, kexit_info) ->
        (recipient, balance, address, recipient, balance, kexit_info)
    %add_eth

sys_selfdestruct_journal_add:
    // stack: address, recipient, balance, kexit_info
    %journal_add_account_destroyed

    // stack: kexit_info
    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

sys_selfdestruct_just_created:
    // Send funds to beneficiary only if the recipient isn't the same as the address.
    %stack (balance, address, recipient, kexit_info) -> (address, recipient, balance, address, recipient, balance, kexit_info)
    EQ ISZERO
    // stack: address ≠ recipient, balance, address, recipient, balance, kexit_info
    MUL
    // stack: maybe_balance, address, recipient, balance, kexit_info
    DUP3
    // stack: recipient, maybe_balance, address, recipient, balance, kexit_info
    %add_eth
    // stack: address, recipient, balance, kexit_info
    DUP1
    %insert_selfdestruct_list
    %jump(sys_selfdestruct_journal_add)

global sys_revert:
    // stack: kexit_info, offset, size
    %stack (kexit_info, offset, size) -> (offset, size, kexit_info, offset, size)
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size
    DUP4 ISZERO %jumpi(revert_zero_size)
    // stack: offset+size, kexit_info, offset, size
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
    %jump(revert_after_gas)
revert_zero_size:
    POP
revert_after_gas:
    // Load the parent's context.
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)

    // Store the return data size in the parent context's metadata.
    %stack (parent_ctx, kexit_info, offset, size) ->
        (parent_ctx, @CTX_METADATA_RETURNDATA_SIZE, size, offset, size, parent_ctx, kexit_info)
    ADD // addr (CTX offsets are already scaled by their segment)
    SWAP1
    // stack: size, addr, offset, size, parent_ctx, kexit_info
    MSTORE_GENERAL
    // stack: offset, size, parent_ctx, kexit_info

    // Store the return data in the parent context's returndata segment.
    PUSH @SEGMENT_MAIN_MEMORY
    GET_CONTEXT
    %build_address

    %stack (addr, size, parent_ctx, kexit_info) ->
        (
        parent_ctx, @SEGMENT_RETURNDATA, // DST
        addr,  // SRC
        size, sys_revert_finish, kexit_info // count, retdest, ...
        )
    %build_address_no_offset
    // stack: DST, SRC, size, sys_revert_finish, kexit_info
    %jump(memcpy_bytes)

sys_revert_finish:
    %leftover_gas
    // stack: leftover_gas
    %revert_checkpoint
    PUSH 0 // success
    %jump(terminate_common)

// The execution is in an exceptional halting state if
// - there is insufficient gas
// - the instruction is invalid
// - there are insufficient stack items
// - a JUMP/JUMPI destination is invalid
// - the new stack size would be larger than 1024, or
// - state modification is attempted during a static call
global fault_exception:
    // stack: (empty)
    %revert_checkpoint
    PUSH 0 // leftover_gas
    // Set the parent context's return data size to 0.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 0)
    PUSH 0 // success
    %jump(terminate_common)

global terminate_common:
    // stack: success, leftover_gas
    // TODO: Panic if we exceeded our gas limit?

    // We want to move the success flag from our (child) context's stack to the
    // parent context's stack. We will write it to memory, specifically
    // SEGMENT_KERNEL_GENERAL[0], then load it after the context switch.
    PUSH 0
    // stack: 0, success, leftover_gas
    %mstore_kernel_general
    // stack: leftover_gas

    // Similarly, we write leftover_gas to SEGMENT_KERNEL_GENERAL[1] so that
    // we can later read it after switching to the parent context.
    PUSH 1
    // stack: 1, leftover_gas
    %mstore_kernel_general
    // stack: (empty)

    // Similarly, we write the parent PC to SEGMENT_KERNEL_GENERAL[2] so that
    // we can later read it after switching to the parent context.
    PUSH 2
    PUSH @SEGMENT_KERNEL_GENERAL
    %build_kernel_address
    %mload_context_metadata(@CTX_METADATA_PARENT_PC)
    MSTORE_GENERAL
    // stack: (empty)

    // Go back to the parent context.
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    SET_CONTEXT
    %decrement_call_depth
    // stack: (empty)

    // Load the fields that we stored in SEGMENT_KERNEL_GENERAL.
    PUSH 1 %mload_kernel_general // leftover_gas
    PUSH 0 %mload_kernel_general // success
    PUSH 2 %mload_kernel_general // parent_pc

    // stack: parent_pc, success, leftover_gas
    JUMP




// Returns 1 if the address is present in SEGMENT_CREATED_CONTRACTS, meaning that it has been
// created during this transaction. Returns 0 otherwise.
// Pre stack: addr
// Post stack: is_just_created
%macro contract_just_created
    // stack: addr
    %mload_global_metadata(@GLOBAL_METADATA_CREATED_CONTRACTS_LEN)
    // stack: nb_created_contracts, addr
    PUSH 0
%%contract_just_created_loop:
    %stack (i, nb_created_contracts, addr) -> (i, nb_created_contracts, i, nb_created_contracts, addr)
    EQ %jumpi(%%contract_just_created_false)
    // stack: i, nb_created_contracts, addr
    DUP1 %mload_kernel(@SEGMENT_CREATED_CONTRACTS)
    // stack: addr_created_contract, i, nb_created_contracts, addr
    DUP4
    // stack: addr, addr_created_contract, i, nb_created_contracts, addr
    EQ %jumpi(%%contract_just_created_true)
    // stack: i, nb_created_contracts, addr
    %increment
    %jump(%%contract_just_created_loop)
%%contract_just_created_true:
    // stack: i, nb_created_contracts, addr
    %pop3
    PUSH 1
    // stack: 1
    %jump(%%after)
%%contract_just_created_false:
    // stack: i, nb_created_contracts, addr
    %pop3
    PUSH 0
    // stack: 0
%%after:
%endmacro
