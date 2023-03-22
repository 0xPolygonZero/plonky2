/// Recursive implementation of exp.
/// Equivalent to:
///     def exp(x, e):
///         if e == 0:
///             # The path where JUMPI does not jump to `step_case`
///             return 1
///         else:
///             # This is under the `step_case` label
///             return (x if e % 2 else 1) * exp(x * x, e // 2)
/// Note that this correctly handles exp(0, 0) == 1.

global exp:
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
    // x, e, return_info
    push 248
sys_exp_gas_loop:
    // stack: shift, x, e, return_info
    dup3
    dup2
    shr
    // stack: e >> shift, shift, x, e, return_info
    %jumpi(sys_exp_gas_end)
    // stack: shift, x, e, return_info
    %sub_const(8)
    // stack: shift := shift - 8, x, e, return_info
    dup1
    %jumpi(sys_exp_gas_loop)
sys_exp_gas_end:
    // stack: shift_bits, x, e, return_info
    %div_const(8)
    // stack: index_of_nz_byte := shift_bits / 8, x, e, return_info
    %add_const(1)
    // stack: byte_size_of_e := index_of_nz_byte + 1, x, e, return_info
    %mul_const(@GAS_EXPBYTE)
    %add_const(@GAS_EXP)
    // stack: 10 + 50 * byte_size_of_e, x, e, return_info
    %charge_gas

    // x, e, return_info
    swap1
    push sys_exp_return
    swap2
    // x, e, sys_exp_return, return_info
    jump exp
sys_exp_return:
    // stack: pow(x, e), return_info
    swap1
    exit_kernel
