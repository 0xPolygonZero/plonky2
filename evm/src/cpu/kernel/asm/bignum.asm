// Return a >= b, where a and b are unbounded big-endian integers represented with one-byte limbs.
global ge_bignum:
    // stack: a_len, b_len, a_start_loc, b_start_loc, retdest
    %stack (lens: 2) -> (lens, lens)
    GT
    %jumpi(greater)
    %stack (lens: 2) -> (lens, lens)
    LT
    %jumpi(less)
eq_loop:
    // stack: a_len-i, b_len-i, a_i_loc, b_i_loc, retdest
    %decrement
    SWAP1
    %decrement
    SWAP1
    // stack: a_len-i-1, b_len-i-1, a_i_loc, b_i_loc, retdest
    %stack (lens: 2, locs: 2) -> (locs, lens)
    // stack: a_i_loc, b_i_loc, a_len-i-1, b_len-i-1, retdest
    %stack (locs: 2) -> (locs, locs)
    // stack: a_i_loc, b_i_loc, a_i_loc, b_i_loc, a_len-i-1, b_len-i-1, retdest
    %mload_kernel_general
    SWAP1
    %mload_kernel_general
    SWAP1
    // stack: a[i], b[i], a_i_loc, b_i_loc, a_len-i-1, b_len-i-1, retdest
    %stack (vals: 2) -> (vals, vals)
    GT
    %jumpi(greater)
    // stack: a[i], b[i], a_i_loc, b_i_loc, a_len-i-1, b_len-i-1, retdest
    %stack (vals: 2) -> (vals, vals)
    LT
    %jumpi(less)
    // stack: a[i], b[i], a_i_loc, b_i_loc, a_len-i-1, b_len-i-1, retdest
    %stack (vals: 2) -> ()
    // stack: a_i_loc, b_i_loc, a_len-i-1, b_len-i-1, retdest
    %increment
    SWAP1
    %increment
    SWAP1
    // stack: a_i_loc+1, b_i_loc_1, a_len-i-1, b_len-i-1, retdest
    %jump(eq_loop)
greater:
    // stack: a_len, a_0_loc, b_len, b_0_loc, retdest
    %stack (all: 4) -> (1)
    // stack: 1, retdest
    SWAP1
    JUMP
less:
    // stack: a_len, a_0_loc, b_len, b_0_loc, retdest
    %stack (all: 4) -> (0)
    // stack: 0, retdest
    SWAP1
    JUMP

// Replaces a with a + b, where a and b are unbounded big-endian integers represented with one-byte limbs.
// Leave b unchanged.
global add_bignum:
    // stack: a_len, b_len, a_start_loc, b_start_loc, retdest
    %stack (al, bl, a, b) -> (al, a, bl, b, bl)
    // stack: a_len, a_start_loc, b_len, b_start_loc, b_len, retdest
    ADD
    %decrement
    // stack: a_end_loc, b_len, b_start_loc, b_len, retdest
    %stack (a, bl, b, bl) -> (bl, b, bl, a)
    // stack: b_len, b_start_loc, b_len, a_end_loc, retdest
    ADD
    %decrement
    // stack: b_end_loc, b_len, a_end_loc, retdest
    %stack (be, bl, ae) -> (0, 0, ae, be, bl)
    // stack: carry=0, i=0, a_end_loc, b_end_loc, n=b_len, retdest
add_loop:
    // stack: carry, i, a_i_loc, b_i_loc, n, retdest
    DUP4
    DUP4
    // stack: a_i_loc, b_i_loc, carry, i, a_i_loc, b_i_loc, n, retdest
    %mload_kernel_general
    SWAP1
    %mload_kernel_general
    SWAP1
    // stack: a[i], b[i], carry, i, a_i_loc, b_i_loc, n, retdest
    ADD
    ADD
    // stack: a[i] + b[i] + carry, i, a_i_loc, b_i_loc, n, retdest
    %stack (val) -> (val, 256, 256, val)
    // stack: a[i] + b[i] + carry, 256, 256, a[i] + b[i] + carry, i, a_i_loc, b_i_loc, n, retdest
    DIV
    // stack: (a[i] + b[i] + carry) // 256, 256, a[i] + b[i] + carry, i, a_i_loc, b_i_loc, n, retdest
    SWAP2
    // stack: a[i] + b[i] + carry, 256, (a[i] + b[i] + carry) // 256, i, a_i_loc, b_i_loc, n, retdest
    MOD
    // stack: c[i] = (a[i] + b[i] + carry) % 256, carry_new = (a[i] + b[i] + carry) // 256, i, a_i_loc, b_i_loc, n, retdest
    DUP4
    // stack: a_i_loc, c[i], carry_new, i, a_i_loc, b_i_loc, n, retdest
    %mstore_kernel_general
    // stack: carry_new, i, a_i_loc, b_i_loc, n, retdest
    %stack (c, i, a, b) -> (a, b, c, i)
    // stack: a_i_loc, b_i_loc, carry_new, i, n, retdest
    %decrement
    SWAP1
    %decrement
    SWAP1
    %stack (a, b, c, i) -> (c, i, a, b)
    // stack: carry_new, i, a_i_loc - 1, b_i_loc - 1, n, retdest
    SWAP1
    %increment
    SWAP1
    // stack: carry_new, i + 1, a_i_loc - 1, b_i_loc - 1, n, retdest
    DUP5
    DUP3
    // stack: i + 1, n, carry_new, i + 1, a_i_loc - 1, b_i_loc - 1, n, retdest
    EQ
    %not_bool
    %jumpi(add_loop)
