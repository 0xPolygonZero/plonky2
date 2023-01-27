
global iszero_bignum:
    // stack: len, start_loc, retdest
    DUP2
    // stack: start_loc, len, start_loc, retdest
    ADD
    // stack: end_loc, start_loc, retdest
    SWAP1
    // stack: cur_loc=start_loc, end_loc, retdest
iszero_loop:
    // stack: cur_loc, end_loc, retdest
    DUP1
    // stack: cur_loc, cur_loc, end_loc, retdest
    %mload_kernel_general
    // stack: cur_val, cur_loc, end_loc, retdest
    %jumpi(neqzero)
    // stack: cur_loc, end_loc, retdest
    %increment
    // stack: cur_loc + 1, end_loc, retdest
    %stack (vals: 2) -> (vals, vals)
    // stack: cur_loc + 1, end_loc, cur_loc + 1, end_loc, retdest
    EQ
    %jumpi(eqzero)
    %jump(iszero_loop)
neqzero:
    // stack: cur_loc, end_loc, retdest
    %stack (vals: 2, retdest) -> (retdest, 0)
    // stack: retdest, 0
    JUMP
eqzero:
    // stack: cur_loc, end_loc, retdest
    %stack (vals: 2, retdest) -> (retdest, 1)
    // stack: retdest, 1
    JUMP
