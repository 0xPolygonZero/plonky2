// Computes the address of a contract based on the conventional scheme, i.e.
//     address = KEC(RLP(sender, nonce))[12:]
//
// Pre stack: sender, nonce, retdest
// Post stack: address
global get_create_address:
    // stack: sender, nonce, retdest
    %alloc_rlp_block
    // stack: rlp_start, sender, nonce, retdest
    %stack (rlp_start, sender, nonce) -> (rlp_start, sender, nonce, rlp_start)
    // stack: rlp_start, sender, nonce, rlp_start, retdest
    %encode_rlp_160 // TODO: or encode_rlp_scalar?
    // stack: rlp_pos, nonce, rlp_start, retdest
    %encode_rlp_scalar
    // stack: rlp_pos, rlp_start, retdest
    %prepend_rlp_list_prefix
    // stack: rlp_prefix_start, rlp_len, retdest
    PUSH @SEGMENT_RLP_RAW
    PUSH 0 // context
    // stack: RLP_ADDR: 3, rlp_len, retdest
    KECCAK_GENERAL
    %mod_const(0x10000000000000000000000000000000000000000) // 2^160
    // stack: address, retdest
    %observe_new_address
    SWAP1
    JUMP

// Convenience macro to call get_create_address and return where we left off.
%macro get_create_address
    %stack (sender, nonce) -> (sender, nonce, %%after)
    %jump(get_create_address)
%%after:
%endmacro

// Computes the address for a contract based on the CREATE2 rule, i.e.
//     address = KEC(0xff || sender || salt || code_hash)[12:]
// Clobbers @SEGMENT_KERNEL_GENERAL.
// Pre stack: sender, salt, code_hash, retdest
// Post stack: address
global get_create2_address:
    // stack: sender, salt, code_hash, retdest
    PUSH 0xff PUSH 0 %mstore_kernel_general
    %stack (sender, salt, code_hash, retdest) -> (0, @SEGMENT_KERNEL_GENERAL, 1, sender, 20, get_create2_address_contd, salt, code_hash, retdest)
    %jump(mstore_unpacking)
global get_create2_address_contd:
    POP
    %stack (salt, code_hash, retdest) -> (0, @SEGMENT_KERNEL_GENERAL, 21, salt, 32, get_create2_address_contd2, code_hash, retdest)
    %jump(mstore_unpacking)
global get_create2_address_contd2:
    POP
    %stack (code_hash, retdest) -> (0, @SEGMENT_KERNEL_GENERAL, 53, code_hash, 32, get_create2_address_finish, retdest)
    %jump(mstore_unpacking)
global get_create2_address_finish:
    POP
    %stack (retdest) -> (0, @SEGMENT_KERNEL_GENERAL, 0, 85, retdest) // context, segment, offset, len
    KECCAK_GENERAL
    // stack: address, retdest
    %mod_const(0x10000000000000000000000000000000000000000) // 2^160
    %observe_new_address
    SWAP1
    JUMP

// This should be called whenever a new address is created. This is only for debugging. It does
// nothing, but just provides a single hook where code can react to newly created addresses.
global observe_new_address:
    // stack: address, retdest
    SWAP1
    // stack: retdest, address
    JUMP

// Convenience macro to call observe_new_address and return where we left off.
%macro observe_new_address
    %stack (address) -> (address, %%after)
    %jump(observe_new_address)
%%after:
%endmacro
