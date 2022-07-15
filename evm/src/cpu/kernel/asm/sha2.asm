global count_bits:
    JUMPDEST
    // stack: n (assumed to be > 0)
    push 0
    // stack: 0, n
    swap1
    // stack: n, 0
    %jump(count_bits_loop)
count_bits_loop:
    JUMPDEST
    // stack: k, bits
    %div2
    // stack: k//2, bits
    swap1
    // stack: bits, k//2
    %increment
    // stack: bits+1, k//2
    swap1
    // stack: k//2, bits+1
    %jumpi(count_bits_loop)
    // stack: 0, bits
    pop
    // stack: bits

global sha2_store:
    JUMPDEST
    // stack: num_u256s, x[0], x[1], x[2], ... , x[num_u256s-1]
    dup1
    // stack: num_u256s, num_u256s, x[0], x[1], x[2], ... , x[num_u256s-1]
    // TODO: use kernel memory, and start address not at 0
    push 0
    // stack: addr=0, num_u256s, num_u256s, x[0], x[1], x[2], ... , x[num_u256s-1]
    mstore
    // stack: num_u256s, x[0], x[1], x[2], ... , x[num_u256s-1]
    push 1
    // stack: addr=1, counter=num_u256s, x[0], x[1], x[2], ... , x[num_u256s-1]
sha2_store_loop:
    JUMPDEST
    // stack: addr, counter, x[num_u256s-counter], ... , x[num_u256s-1]
    dup1
    // stack: addr, addr, counter, x[num_u256s-counter], ... , x[num_u256s-1]
    swap3
    // stack: x[num_u256s-counter], addr, counter, addr,  ... , x[num_u256s-1]
    swap1
    // stack: addr, x[num_u256s-counter], counter, addr,  ... , x[num_u256s-1]
    mstore
    // stack: counter, addr,  ... , x[num_u256s-1]
    %decrement
    // stack: counter-1, addr,  ... , x[num_u256s-1]
    iszero
    %jumpi(sha2_store_end)
    swap1
    // stack: addr, counter-1,  ... , x[num_u256s-1]
    %increment
    // stack: addr+1, counter-1,  ... , x[num_u256s-1]
    %jump(sha2_store_loop)
sha2_store_end:
    JUMPDEST

// Precodition: input is in memory, starting at [TODO: fix] 0, of the form
//              num_u256s, x[0], x[1], ..., x[num_u256s-1]
// Postcodition: input is in memory, starting at [TODO: fix] 0, of the form
//               num_blocks, block0[0], block0[1], block1[0], ..., blocklast[1]
global sha2_pad:
    // TODO: use kernel memory, and start address not at 0
    push 0
    mload
    // stack: num_u256s
    dup1
    // stack: num_u256s, num_u256s
    %iseven
    // stack: is_even, num_u256s
    swap1
    // stack: num_u256s, is_even
    dup1
    // stack: num_u256s, num_u256s, is_even
    mload
    // stack: x[num_u256s-1], num_u256s, is_even
    dup1
    // stack: x[num_u256s-1], x[num_u256s-1], num_u256s, is_even
    %count_bits
    // stack: num_bits, x[num_u256s-1], num_u256s, is_even
    dup1
    // stack: num_bits, num_bits, x[num_u256s-1], num_u256s, is_even
    swap3
    // stack: num_u256s, num_bits, x[num_u256s-1], num_bits, is_even
    %decrement
    // stack: num_u256s-1, num_bits, x[num_u256s-1], num_bits, is_even
    push 256
    mul
    // stack: (num_u256s-1)*256, num_bits, x[num_u256s-1], num_bits, is_even
    add
    // stack: message_bits, x[num_u256s-1], num_bits, is_even
    swap2
    // stack: num_bits, x[num_u256s-1], message_bits, is_even
    dup1
    // stack: num_bits, num_bits, x[num_u256s-1], message_bits, is_even
    dup1
    // stack: num_bits, num_bits, num_bits, x[num_u256s-1], message_bits, is_even
    dup1
    %lt(191)
    // stack: num_bits<191, num_bits, num_bits,x[num_u256s-1], message_bits, is_even
    swap1
    // stack: num_bits, num_bits<191, num_bits, x[num_u256s-1], message_bits, is_even
    dup1
    %eq(256)
    // stack: num_bits==256, num_bits<191, num_bits, x[num_u256s-1], message_bits, is_even
    push 0
    // stack: 0, num_bits==256, num_bits<191, num_bits, x[num_u256s-1], message_bits, is_even
    swap6
    // stack: is_even, num_bits==256, num_bits<191, num_bits, x[num_u256s-1], message_bits
    dup2
    dup2
    and
    %jumpi(pad_case1)
    not
    // stack: is_odd, num_bits==256, num_bits<191, num_bits, x[num_u256s-1], message_bits
    dup2
    dup2
    and
    %jumpi(pad_case2)
    swap1
    // stack: num_bits==256, is_odd, num_bits<191, num_bits, x[num_u256s-1], message_bits
    pop
    // stack: is_odd, num_bits<191, num_bits, x[num_u256s-1], message_bits
    not
    // stack: is_even, num_bits<191, num_bits, x[num_u256s-1], message_bits
    dup2
    dup2
    and
    %jumpi(pad_case3)
    not
    // stack: is_odd, num_bits<191, num_bits, x[num_u256s-1], message_bits
    dup2
    dup2
    and
    %jumpi(pad_case4)
    swap1
    // stack: num_bits<191, is_odd, num_bits, x[num_u256s-1], message_bits
    pop
    // stack: is_odd, num_bits, x[num_u256s-1], message_bits
    not
    // stack: is_even, num_bits, x[num_u256s-1], message_bits
    %jumpi(pad_case5)
    %jump(pad_case6)
pad_case1:
    // CASE 1: num_u256s is even; num_bits == 256
    JUMPDEST
    // stack: is_odd, num_bits==256, num_bits<191, num_bits, x[num_u256s-1], message_bits
    %pop5
    // stack: message_bits
    push 0
    mload
    // stack: num_u256s, message_bits
    %increment
    // stack: num_u256s+1, message_bits
    dup1
    // stack: num_u256s+1, num_u256s+1, message_bits
    push 2
    push 255
    %jump(exp)
    // stack: 2^255, num_u256s+1, num_u256s+1, message_bits
    swap
    // stack: num_u256s+1, 2^255, num_u256s+1, message_bits
    mstore
    // stack: num_u256s+1, message_bits
    %increment
    // stack: num_u256s+2, message_bits
    dup1
    // stack: num_u256s+2, num_u256s+2, message_bits
    swap2
    // stack: message_bits, num_u256s+2, num_u256s+2
    swap1
    // stack: num_u256s+2, message_bits, num_u256s+2
    mstore
    // stack: num_u256s+2
    %div2
    // stack: num_blocks=(num_u256s+2)//2
    push 0
    mstore
    %jump(pad_end)
pad_case2:
    // CASE 2: num_u256s is odd; num_bits == 256
    JUMPDEST
    // stack: is_even, num_bits==256, num_bits<191, num_bits, x[num_u256s-1], message_bits
    %pop5
    // stack: message_bits
    push 0
    mload
    // stack: num_u256s, message_bits
    %increment
    // stack: num_u256s+1, message_bits
    swap
    // stack: message_bits, num_u256s+1
    push 2
    push 255
    %jump(exp)
    add
    // stack: 2^255 + message_bits, num_u256s+1
    swap1
    // stack: num_u256s+1, 2^255 + message_bits
    dup1
    // stack: num_u256s+1, num_u256s+1, 2^255 + message_bits
    swap2
    // stack: 2^255 + message_bits, num_u256s+1, num_u256s+1
    swap1
    // stack: num_u256s+1, 2^255 + message_bits, num_u256s+1
    mstore
    // stack: num_u256s+1
    div2
    // stack: num_blocks=(num_u256s+1)//2
    push 0
    mstore
    %jump(pad_end)
pad_case3:
    // CASE 3: num_u256s is even; num_bits < 191
    JUMPDEST
    // stack: is_even, num_bits<191, num_bits, x[num_u256s-1], message_bits
    %pop2
    // stack: num_bits, x[num_u256s-1], message_bits
    swap1
    // stack: x[num_u256s-1], num_bits, message_bits
    push 2
    mul
    %increment
    // stack: 2*x[num_u256s-1]+1, num_bits, message_bits
    swap1
    // stack: num_bits, 2*x[num_u256s-1]+1, message_bits
    push 255
    sub
    // stack: 256 - (num_bits + 1), 2*x[num_u256s-1]+1, message_bits
    push 2
    %jump(exp)
    // stack: 2^(256 - (num_bits + 1)), 2*x[num_u256s-1]+1, message_bits
    mul
    // stack: [x[num_u256s-1] || 1 || 0s], message_bits
    add
    // stack: [x[num_u256s-1] || 1 || 0s | message_bits]
    push 0
    mload
    // stack: num_u256s, [x[num_u256s-1] || 1 || 0s | message_bits]
    mstore
    push 0
    mload
    // stack: num_u256s
    %div2
    // stack: num_blocks=num_u256s//2
    push 0
    mstore
    %jump(pad_end)
