global sys_extcodehash:
    // stack: kexit_info, address
    SWAP1 %u256_to_addr
    // stack: address, kexit_info
    DUP1 %insert_accessed_addresses
    // stack: cold_access, address, kexit_info
    PUSH @GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS
    MUL
    PUSH @GAS_WARMACCESS
    ADD
    %stack (gas, address, kexit_info) -> (gas, kexit_info, address)
    %charge_gas
    // stack: kexit_info, address

    SWAP1
    DUP1 %is_dead %jumpi(extcodehash_dead)
    %extcodehash
    // stack: hash, kexit_info
    SWAP1
    EXIT_KERNEL
extcodehash_dead:
    %stack (address, kexit_info) -> (kexit_info, 0)
    EXIT_KERNEL

global extcodehash:
    // stack: address, retdest
    %mpt_read_state_trie
    // stack: account_ptr, retdest
    DUP1 ISZERO %jumpi(retzero)
    %add_const(3)
    // stack: codehash_ptr, retdest
    %mload_trie_data
    // stack: codehash, retdest
    SWAP1 JUMP
retzero:
    %stack (account_ptr, retdest) -> (retdest, 0)
    JUMP

%macro extcodehash
    %stack (address) -> (address, %%after)
    %jump(extcodehash)
%%after:
%endmacro

%macro ext_code_empty
    %extcodehash
    %eq_const(@EMPTY_STRING_HASH)
%endmacro

%macro extcodesize
    %stack (address) -> (address, %%after)
    %jump(extcodesize)
%%after:
%endmacro

global sys_extcodesize:
    // stack: kexit_info, address
    SWAP1 %u256_to_addr
    // stack: address, kexit_info
    DUP1 %insert_accessed_addresses
    // stack: cold_access, address, kexit_info
    PUSH @GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS
    MUL
    PUSH @GAS_WARMACCESS
    ADD
    %stack (gas, address, kexit_info) -> (gas, kexit_info, address)
    %charge_gas
    // stack: kexit_info, address

    SWAP1
    // stack: address, kexit_info
    %extcodesize
    // stack: code_size, kexit_info
    SWAP1
    EXIT_KERNEL

global extcodesize:
    // stack: address, retdest
    %next_context_id
    // stack: codesize_ctx, address, retdest
    SWAP1
    // stack: address, codesize_ctx, retdest
    %jump(load_code_initial)

// Loads the code at `address` into memory, at the given context and segment, starting at offset 0.
// Checks that the hash of the loaded code corresponds to the `codehash` in the state trie.
// Pre stack: address, ctx, segment, retdest
// Post stack: code_size
global load_code:
    %stack (address, ctx, segment, retdest) -> (extcodehash, address, load_code_ctd, ctx, segment, retdest)
    JUMP
load_code_ctd:
    // stack: codehash, ctx, segment, retdest
    DUP1 ISZERO %jumpi(load_code_non_existent_account)
    PROVER_INPUT(account_code::length)
    // stack: code_size, codehash, ctx, segment, retdest
    PUSH 0

// Loop non-deterministically querying `code[i]` and storing it in `SEGMENT_KERNEL_ACCOUNT_CODE`
// at offset `i`, until `i==code_size`.
load_code_loop:
    // stack: i, code_size, codehash, ctx, segment, retdest
    DUP2 DUP2 EQ
    // stack: i == code_size, i, code_size, codehash, ctx, segment, retdest
    %jumpi(load_code_check)
    DUP1
    // stack: i, i, code_size, codehash, ctx, segment, retdest
    DUP6 // segment
    DUP6 // context
    PROVER_INPUT(account_code::get)
    // stack: opcode, context, segment, i, i, code_size, codehash, ctx, segment, retdest
    MSTORE_GENERAL
    // stack: i, code_size, codehash, ctx, segment, retdest
    %increment
    // stack: i+1, code_size, codehash, ctx, segment, retdest
    %jump(load_code_loop)

// Check that the hash of the loaded code equals `codehash`.
load_code_check:
    // stack: i, code_size, codehash, ctx, segment, retdest
    %stack (i, code_size, codehash, ctx, segment, retdest)
        -> (ctx, segment, 0, code_size, codehash, retdest, code_size)
    KECCAK_GENERAL
    // stack: shouldbecodehash, codehash, retdest, code_size
    %assert_eq
    JUMP

load_code_non_existent_account:
    %stack (codehash, ctx, segment, retdest) -> (retdest, 0)
    JUMP

// Loads the code at `address` into memory, at the given context in the code segment, starting at offset 0.
// Checks that the hash of the loaded code corresponds to the `codehash` in the state trie.
// Pre stack: address, ctx, retdest
// Post stack: code_size
global load_code_initial:
    %stack (address, ctx, retdest) -> (extcodehash, address, load_code_initial_ctd, ctx, retdest)
    JUMP
load_code_initial_ctd:
    // stack: codehash, ctx, retdest
    DUP1 ISZERO %jumpi(load_code_initial_non_existent_account)
    // Load the code non-deterministically in memory and return the length.
    PROVER_INPUT(initialize_code)
    %stack (code_size, codehash, ctx, retdest) -> (ctx, @SEGMENT_CODE, 0, code_size, codehash, ctx, retdest, code_size)
    // Check that the hash of the loaded code equals `codehash`.
    KECCAK_GENERAL
    // stack: shouldbecodehash, codehash, ctx, retdest, code_size
    %assert_eq
    // Write 33 zeros after code_size for soundness.
    %stack (ctx, retdest, code_size) -> (ctx, @SEGMENT_CODE, code_size, retdest, code_size)
    %rep 33
        // stack: ctx, segment, i, retdest, code_size
        DUP3 DUP3 DUP3
        PUSH 0
        // stack: 0, ctx, segment, i, ctx, segment, i, retdest, code_size
        MSTORE_GENERAL
        // stack: ctx, segment, i, retdest, code_size
        DUP3 %increment
        // stack: i+1, ctx, segment, i, retdest, code_size
        SWAP3 POP
        // stack: ctx, segment, i+1, retdest, code_size
    %endrep
    // stack: ctx, segment, code_size+32, retdest, code_size
    %pop3
    JUMP

load_code_initial_non_existent_account:
    // Write 0 at address 0 for soundness.
    // stack: codehash, ctx, retdest
    %stack (codehash, ctx, retdest) -> (0, ctx, @SEGMENT_CODE, 0, retdest, 0)
    MSTORE_GENERAL
    // stack: retdest, 0
    JUMP