add_end:
    STOP
    // stack: carry_new, i + 1, a_i_loc - 1, b_i_loc - 1, n, retdest
    %stack (c, i, a, b, n) -> (c, a)
    // stack: carry_new, a_i_loc - 1, retdest
    // If carry = 0, no need to decrement.
    ISZERO
    %jumpi(increment_end)
increment_loop:
    // stack: cur_loc, retdest
    DUP1
    %mload_kernel_general
    // stack: val, cur_loc, retdest
    %increment
    // stack: val+1, cur_loc, retdest
    %stack (v, l) -> (l, v, l, v)
    DUP2
    // stack: cur_loc, val+1, cur_loc, val+1, retdest
    %mstore_kernel_general
    // stack: cur_loc, val+1, retdest
    %decrement
    // stack: cur_loc - 1, val+1, retdest
    SWAP1
    // stack: val+1, cur_loc - 1, retdest
    %eq_const(256)
    NOT
    %jumpi(increment_end)
    // stack: cur_loc - 1, retdest
    PUSH 0
    DUP2
    // stack: cur_loc - 1, 0, cur_loc - 1, retdest
    %mstore_kernel_general
    %jump(increment_loop)
increment_end:
    // cur_loc, retdest
    POP
    // retdest
    JUMP

%macro subtract_limb
    // stack: a_i, b_i, borrow
    DUP3
    DUP2
    SUB
    // stack: a_i - borrow, a_i, b_i, borrow
    DUP3
    // stack: b_i, a_i - borrow, a_i, b_i, borrow
    GT
    // stack: borrow_new, a_i, b_i, borrow
    DUP1
    PUSH 256
    MUL
    // stack: to_add, borrow_new, a_i, b_i, borrow
    %stack (t, bn, other: 3) -> (t, other, bn)
    // stack: to_add, a_i, b_i, borrow, borrow_new
    ADD
    SUB
    SUB
    // stack: c_i, borrow_new
%endmacro

// Replaces a with a - b, where a and b are unbounded big-endian integers represented with one-byte limbs.
// Leave b unchanged.
// Assumes a >= b.
global sub_bignum:
    // stack: a_len, b_len, a_start_loc, b_start_loc, retdest
    %stack (al, bl, a, b) -> (al, a, bl, b, bl)
    // stack: a_len, a_start_loc, b_len, b_start_loc, b_len, retdest
    ADD
    %decrement
    // stack: a_end_loc, b_len, b_start_loc, b_len, retdest
    %stack (a, bl, b, bl) -> (bl, b, bl, a)
    // stack: b_len, b_start_loc, b_len, a_end_loc, retdest
    ADD
    %decrement
    // stack: b_end_loc, b_len, a_end_loc, retdest
    %stack (be, bl, ae) -> (0, 0, ae, be, bl)
    // stack: borrow=0, i=0, a_end_loc, b_end_loc, n=b_len, retdest
sub_loop:
    // stack: borrow, i, a_i_loc, b_i_loc, n, retdest
    DUP4
    DUP4
    // stack: a_i_loc, b_i_loc, borrow, i, a_i_loc, b_i_loc, n, retdest
    %mload_kernel_general
    SWAP1
    %mload_kernel_general
    SWAP1
    // stack: a[i], b[i], borrow, i, a_i_loc, b_i_loc, n, retdest
    %subtract_limb
    // stack: c[i], borrow_new, i, a_i_loc, b_i_loc, n, retdest
    DUP4
    // stack: a_i_loc, c[i], borrow_new, i, a_i_loc, b_i_loc, n, retdest
    %mstore_kernel_general
    // stack: borrow_new, i, a_i_loc, b_i_loc, n, retdest
    %stack (bn, i, a, b) -> (a, b, bn, i)
    // stack: a_i_loc, b_i_loc, borrow_new, i, n, retdest
    %decrement
    SWAP1
    %decrement
    SWAP1
    %stack (a, b, bn, i) -> (bn, i, a, b)
    // stack: borrow_new, i, a_i_loc - 1, b_i_loc - 1, n, retdest
    SWAP1
    %increment
    SWAP1
    // stack: borrow_new, i + 1, a_i_loc - 1, b_i_loc - 1, n, retdest
    DUP5
    DUP3
    // stack: i + 1, n, borrow_new, i + 1, a_i_loc - 1, b_i_loc - 1, n, retdest
    EQ
    %not_bool
    %jumpi(sub_loop)
