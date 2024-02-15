global sys_extcodehash:
    // stack: kexit_info, address
    SWAP1 %u256_to_addr
    // stack: address, kexit_info
    SWAP1
    DUP2 %insert_accessed_addresses
    // stack: cold_access, kexit_info, address
    PUSH @GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS
    MUL
    PUSH @GAS_WARMACCESS
    ADD
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
    %key_code %smt_read_state %mload_trie_data
    // stack: codehash, retdest
    SWAP1 JUMP

%macro extcodehash
    %stack (address) -> (address, %%after)
    %jump(extcodehash)
%%after:
%endmacro

%macro ext_code_empty
    %extcodehash
    %eq_const(@EMPTY_STRING_POSEIDON_HASH)
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
    SWAP1
    DUP2 %insert_accessed_addresses
    // stack: cold_access, kexit_info, address
    PUSH @GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS
    MUL
    PUSH @GAS_WARMACCESS
    ADD
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
    // stack: padded_code_size, codehash, ctx, retdest
    %jump(poseidon_hash_code)

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

// TODO: This could certainly be optimized, or implemented directly in the Poseidon Stark.
poseidon_hash_code:
    // stack: padded_code_size, codehash, ctx, retdest
    %stack (padded_code_size, codehash, ctx) -> (0, 0, padded_code_size, ctx, codehash)
