// use locations in kernel genoral memory

// Return x >= p, where x and p are unbounded big-endian integers represented with one-byte limbs.
global ge_bignum:
    // stack: x_len, p_len, x_start_loc, p_start_loc, retdest
    %stack: (lens: 2) -> (lens, lens)
    GT
    %jumpi(greater)
    %stack: (lens: 2) -> (lens, lens)
    LT
    %jumpi(less)
eq_loop:
    // stack: x_len-i, p_len-i, x_i_loc, p_i_loc, retdest
    %decrement
    SWAP1
    %decrement
    SWAP1
    // stack: x_len-i-1, p_len-i-1, x_i_loc, p_i_loc, retdest
    %stack (lens: 2, locs: 2) -> (locs, lens)
    // stack: x_i_loc, p_i_loc, x_len-i-1, p_len-i-1, retdest
    %stack: (locs: 2) -> (locs, locs)
    // stack: x_i_loc, p_i_loc, x_i_loc, p_i_loc, x_len-i-1, p_len-i-1, retdest
    %mload_kernel_general
    SWAP1
    %mload_kernel_general
    SWAP1
    // stack: x[i], p[i], x_i_loc, p_i_loc, x_len-i-1, p_len-i-1, retdest
    %stack: (vals: 2) -> (vals, vals)
    GT
    %jumpi(greater)
    // stack: x[i], p[i], x_i_loc, p_i_loc, x_len-i-1, p_len-i-1, retdest
    %stack: (vals: 2) -> (vals, vals)
    LT
    %jumpi(less)
    // stack: x[i], p[i], x_i_loc, p_i_loc, x_len-i-1, p_len-i-1, retdest
    %stack: (vals: 2) -> ()
    // stack: x_i_loc, p_i_loc, x_len-i-1, p_len-i-1, retdest
    %increment
    SWAP1
    %increment
    SWAP1
    // stack: x_i_loc+1, p_i_loc_1, x_len-i-1, p_len-i-1, retdest
    %jump(eq_loop)
greater:
    // stack: x_len, x_0_loc, p_len, p_0_loc, retdest
    %stack (all: 4) -> (1)
    // stack: 1, retdest
    SWAP1
    JUMP
less:
    // stack: x_len, x_0_loc, p_len, p_0_loc, retdest
    %stack (all: 4) -> (0)
    // stack: 0, retdest
    SWAP1
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
    SWAP1
    // stack: b_len, b_end_loc, a_end_loc, retdest
    %increment
    // stack: n, b_end_loc, a_end_loc, retdest
    SWAP2
    // stack: a_end_loc, b_end_loc, n, retdest
    %stack () -> (0, 0)
    // stack: borrow=0, i=0, a_end_loc, b_end_loc, n, retdest
sub_loob:
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
    DUP3
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
    LE
    %jumpi(sub_loop)
sub_end:
    // stack: borrow_new, i + 1, a_i_loc - 1, b_i_loc - 1, n, retdest

decrement_loop:

decrement_end:
    // subtract 


    
    // restict to lowest p_len limbs of x!
    // loop for each limb:
    //      if ge, subtract
    //      if smaller
    //            add 1<<8, subtract
    //            take one from previous
        
    


// Return x % p, where x and p are unbounded integers represented with one-byte limbs.
global mod_bignum:
    // stack: x_len, p_len, x[0], ..., x[x_len], p[0], ..., p[p_len]
    // stack: x_len, p_len, x_0_loc, p_0_loc, retdest

    
    // save both to memory
global mod_unbounded_inner:
    // call 

global mod_unbounded_inner:

    // while x > p:
        x -= p
    



/// Recursive implementation of exp.
/// Equivalent to:
///     def modexp(x, e, p):
///         if e == 0:
///             # The path where JUMPI does not jump to `step_case`
///             return 1
///         else:
///             # This is under the `step_case` label
                let res = (x if e % 2 else 1) * exp(x * x, e // 2)
                return res % p if 
                if res > p:
                    return res % p
                    

///             return 
/// Note that this correctly handles exp(0, 0) == 1.

global modexp:
    // stack: x, e, retdest
    dup2
    // stack: e, x, e, retdest
    %jumpi(step_case)
    // stack: x, e, retdest
    pop
    // stack: e, retdest
    pop
    // stack: retdest
    push 1
    // stack: 1, retdest
    swap1
    // stack: retdest, 1
    jump

step_case:
    // stack: x, e, retdest
    push recursion_return
    // stack: recursion_return, x, e, retdest
    push 2
    // stack: 2, recursion_return, x, e, retdest
    dup4
    // stack: e, 2, recursion_return, x, e, retdest
    div
    // stack: e / 2, recursion_return, x, e, retdest
    dup3
    // stack: x, e / 2, recursion_return, x, e, retdest
    %square
    // stack: x * x, e / 2, recursion_return, x, e, retdest
    %jump(exp)
recursion_return:
    // stack: exp(x * x, e / 2), x, e, retdest
    push 2
    // stack: 2, exp(x * x, e / 2), x, e, retdest
    dup4
    // stack: e, 2, exp(x * x, e / 2), x, e, retdest
    mod
    // stack: e % 2, exp(x * x, e / 2), x, e, retdest
    push 1
    // stack: 1, e % 2, exp(x * x, e / 2), x, e, retdest
    dup4
    // stack: x, 1, e % 2, exp(x * x, e / 2), x, e, retdest
    sub
    // stack: x - 1, e % 2, exp(x * x, e / 2), x, e, retdest
    mul
    // stack: (x - 1) * (e % 2), exp(x * x, e / 2), x, e, retdest
    push 1
    // stack: 1, (x - 1) * (e % 2), exp(x * x, e / 2), x, e, retdest
    add
    // stack: 1 + (x - 1) * (e % 2), exp(x * x, e / 2), x, e, retdest
    mul
    // stack: (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2), x, e, retdest
    swap3
    // stack: retdest, x, e, (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2)
    swap2
    // stack: e, x, retdest, (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2)
    pop
    // stack: x, retdest, (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2)
    pop
    // stack: retdest, (1 + (x - 1) * (e % 2)) * exp(x * x, e / 2)
    jump

global sys_exp:
    PANIC
