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
    %jump(load_code)

// Loads the code at `address` into memory, in the code segment of the given context, starting at offset 0.
// Checks that the hash of the loaded code corresponds to the `codehash` in the state trie.
// Pre stack: address, ctx, retdest
// Post stack: code_size
//
// NOTE: The provided `dest` **MUST** have a virtual address of 0.
global load_code:
    %stack (address, ctx, retdest) -> (extcodehash, address, load_code_ctd, ctx, retdest)
    JUMP
load_code_ctd:
    // stack: codehash, ctx, retdest
    DUP1 ISZERO %jumpi(load_code_non_existent_account)
    // Load the code non-deterministically in memory and return the length.
    PROVER_INPUT(account_code)
    %stack (code_size, codehash, ctx, retdest) -> (ctx, code_size, codehash, retdest, code_size)
    // Check that the hash of the loaded code equals `codehash`.
    // ctx == DST, as SEGMENT_CODE == offset == 0.
    KECCAK_GENERAL
    // stack: shouldbecodehash, codehash, retdest, code_size
    %assert_eq
    // stack: retdest, code_size
    JUMP

load_code_non_existent_account:
    // Write 0 at address 0 for soundness: SEGMENT_CODE == 0, hence ctx == addr.
    // stack: codehash, addr, retdest
    %stack (codehash, addr, retdest) -> (0, addr, retdest, 0)
    MSTORE_GENERAL
    // stack: retdest, 0
    JUMP

// Identical to load_code, but adds 33 zeros after code_size for soundness reasons.
// If the code ends with an incomplete PUSH, we must make sure that every subsequent read is 0,
// accordingly to the Ethereum specs.
// Pre stack: address, ctx, retdest
// Post stack: code_size
global load_code_padded:
    %stack (address, ctx, retdest) -> (address, ctx, load_code_padded_ctd, ctx, retdest)
    %jump(load_code)

load_code_padded_ctd:
    // SEGMENT_CODE == 0.
    // stack: code_size, ctx, retdest
    %stack (code_size, ctx, retdest) -> (ctx, code_size, 0, retdest, code_size)
    ADD 
    // stack: addr, 0, retdest, code_size
    MSTORE_32BYTES_32
    // stack: addr', retdest, code_size
    PUSH 0
    MSTORE_GENERAL
    // stack: retdest, code_size
    JUMP