sub_end:
    // stack: borrow_new, i + 1, a_i_loc - 1, b_i_loc - 1, n, retdest
    %stack (bn, i, a, b, n) -> (bn, a)
    // stack: borrow_new, a_i_loc - 1, retdest
    // If borrow = 0, no need to decrement.
    ISZERO
    %jumpi(decrement_end)
decrement_loop:
    // If borrow = 1, we need to subtract 1 from the prior limb of a.
    // stack: cur_loc, retdest
    DUP1
    %mload_kernel_general
    // stack: val, cur_loc, retdest
    %decrement
    // stack: val-1, cur_loc, retdest
    %stack (v, l) -> (l, v, l, v)
    DUP2
    // stack: cur_loc, val-1, cur_loc, val-1, retdest
    %mstore_kernel_general
    // stack: cur_loc, val-1, retdest
    %decrement
    // stack: cur_loc - 1, val-1, retdest
    SWAP1
    // stack: val-1, cur_loc - 1, retdest
    %increment
    %eq_const(0)
    NOT
    %jumpi(decrement_end)
    // stack: cur_loc - 1, retdest
    PUSH 255
    DUP2
    // stack: cur_loc - 1, 0, cur_loc - 1, retdest
    %mstore_kernel_general
    %jump(decrement_loop)
decrement_end:
    // cur_loc, retdest
    POP
    // retdest
    JUMP


    
    // restict to lowest p_len limbs of x!
    // loop for each limb:
    //      if ge, subtract
    //      if smaller
    //            add 1<<8, subtract
    //            take one from previous
        
    


//// Return x % p, where x and p are unbounded integers represented with one-byte limbs.
//global mod_bignum:
//    // stack: x_len, p_len, x[0], ..., x[x_len], p[0], ..., p[p_len]
//    // stack: x_len, p_len, x_0_loc, p_0_loc, retdest
//
//    
//    // save both to memory
//global mod_unbounded_inner:
//    // call 
//
//global mod_unbounded_inner:
//
//    // while x > p:
//        x -= p
//    
//
//
//
///// Recursive implementation of exp.
///// Equivalent to:
/////     def modexp(x, e, p):
/////         if e == 0:
/////             # The path where JUMPI does not jump to `step_case`
/////             return 1
/////         else:
/////             # This is under the `step_case` label
//                let res = (x if e % 2 else 1) * exp(x * x, e // 2)
//                return res % p if 
//                if res > p:
//                    return res % p
//                    
//
/////             return 
///// Note that this correctly handles exp(0, 0) == 1.
//
//global modexp:
//    // stack: x, e, retdest
//    dup2
//    // stack: e, x, e, retdest
//    %jumpi(step_case)
//    // stack: x, e, retdest
//    pop
//    // stack: e, retdest
//    pop
//    // stack: retdest
//    push 1
//    // stack: 1, retdest
//    swap1
//    // stack: retdest, 1
//    jump
//
//step_case:
//    // stack: x, e, retdest
//    push recursion_return
//    // stack: recursion_return, x, e, retdest
//    push 2
//    // stack: 2, recursion_return, x, e, retdest
//    dup4
//    // stack: e, 2, recursion_return, x, e, retdest
//    div
//    // stack: e / 2, recursion_return, x, e, retdest
//    dup3
//    // stack: x, e / 2, recursion_return, x, e, retdest
//    %square
//    // stack: x * x, e / 2, recursion_return, x, e, retdest
//    %jump(exp)
//recursion_return:
//    // stack: exp(x * x, e / 2), x, e, retdest
//    push 2
//    // stack: 2, exp(x * x, e / 2), x, e, retdest
//    dup4
//    // stack: e, 2, exp(x * x, e / 2), x, e, retdest
//    mod
//    // stack: e % 2, exp(x * x, e / 2), x, e, retdest
//    push 1
//    // stack: 1, e % 2, exp(x * x, e / 2), x, e, retdest
//    dup4
//    // stack: x, 1, e % 2, exp(x * x, e / 2), x, e, retdest
//    sub
//    // stack: x - 1, e % 2, exp(x * x, e / 2), x, e, retdest
//    mul
//    // stack: (x - 1) * (e % 2), exp(x * x, e / 2), x, e, retdest
//    push 1
//    // stack: 1, (x - 1) * (e % 2), exp(x * x, e / 2), x, e, retdest
//    add
//    // stack: 1 + (x - 1) * (e % 2), exp(x * x, e / 2), x, e, retdest
//    mul
//    // stack: (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2), x, e, retdest
//    swap3
//    // stack: retdest, x, e, (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2)
//    swap2
//    // stack: e, x, retdest, (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2)
//    pop
//    // stack: x, retdest, (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2)
//    pop
//    // stack: retdest, (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2)
//    jump
//
//global sys_exp:
//    PANIC
