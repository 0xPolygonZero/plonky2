global ec_mul:
    JUMPDEST
    // stack: x, y, s, retdest
    DUP2
    // stack: y, x, y, s, retdest
    DUP2
    // stack: x, y, x, y, s, retdest
    %ec_check
    // stack: isValid(x, y), x, y, s, retdest
    PUSH ec_mul_valid_point
    // stack: ec_mul_valid_point, isValid(x, y), x, y, s, retdest
    JUMPI
    // stack: x, y, s, retdest
    POP
    // stack: y, s, retdest
    POP
    // stack: s, retdest
    POP
    // stack: retdest
    JUMP

// Same algorithm as `exp`
ec_mul_valid_point:
    JUMPDEST
    // stack: x, y, s, retdest
    DUP3
    // stack: s, x, y, s, retdest
    PUSH step_case
    // stack: step_case, s, x, y, s, retdest
    JUMPI
    // stack: x, y, s, retdest
    POP
    // stack: y, s, retdest
    POP
    // stack: s, retdest
    POP
    // stack: retdest
    PUSH 0
    // stack: 0, retdest
    PUSH 0
    // stack: 0, 0, retdest
    SWAP2
    // stack: retdest, 0, 0
    JUMP

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
    PUSH ec_double
    // stack: ec_double, x, y, step_case_contd, s / 2, recursion_return, x, y, s, retdest
    JUMP

// Assumption: 2(x,y) = (x',y')
step_case_contd:
    JUMPDEST
    // stack: x', y', s / 2, recursion_return, x, y, s, retdest
    PUSH ec_mul_valid_point
    // stack: ec_mul_valid_point, x', y', s / 2, recursion_return, x, y, s, retdest
    JUMP

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
    PUSH odd_scalar
    // stack: odd_scalar, s & 1, x', y', x, y, retdest
    JUMPI
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
    PUSH ec_add_valid_points
    // stack: ec_add_valid_points, x', y', x, y, retdest
    JUMP
