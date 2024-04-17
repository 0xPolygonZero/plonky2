// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Sets a[0:len] += b[0:len] * val, and returns the carry (a limb of up to 128 bits).
global addmul_bignum:
    // stack: len, a_start_loc, b_start_loc, val, retdest
    DUP1
    // stack: len, len, a_start_loc, b_start_loc, val, retdest
    ISZERO
    %jumpi(len_zero)
    PUSH 0
    // stack: carry_limb=0, i=len, a_cur_loc=a_start_loc, b_cur_loc=b_start_loc, val, retdest
addmul_loop:
    // stack: carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP4
    // stack: b_cur_loc, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    %mload_current_general
    // stack: b[cur], carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP6
    // stack: val, b[cur], carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    MUL
    // stack: val * b[cur], carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP1
    // stack: val * b[cur], val * b[cur], carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    %shr_const(128)
    // stack: (val * b[cur]) // 2^128, val * b[cur], carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    SWAP1
    // stack: val * b[cur], (val * b[cur]) // 2^128, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    %shl_const(128)
    %shr_const(128)
    // stack: prod_lo = val * b[cur] % 2^128, prod_hi = (val * b[cur]) // 2^128, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP5
    // stack: a_cur_loc, prod_lo, prod_hi, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    %mload_current_general
    // stack: a[cur], prod_lo, prod_hi, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP1
    // stack: a[cur], a[cur], prod_lo, prod_hi, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    SWAP2
    // stack: prod_lo, a[cur], a[cur], prod_hi, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    ADD
    %shl_const(128)
    %shr_const(128)
    // stack: prod_lo' = (prod_lo + a[cur]) % 2^128, a[cur], prod_hi, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP1
    // stack: prod_lo', prod_lo', a[cur], prod_hi, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    SWAP2
    // stack: a[cur], prod_lo', prod_lo', prod_hi, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    GT
    // stack: prod_lo_carry_limb = a[cur] > prod_lo', prod_lo', prod_hi, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    SWAP1
    // stack: prod_lo', prod_lo_carry_limb, prod_hi, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    SWAP2
    // stack: prod_hi, prod_lo_carry_limb, prod_lo', carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    ADD
    // stack: prod_hi' = prod_hi + prod_lo_carry_limb, prod_lo', carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP3
    // stack: carry_limb, prod_hi', prod_lo', carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP3
    // stack: prod_lo', carry_limb, prod_hi', prod_lo', carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    ADD
    %shl_const(128)
    %shr_const(128)
    // stack: to_write = (prod_lo' + carry_limb) % 2^128, prod_hi', prod_lo', carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    SWAP2
    // stack: prod_lo', prod_hi', to_write, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP3
    // stack: to_write, prod_lo', prod_hi', to_write, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    LT
    // stack: carry_limb_new = to_write < prod_lo', prod_hi', to_write, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    %stack (vals: 3, c) -> (vals)
    // stack: carry_limb_new, prod_hi', to_write, i, a_cur_loc, b_cur_loc, val, retdest
    ADD
    // stack: carry_limb = carry_limb_new' + prod_hi', to_write, i, a_cur_loc, b_cur_loc, val, retdest
    SWAP1
    // stack: to_write, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    DUP4
    // stack: a_cur_loc, to_write, carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    %mstore_current_general
    // stack: carry_limb, i, a_cur_loc, b_cur_loc, val, retdest
    SWAP1
    // stack: i, carry_limb, a_cur_loc, b_cur_loc, val, retdest
    %decrement
    // stack: i-1, carry_limb, a_cur_loc, b_cur_loc, val, retdest
    SWAP2
    // stack: a_cur_loc, carry_limb, i-1, b_cur_loc, val, retdest
    %increment
    // stack: a_cur_loc+1, carry_limb, i-1, b_cur_loc, val, retdest
    SWAP3
    // stack: b_cur_loc, carry_limb, i-1, a_cur_loc+1, val, retdest
    %increment
    // stack: b_cur_loc+1, carry_limb, i-1, a_cur_loc+1, val, retdest
    %stack (b, c, i, a) -> (c, i, a, b)
    // stack: carry_limb, i-1, a_cur_loc+1, b_cur_loc+1, val, retdest
    DUP2
    // stack: i-1, carry_limb, i-1, a_cur_loc+1, b_cur_loc+1, val, retdest
    %jumpi(addmul_loop)
addmul_end:
    // stack: carry_limb_new, i-1, a_cur_loc+1, b_cur_loc+1, val, retdest
    %stack (c, i, a, b, v) -> (c)
    // stack: carry_limb_new, retdest
    SWAP1
    // stack: retdest, carry_limb_new
    JUMP

len_zero:
    // stack: len, a_start_loc, b_start_loc, val, retdest
    %pop4
    // stack: retdest
    PUSH 0
    // stack: carry_limb=0, retdest
    SWAP1
    JUMP
