// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Multiplies a bignum by a constant value.
bignum_mul_helper:
    // stack: n=length, i=start_loc, val, retdest
mul_helper_loop:
    // stack: n, i, val, retdest
    DUP2
    // stack: i, n, i, val, retdest
    %mload_kernel_general
    // stack: bignum[i], n, i, val, retdest
    DUP4
    // stack: val, bignum[i], n, i, val, retdest
    MUL
    // stack: val * bignum[i], n, i, val, retdest
    DUP3
    // stack: i, val * bignum[i], n, i, val, retdest
    %mstore_kernel_general
    // stack: n, i, val, retdest
    %decrement
    SWAP1
    %increment
    SWAP1
    // stack: n - 1, i + 1, val, retdest
    DUP1
    // stack: n - 1, n - 1, i + 1, val, retdest
    ISZERO
    %jumpi(mul_helper_end)
    %jump(mul_helper_loop)
mul_helper_end:
    // stack: n = 0, i, val, retdest
    %stack (vals: 3) -> ()
    // stack: retdest
    JUMP

// Reduces a bignum with limbs possibly greater than 128 bits to a normalized bignum with length (length + 1).
mul_bignum_reduce_helper:
    // stack: length, start_loc, retdest
    %stack (vals: 2) -> (vals, 0)
    // stack: n=length, i=start_loc, carry=0, retdest
reduce_loop:
    // stack: n, i, retdest
    DUP2
    // stack: i, n, i, retdest
    %mload_kernel_general
    // stack: bignum[i], n, i, retdest
    PUSH 1
    %shl_const(128)
    // stack: 2^128, bignum[i], n, i, retdest
    %stack (mod, val) -> (val, mod, mod, val)
    // stack: bignum[i], 2^128, 2^128, bignum[i], n, i, retdest
    MOD
    // stack: bignum[i] % 2^128, 2^128, bignum[i], n, i, retdest
    SWAP2
    // stack: bignum[i], 2^128, bignum[i] % 2^128, n, i, retdest
    DIV
    // stack: bignum[i] // 2^128, bignum[i] % 2^128, n, i, retdest
    DUP4
    // stack: i, bignum[i] // 2^128, bignum[i] % 2^128, n, i, retdest
    %increment
    // stack: i+1, bignum[i] // 2^128, bignum[i] % 2^128, n, i, retdest
    SWAP1
    // stack: bignum[i] // 2^128, i+1, bignum[i] % 2^128, n, i, retdest
    DUP2
    // stack: i+1, bignum[i] // 2^128, i+1, bignum[i] % 2^128, n, i, retdest
    %mload_kernel_general
    // stack: bignum[i+1], bignum[i] // 2^128, i+1, bignum[i] % 2^128, n, i, retdest
    ADD
    // stack: bignum[i+1] + bignum[i] // 2^128, i+1, bignum[i] % 2^128, n, i, retdest
    SWAP1
    // stack: i+1, bignum[i+1] + bignum[i] // 2^128, bignum[i] % 2^128, n, i, retdest
    %mstore_kernel_general
    // stack: bignum[i] % 2^128, n, i, retdest
    DUP3
    // stack: i, bignum[i] % 2^128, n, i, retdest
    %mstore_kernel_general
    // stack: n, i, retdest
    %decrement
    SWAP1
    %increment
    SWAP1
    // stack: n - 1, i + 1, retdest
    DUP1
    // stack: n - 1, n - 1, i + 1, retdest
    ISZERO
    %jumpi(reduce_end)
    %jump(reduce_loop)
reduce_end:
    // stack: n = 0, i, retdest
    %stack (vals: 2) -> ()
    // stack: retdest
    JUMP

// Stores a * b in output_loc, leaving a and b unchanged.
// Both a and b have given length; a * b will have length 2 * n.
// Scratch space needs to have space for length + 1 limbs available.
global mul_bignum:
    // stack: length, a_start_loc, b_start_loc, output_loc, scratch_space, retdest
    
mul_loop:
    // stack: n, ai, bi, output_loc, retdest

mul_end:
