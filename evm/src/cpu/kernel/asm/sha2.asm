global count_bits:
    // stack: n (assumed to be > 0)
    push 0
    // stack: 0, n
    swap1
    // stack: n, 0
count_bits_loop:
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



global sha2_append1:
    // TODO: use kernel memory, and start address not at 0
    push 0
    mload
    // stack: num_u256s
    mload
    // stack: x[num_u256s-1]
    dup1
    // stack: x[num_u256s-1], x[num_u256s-1]
    %count_bits
    // stack: num_bits, x[num_u256s-1]
    %eq(256)
    %jumpi(pad_if256)
    %jump(pad_else)
append_if256:
    JUMPDEST
    // stack: num_bits, x[num_u256s-1]
    %pop2
    push 0
    mload
    // stack: num_u256s
    %increment
    // stack: num_u256s+1
    dup1
    // stack: num_u256s+1, num_u256s+1
    push 0
    mstore
    // stack: num_u256s+1
    push 1
    // stack: 1, num_u256s+1
    swap1
    // stack: num_u256s+1, 1
    mstore
    %jump(pad_end)
append_else:
    JUMPDEST
    // stack: num_bits, x[num_u256s-1]
    pop
    // stack: x[num_u256s-1]
    push 2
    mul
    // stack: 2*x[num_u256s-1]
    %increment
    // stack: 2*x[num_u256s-1]+1
    push 0
    mload
    // stack: num_u256s, 2*x[num_u256s-1]+1
    mstore
append_end:
    JUMPDEST

