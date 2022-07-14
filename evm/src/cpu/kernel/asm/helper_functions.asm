global swapn
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
    swap1
case2:
    swap2
case3:
    swap3
case4:
    swap4
case5:
    swap5
case6:
    swap6
case7:
    swap7
case8:
    swap8
case9:
    swap9
case10:
    swap10
case11:
    swap11
case12:
    swap12
case13:
    swap13
case14:
    swap14
case15:
    swap15
case16:
    swap16
swapn_end:


global insertn:
    // stack: n, val, ...
    dup
    // stack: n, n, val, ...
    swap2
    // stack: val, n, n, ...
    swap1
    // stack: n, val, n, ...
    %swapn
    // stack: [nth], n, ..., val
    swap1
    // stack: n, [nth], ..., val
swap_back_loop:
    // stack: k, k, [kth], ..., [k-1st]
    dup
    // stack: k, k, [kth], ..., [k-1st]
    swap2
    // stack: [kth], k, k, ..., [k-1st]
    swap1
    // stack: k, [kth], k, ..., [k-1st]
    %swapn
    // stack: [k-1st], k, ..., [k-2nd], [kth]
    swap1
    // stack: k, [k-1st], ..., [k-2nd], [kth]
    %decrement
    // stack: k-1, [k-1st], ..., [k-2nd], [kth]
    iszero
    not
    %jumpi(swap_back_loop)