pad_case4:
    // CASE 4: num_u256s is odd; num_bits < 191
    JUMPDEST
    // stack: is_odd, num_bits<191, num_bits, x[num_u256s-1], message_bits
    %pop2
    // stack: num_bits, x[num_u256s-1], message_bits
    swap1
    // stack: x[num_u256s-1], num_bits, message_bits
    push 2
    mul
    %increment
    // stack: 2*x[num_u256s-1]+1, num_bits, message_bits
    swap1
    // stack: num_bits, 2*x[num_u256s-1]+1, message_bits
    push 255
    sub
    // stack: 256 - (num_bits + 1), 2*x[num_u256s-1]+1, message_bits
    push 2
    %jump(exp)
    // stack: 2^(256 - (num_bits + 1)), 2*x[num_u256s-1]+1, message_bits
    mul
    // stack: [x[num_u256s-1] || 1 || 0s], message_bits
    push 0
    mload
    // stack: num_u256s, [x[num_u256s-1] || 1 || 0s], message_bits
    mstore
    // stack: message_bits
    push 0
    mload
    // stack: num_u256s, message_bits
    %increment
    // stack: num_u256s+1, message_bits
    mstore
    push 0
    mload
    // stack: num_u256s
    %increment
    // stack: num_u256s+1
    %div2
    // stack: num_blocks=(num_u256s+1)//2
    push 0
    mstore
    %jump(pad_end)
pad_case5:
    // CASE 5: num_u256s is even; 191 <= num_bits < 256
    JUMPDEST
    // stack: is_even, num_bits, x[num_u256s-1], message_bits
    pop
    // stack: num_bits, x[num_u256s-1], message_bits
    swap1
    // stack: x[num_u256s-1], num_bits, message_bits
    push 2
    mul
    %increment
    // stack: 2*x[num_u256s-1]+1, num_bits, message_bits
    swap1
    // stack: num_bits, 2*x[num_u256s-1]+1, message_bits
    push 255
    sub
    // stack: 256 - (num_bits + 1), 2*x[num_u256s-1]+1, message_bits
    push 2
    %jump(exp)
    // stack: 2^(256 - (num_bits + 1)), 2*x[num_u256s-1]+1, message_bits
    mul
    // stack: [x[num_u256s-1] || 1 || 0s], message_bits
    push 0
    mload
    // stack: num_u256s, [x[num_u256s-1] || 1 || 0s], message_bits
    dup1
    // stack: num_u256s, num_u256s, [x[num_u256s-1] || 1 || 0s], message_bits
    swap2
    // stack: [x[num_u256s-1] || 1 || 0s], num_u256s, num_u256s, message_bits
    swap1
    // stack: num_u256s, [x[num_u256s-1] || 1 || 0s], num_u256s, message_bits
    mstore
    // stack: num_u256s, message_bits
    push 2
    add
    // stack: num_u256s+2, message_bits
    dup1
    // stack: num_u256s+2, num_u256s+2, message_bits
    swap2
    // stack: message_bits, num_u256s+2, num_u256s+2
    swap1
    // stack: num_u256s+2, message_bits, num_u256s+2
    mstore
    // stack: num_u256s+2
    div2
    // stack: num_blocks=(num_u256s+2)//2
    push 0
    mstore
    %jump(pad_end)
pad_case6:
    // CASE 6: num_u256s is odd; 191 <= num_bits < 256
    JUMPDEST
    // stack: is_even, num_bits, x[num_u256s-1], message_bits
    pop
    // stack: num_bits, x[num_u256s-1], message_bits
    swap1
    // stack: x[num_u256s-1], num_bits, message_bits
    push 2
    mul
    %increment
    // stack: 2*x[num_u256s-1]+1, num_bits, message_bits
    swap1
    // stack: num_bits, 2*x[num_u256s-1]+1, message_bits
    push 255
    sub
    // stack: 256 - (num_bits + 1), 2*x[num_u256s-1]+1, message_bits
    push 2
    %jump(exp)
    // stack: 2^(256 - (num_bits + 1)), 2*x[num_u256s-1]+1, message_bits
    mul
    // stack: [x[num_u256s-1] || 1 || 0s], message_bits
    push 0
    mload
    // stack: num_u256s, [x[num_u256s-1] || 1 || 0s], message_bits
    dup1
    // stack: num_u256s, num_u256s, [x[num_u256s-1] || 1 || 0s], message_bits
    swap2
    // stack: [x[num_u256s-1] || 1 || 0s], num_u256s, num_u256s, message_bits
    swap1
    // stack: num_u256s, [x[num_u256s-1] || 1 || 0s], num_u256s, message_bits
    mstore
    // stack: num_u256s, message_bits
    %increment
    // stack: num_u256s+1, message_bits
    dup1
    // stack: num_u256s+1, num_u256s+1, message_bits
    swap2
    // stack: message_bits, num_u256s+1, num_u256s+1
    swap1
    // stack: num_u256s+1, message_bits, num_u256s+1
    mstore
    // stack: num_u256s+1
    div2
    // stack: num_blocks=(num_u256s+1)//2
    push 0
    mstore
pad_end:
    JUMPDEST
