// use locations in kernel genoral memory

// Return x >= p, where x and p are unbounded integers represented with one-byte limbs.
global ge_unbounded:
    // stack: x_len, x_0_loc, p_len, p_0_loc

    // load x_len, p_len
    // compare
    // if same, loop:
    //    load next limb of each
    //    3-way compare
    //    if equal, back to loop
    

// Return x - p, where x and p are unbounded integers represented with one-byte limbs.
// Assumes x >= p.
global sub_unbounded:
    // stack: x_len, x_0_loc, p_len, p_0_loc
    
    // restict to lowest p_len limbs of x???
    // loop for each limb:
    //      if ge, subtract
    //      if smaller
    //            add 1<<8, subtract
    //            take one from previous
        
    


// Return x % p, where x and p are unbounded integers represented with one-byte limbs.
global mod_unbounded:
    // stack: x_len, x[0], ..., x[x_len], p_len, p[0], ..., p[p_len]
    
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
