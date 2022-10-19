// use locations in kernel genoral memory

// Return x >= p, where x and p are unbounded integers represented with one-byte limbs.
global ge_unbounded:
    // stack: x_len, p_len, x_0_loc, p_0_loc, retdest
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


// Replaces x with x - p, where x and p are unbounded integers represented with one-byte limbs.
// Leave p unchanged.
// Assumes x >= p.
global sub_unbounded:
    // stack: x_len, p_len, x_0_loc, p_0_loc, retdest

    // Leave the first (x_len - p_len - 1) limbs of x alone, because subtracting p from x doesn't affect them.
    %stack: (xp: 2) -> (xp, xp)
    SUB
    %decrement
    // stack: x_len - p_len - 1, x_len, p_len, x_0_loc, p_0_loc, retdest
    %stack: (diff, xl, pl, xloc) -> (xloc, diff, pl)
    // stack: x_0_loc, x_len - p_len - 1, p_len, p_0_loc, retdest
    ADD
    // stack: x_0_loc', p_len, p_0_loc, retdest
    SWAP1
    // stack: len, x_0_loc', p_0_loc, retdest
sub_loop:
    // stack: cur_len, x_i_loc, p_i_loc, retdest
    %stack: (len, x, p) -> (x, p, len, x, p)
    // stack: x_i_loc, p_i_loc, cur_len, x_i_loc, p_i_loc, retdest
    %mload_kernel_general
    SWAP1
    %mload_kernel_general
    SWAP1
    // stack: x[i], p[i], cur_len, x_i_loc, p_i_loc, retdest
    DUP2
    DUP2
    LT
    %jumpi(sub_loop_less)
sub_loop_ge:
    // stack: x[i], p[i], cur_len, x_i_loc, p_i_loc, retdest
    SUB
    // stack: x[i] - p[i], cur_len, x_i_loc, p_i_loc, retdest
    DUP3
    // stack: x_i_loc, x[i] - p[i], cur_len, x_i_loc, p_i_loc, retdest
    %mstore_kernel_general
    %jump(sub_loop_end)
sub_loop_less:
    // stack: x[i], p[i], cur_len, x_i_loc, p_i_loc, retdest
    PUSH 256
    ADD
    SUB
    // stack: 1 << 8 + x[i] - p[i], cur_len, x_i_loc, p_i_loc, retdest
    DUP3
    // stack: x_i_loc, 1 << 8 + x[i] - p[i], cur_len, x_i_loc, p_i_loc, retdest
    %mstore_kernel_general
    // stack: cur_len, x_i_loc, p_i_loc, retdest
    DUP2
    // stack: x_i_loc, cur_len, x_i_loc, p_i_loc, retdest
    %decrement
    // stack: x_i_loc - 1, cur_len, x_i_loc, p_i_loc, retdest
    DUP1
    %mload_kernel_general
    // stack: x[i-1], x_i_loc - 1, cur_len, x_i_loc, p_i_loc, retdest
decrement_carry_loop:
    DUP1
    ISZERO
    NOT
    %jumpi(decrement_carry_end)
    // stack: x[i-k], x_i_loc - k, cur_len, x_i_loc, p_i_loc, retdest
    POP
    // stack: x_i_loc - k, cur_len, x_i_loc, p_i_loc, retdest
    %decrement
    // stack: x_i_loc - k - 1, cur_len, x_i_loc, p_i_loc, retdest
    DUP1
    %mload_kernel_general
    // stack: x[i-k-1], x_i_loc - k - 1, cur_len, x_i_loc, p_i_loc, retdest
    %jump(decrement_carry_loop)
decrement_carry_end:
    // stack: x[i-k], x_i_loc - k, cur_len, x_i_loc, p_i_loc, retdest
    %decrement
    // stack: x[i-k] - 1, x_i_loc - k, cur_len, x_i_loc, p_i_loc, retdest
    SWAP1
    // stack: x_i_loc - k, x[i-k] - 1, cur_len, x_i_loc, p_i_loc, retdest
    %mstore_kernel_general
    %jump(sub_loop_end)
sub_loop_end:
    // 
sub_end:


    
    // restict to lowest p_len limbs of x!
    // loop for each limb:
    //      if ge, subtract
    //      if smaller
    //            add 1<<8, subtract
    //            take one from previous
        
    


// Return x % p, where x and p are unbounded integers represented with one-byte limbs.
global mod_unbounded:
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
