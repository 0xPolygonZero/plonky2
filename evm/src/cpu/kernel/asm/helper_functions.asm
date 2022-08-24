global swapn:
    JUMPDEST

    // stack: n, ...
    %eq(1)
    %jumpi(case1)
    %eq(2)
    %jumpi(case2)
    %eq(3)
    %jumpi(case3)
    %eq(4)
    %jumpi(case4)
    %eq(5)
    %jumpi(case5)
    %eq(6)
    %jumpi(case6)
    %eq(7)
    %jumpi(case7)
    %eq(8)
    %jumpi(case8)
    %eq(9)
    %jumpi(case9)
    %eq(10)
    %jumpi(case10)
    %eq(11)
    %jumpi(case11)
    %eq(12)
    %jumpi(case12)
    %eq(13)
    %jumpi(case13)
    %eq(14)
    %jumpi(case14)
    %eq(15)
    %jumpi(case15)
    %eq(16)
    %jumpi(case16)
case1:
    JUMPDEST
    swap1
    %jump(swapn_end)
case2:
    JUMPDEST
    swap2
case3:
    JUMPDEST
    swap3
case4:
    JUMPDEST
    swap4
case5:
    JUMPDEST
    swap5
case6:
    JUMPDEST
    swap6
case7:
    JUMPDEST
    swap7
case8:
    JUMPDEST
    swap8
case9:
    JUMPDEST
    swap9
case10:
    JUMPDEST
    swap10
case11:
    JUMPDEST
    swap11
case12:
    JUMPDEST
    swap12
case13:
    JUMPDEST
    swap13
case14:
    JUMPDEST
    swap14
case15:
    JUMPDEST
    swap15
case16:
    JUMPDEST
    swap16
swapn_end:
    JUMPDEST


global insertn:
    JUMPDEST

    // stack: n, val, ... 
    dup1
    // stack: n, n, val, ...
    swap2
    // stack: val, n, n, ...
    swap1
    // stack: n, val, n, ...
    %jump(swapn)
    // stack: [nth], n, ..., val
    swap1
    // stack: n, [nth], ..., val
swap_back_loop:
    // stack: k, [kth], ..., [k-1st]
    dup1
    // stack: k, k, [kth], ..., [k-1st]
    swap2
    // stack: [kth], k, k, ..., [k-1st]
    swap1
    // stack: k, [kth], k, ..., [k-1st]
    %jump(swapn)
    // stack: [k-1st], k, ..., [k-2nd], [kth]
    swap1
    // stack: k, [k-1st], ..., [k-2nd], [kth]
    %decrement
    // stack: k-1, [k-1st], ..., [k-2nd], [kth]
    iszero
    not
    %jumpi(swap_back_loop)
