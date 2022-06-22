global exp:
// we don't seem to handle global labels yet, so this function has a local label too for now:
exp:
    // stack: x, e, retdest
    dup2
    // stack: e, x, e, retdest
    push step_case
    // stack: step_case, e, x, e, retdest
    jumpi
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
    dup1
    // stack: x, x, e / 2, recursion_return, x, e, retdest
    mul
    // stack: x * x, e / 2, recursion_return, x, e, retdest
    push exp
    // stack: exp, x * x, e / 2, recursion_return, x, e, retdest
    jump
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
    // stack: retdest, x, e, (x / (e % 2)) * exp(x * x, e / 2)
    swap2
    // stack: e, x, retdest, (x / (e % 2)) * exp(x * x, e / 2)
    pop
    // stack: x, retdest, (x / (e % 2)) * exp(x * x, e / 2)
    pop
    // stack: retdest, (x / (e % 2)) * exp(x * x, e / 2)
    jump