poseidon_hash_code_loop:
    // stack: i, capacity, padded_code_size, ctx, codehash, retdest
    DUP3 DUP2 EQ %jumpi(poseidon_hash_code_after)
    %stack (i, capacity, code_size, ctx) -> (i, ctx, i, capacity, code_size, ctx)
    ADD MLOAD_GENERAL
    %stack (b, i, capacity, code_size, ctx) -> (1, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(8) ADD
    %stack (b, i, capacity, code_size, ctx) -> (2, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(16) ADD
    %stack (b, i, capacity, code_size, ctx) -> (3, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(24) ADD
    %stack (b, i, capacity, code_size, ctx) -> (4, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(32) ADD
    %stack (b, i, capacity, code_size, ctx) -> (5, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(40) ADD
    %stack (b, i, capacity, code_size, ctx) -> (6, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(48) ADD

    %stack (b, i, capacity, code_size, ctx) -> (7, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(64) ADD
    %stack (b, i, capacity, code_size, ctx) -> (8, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(72) ADD
    %stack (b, i, capacity, code_size, ctx) -> (9, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(80) ADD
    %stack (b, i, capacity, code_size, ctx) -> (10, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(88) ADD
    %stack (b, i, capacity, code_size, ctx) -> (11, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(96) ADD
    %stack (b, i, capacity, code_size, ctx) -> (12, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(104) ADD
    %stack (b, i, capacity, code_size, ctx) -> (13, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(112) ADD

    %stack (b, i, capacity, code_size, ctx) -> (14, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(128) ADD
    %stack (b, i, capacity, code_size, ctx) -> (15, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(136) ADD
    %stack (b, i, capacity, code_size, ctx) -> (16, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(144) ADD
    %stack (b, i, capacity, code_size, ctx) -> (17, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(152) ADD
    %stack (b, i, capacity, code_size, ctx) -> (18, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(160) ADD
    %stack (b, i, capacity, code_size, ctx) -> (19, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(168) ADD
    %stack (b, i, capacity, code_size, ctx) -> (20, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(176) ADD

    %stack (b, i, capacity, code_size, ctx) -> (21, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(192) ADD
    %stack (b, i, capacity, code_size, ctx) -> (22, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(200) ADD
    %stack (b, i, capacity, code_size, ctx) -> (23, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(208) ADD
    %stack (b, i, capacity, code_size, ctx) -> (24, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(216) ADD
    %stack (b, i, capacity, code_size, ctx) -> (25, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(224) ADD
    %stack (b, i, capacity, code_size, ctx) -> (26, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(232) ADD
    %stack (b, i, capacity, code_size, ctx) -> (27, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(240) ADD
    %stack (B0, i, capacity, code_size, ctx) -> (i, capacity, code_size, ctx, B0)

    %stack (i, capacity, code_size, ctx) -> (28, i, ctx, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL
    %stack (b, i, capacity, code_size, ctx) -> (29, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(8) ADD
    %stack (b, i, capacity, code_size, ctx) -> (30, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(16) ADD
    %stack (b, i, capacity, code_size, ctx) -> (31, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(24) ADD
    %stack (b, i, capacity, code_size, ctx) -> (32, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(32) ADD
    %stack (b, i, capacity, code_size, ctx) -> (33, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(40) ADD
    %stack (b, i, capacity, code_size, ctx) -> (34, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(48) ADD

    %stack (b, i, capacity, code_size, ctx) -> (35, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(64) ADD
    %stack (b, i, capacity, code_size, ctx) -> (36, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(72) ADD
    %stack (b, i, capacity, code_size, ctx) -> (37, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(80) ADD
    %stack (b, i, capacity, code_size, ctx) -> (38, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(88) ADD
    %stack (b, i, capacity, code_size, ctx) -> (39, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(96) ADD
    %stack (b, i, capacity, code_size, ctx) -> (40, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(104) ADD
    %stack (b, i, capacity, code_size, ctx) -> (41, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(112) ADD

    %stack (b, i, capacity, code_size, ctx) -> (42, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(128) ADD
    %stack (b, i, capacity, code_size, ctx) -> (43, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(136) ADD
    %stack (b, i, capacity, code_size, ctx) -> (44, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(144) ADD
    %stack (b, i, capacity, code_size, ctx) -> (45, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(152) ADD
    %stack (b, i, capacity, code_size, ctx) -> (46, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(160) ADD
    %stack (b, i, capacity, code_size, ctx) -> (47, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(168) ADD
    %stack (b, i, capacity, code_size, ctx) -> (48, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(176) ADD

    %stack (b, i, capacity, code_size, ctx) -> (49, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(192) ADD
    %stack (b, i, capacity, code_size, ctx) -> (50, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(200) ADD
    %stack (b, i, capacity, code_size, ctx) -> (51, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(208) ADD
    %stack (b, i, capacity, code_size, ctx) -> (52, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(216) ADD
    %stack (b, i, capacity, code_size, ctx) -> (53, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(224) ADD
    %stack (b, i, capacity, code_size, ctx) -> (54, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(232) ADD
    %stack (b, i, capacity, code_size, ctx) -> (55, i, ctx, b, i, capacity, code_size, ctx)
    ADD ADD MLOAD_GENERAL %shl_const(240) ADD
    %stack (B1, i, capacity, code_size, ctx, B0) -> (B0, B1, capacity, i, code_size, ctx)
    POSEIDON
    %stack (capacity, i, padded_code_size, ctx) -> (i, capacity, padded_code_size, ctx)
    // stack: i, capacity, padded_code_size, ctx, codehash, retdest
    %add_const(56)
    %jump(poseidon_hash_code_loop)

poseidon_hash_code_after:
    // stack: i, capacity, padded_code_size, ctx, codehash, retdest
    %stack (i, capacity, padded_code_size, ctx, codehash) -> (capacity, codehash, padded_code_size, ctx)
    %assert_eq
    // stack: padded_code_size, ctx, retdest
    %decrement
remove_padding_loop:
    // stack: offset, ctx, retdest
    DUP2 DUP2 ADD DUP1 MLOAD_GENERAL
    // stack: code[offset], offset+ctx, offset, ctx, retdest
    SWAP1 PUSH 0 MSTORE_GENERAL
    // stack: code[offset], offset, ctx, retdest
    %and_const(1) %jumpi(remove_padding_after)
    // stack: offset, ctx, retdest
    %decrement %jump(remove_padding_loop)

remove_padding_after:
    %stack (offset, ctx, retdest) -> (retdest, offset)
    JUMP
