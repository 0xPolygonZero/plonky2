global Rol:
    jumpdest
    // stack: n, x, retdest
    swap1
    // stack: x, n, retdest
    dup1
    // stack: x, x, n, retdest
    dup3
    // stack: n, x, x, n, retdest
    push 32
    // stack: 32, n, x, x, n, retdest
    sub
    // stack: 32-n, x, x, n, retdest
    swap1
    // stack: x, 32-n, x, n, retdest
    shr
    // stack: x << (32-n), x, n, retdest
    swap2
    // stack: n, x, x << (32-n), retdest
    swap1
    // stack: x, n, x << (32-n), retdest
    shl
    // stack: x >> n, x << (32-n), retdest
    push 0xffffffff
    // stack: 0xffffffff, (x >> n), x << (32-n), retdest
    and
    // stack: (x >> n) & 0xffffffff, x << (32-n), retdest
    or
    // stack: ((x >> n) & 0xffffffff) | (x << (32-n)), retdest
    swap1
    // stack: retdest, ((x >> n) & 0xffffffff) | (x << (32-n))
    jump


global F0:
    jumpdest
    // stack: x, y, z, retdest
    xor
    // stack: x ^ y, z, retdest
    xor
    // stack: x ^ y ^ z, retdest
    swap1
    // stack: retdest, x ^ y ^ z
    jump


global F1:
    jumpdest
    // stack: x, y, z, retdest
    dup1
    // stack: x, x, y, z, retdest
    swap2
    // stack: y, x, x, z, retdest
    and
    // stack: y & x, x, z, retdest
    swap2
    // stack: z, x, y & x, retdest
    swap1
    // stack: x, z, y & x, retdest
    %not_u32
    // stack: ~x, z, y & x, retdest
    and
    // stack: ~x & z, y & x, retdest
    or
    // stack: (~x & z) | (y & x), retdest
    swap1
    // stack: retdest, (~x & z) | (y & x)
    jump


global F2:
    jumpdest
    // stack: x, y, z, retdest
    swap1
    // stack: y, x, z, retdest
    %not_u32
    // stack: ~y, x, z, retdest
    or
    // stack: ~y | x, z, retdest
    xor
     // stack: (~y | x) ^ z, retdest
    swap1
    // stack: retdest, (~y | x) ^ z
    jump


global F3:
    jumpdest
    // stack: x, y, z, retdest
    dup3
    // stack: z, x, y, z, retdest
    and
    // stack: z & x, y, z, retdest
    swap2
    // stack: z, y, z & x, retdest
    %not_u32
    // stack: ~z, y, z & x, retdest
    and
    // stack: ~z & y, z & x, retdest
    or
    // stack: (~z & y) | (z & x), retdest
    swap1
    // stack: retdest, (~z & y) | (z & x)
    jump 


global F4:
    jumpdest 
    // stack: x, y, z, retdest
    swap2
    // stack: z, y, x, retdest
    %not_u32
    // stack: ~z, y, x, retdest
    or
    // stack: ~z | y, x, retdest
    xor
    // stack: (~z | y) ^ x, retdest
    swap1
    // stack: retdest, (~z | y) ^ x
    jump
