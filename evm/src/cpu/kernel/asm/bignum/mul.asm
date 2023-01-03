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
bignum_reduce_helper:
    // stack: length, start_loc, retdest
    %stack (vals: 2) -> (vals, 0)
    // stack: n=length, i=start_loc, carry=0, retdest
reduce_loop:
    // stack: n, i, retdest
    DUP2
    // stack: i, n, i, retdest
    %mload_kernel_general
    // stack: bignum[i], n, i, retdest
    // stack: n, i, retdest
    %decrement
    SWAP1
    %increment
    SWAP1
    // stack: n - 1, i + 1, carry, retdest
    DUP1
    // stack: n - 1, n - 1, i + 1, carry, retdest
    ISZERO
    %jumpi(reduce_end)
    %jump(reduce_loop)
reduce_end:
    // stack: n = 0, i, carry, retdest
    %stack (vals: 3) -> ()
    // stack: retdest
    JUMP

// Stores a * b in output_loc, leaving a and b unchanged.
global mul_bignum:
    // stack: length, a_start_loc, b_start_loc, output_loc, retdest
    
mul_loop:

mul_end:
