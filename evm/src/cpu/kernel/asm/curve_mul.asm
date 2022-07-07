// BN254 elliptic curve scalar multiplication.
// Recursive implementation, same algorithm as in `exp.asm`.
global ec_mul:
    // Uncomment for test inputs.
    // PUSH 0xdeadbeef
    // PUSH 0xd
    // PUSH 2
    // PUSH 1
    JUMPDEST
    // stack: x, y, s, retdest
    DUP2
    // stack: y, x, y, s, retdest
    DUP2
    // stack: x, y, x, y, s, retdest
    %ec_isidentity
    // stack: (x,y)==(0,0), x, y, s, retdest
    %jumpi(ret_zero)
    // stack: x, y, s, retdest
    DUP2
    // stack: y, x, y, s, retdest
    DUP2
    // stack: x, y, x, y, s, retdest
    %ec_check
    // stack: isValid(x, y), x, y, s, retdest
    %jumpi(ec_mul_valid_point)
    // stack: x, y, s, retdest
    %pop3
    %ec_invalid_input

// Same algorithm as in `exp.asm`
ec_mul_valid_point:
    JUMPDEST
    // stack: x, y, s, retdest
    DUP3
    // stack: s, x, y, s, retdest
    %jumpi(step_case)
    // stack: x, y, s, retdest
    %jump(ret_zero)

step_case:
    JUMPDEST
    // stack: x, y, s, retdest
    PUSH recursion_return
    // stack: recursion_return, x, y, s, retdest
    PUSH 2
    // stack: 2, recursion_return, x, y, s, retdest
    DUP5
    // stack: s, 2, recursion_return, x, y, s, retdest
    DIV
    // stack: s / 2, recursion_return, x, y, s, retdest
    PUSH step_case_contd
    // stack: step_case_contd, s / 2, recursion_return, x, y, s, retdest
    DUP5
    // stack: y, step_case_contd, s / 2, recursion_return, x, y, s, retdest
    DUP5
    // stack: x, y, step_case_contd, s / 2, recursion_return, x, y, s, retdest
    %jump(ec_double)

// Assumption: 2(x,y) = (x',y')
step_case_contd:
    JUMPDEST
    // stack: x', y', s / 2, recursion_return, x, y, s, retdest
    %jump(ec_mul_valid_point)

recursion_return:
    JUMPDEST
    // stack: x', y', x, y, s, retdest
    SWAP4
    // stack: s, y', x, y, x', retdest
    PUSH 1
    // stack: 1, s, y', x, y, x', retdest
    AND
    // stack: s & 1, y', x, y, x', retdest
    SWAP1
    // stack: y', s & 1, x, y, x', retdest
    SWAP2
    // stack: x, s & 1, y', y, x', retdest
    SWAP3
    // stack: y, s & 1, y', x, x', retdest
    SWAP4
    // stack: x', s & 1, y', x, y, retdest
    SWAP1
    // stack: s & 1, x', y', x, y, retdest
    %jumpi(odd_scalar)
    // stack: x', y', x, y, retdest
    SWAP3
    // stack: y, y', x, x', retdest
    POP
    // stack: y', x, x', retdest
    SWAP1
    // stack: x, y', x', retdest
    POP
    // stack: y', x', retdest
    SWAP2
    // stack: retdest, x', y'
    JUMP

odd_scalar:
    JUMPDEST
    // stack: x', y', x, y, retdest
    %jump(ec_add_valid_points)

ret_zero:
    JUMPDEST
    // stack: x, y, s, retdest
    %pop3
    // stack: retdest
    PUSH 0
    // stack: 0, retdest
    PUSH 0
    // stack: 0, 0, retdest
    SWAP2
    // stack: retdest, 0, 0
    JUMP
