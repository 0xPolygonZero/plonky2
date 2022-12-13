%macro blake2b_g_function
    // Function to mix two input words, x and y, into the four words indexed by a, b, c, d (which
    // are in the range 0..16) in the internal state.
    // The internal state is stored in memory starting at the address start.
    // stack: a, b, c, d, x, y, start
    %stack (indices: 4) -> (indices, indices)
    // stack: a, b, c, d, a, b, c, d, x, y, start
    DUP11
    // stack: start, a, b, c, d, a, b, c, d, x, y, start
    %stack (start, a, b, c, d) -> (d, start, c, start, b, start, a, start)
    // stack: d, start, c, start, b, start, a, start, a, b, c, d, x, y, start
    ADD
    %mload_kernel_general
    // stack: v[d], c, start, b, start, a, start, a, b, c, d, x, y, start
    %stack (vd, remaining: 6) -> (remaining, vd)
    // stack: c, start, b, start, a, start, v[d], a, b, c, d, x, y, start
    ADD
    %mload_kernel_general
    %stack (vc, remaining: 4) -> (remaining, vc)
    // stack: b, start, a, start, v[c], v[d], a, b, c, d, x, y, start
    ADD
    %mload_kernel_general
    // stack: v[b], a, start, v[c], v[d], a, b, c, d, x, y, start
    %stack (vb, remaining: 2) -> (remaining, vb)
    // stack: a, start, v[b], v[c], v[d], a, b, c, d, x, y, start
    ADD
    %mload_kernel_general
    // stack: v[a], v[b], v[c], v[d], a, b, c, d, x, y, start
    DUP2
    // stack: v[b], v[a], v[b], v[c], v[d], a, b, c, d, x, y, start
    DUP10
    // stack: x, v[b], v[a], v[b], v[c], v[d], a, b, c, d, x, y, start
    ADD
    ADD
    %as_u64
    // stack: v[a]' = (v[a] + v[b] + x) % 2^64, v[b], v[c], v[d], a, b, c, d, x, y, start
    %stack (a, b, c, d) -> (a, d, a, b, c, d)
    // stack: v[a]', v[d], v[a]', v[b], v[c], v[d], a, b, c, d, x, y, start
    XOR
    %rotr_64(32)
    // stack: v[d]' = (v[d] ^ v[a]') >>> 32, v[a]', v[b], v[c], v[d], a, b, c, d, x, y, start
    %stack (top: 4, vd) -> (top)
    // stack: v[d]', v[a]', v[b], v[c], a, b, c, d, x, y, start
    %stack (d, a, b, c) -> (c, d, a, b, d)
    // stack: v[c], v[d]', v[a]', v[b], v[d]', a, b, c, d, x, y, start
    ADD
    %as_u64
    // stack: v[c]' = (v[c] + v[d]') % 2^64, v[a]', v[b], v[d]', a, b, c, d, x, y, start
    %stack (c, a, b, d) -> (b, c, a, c, d)
    // stack: v[b], v[c]', v[a]', v[c]', v[d]', a, b, c, d, x, y, start
    XOR
    %rotr_64(24)
    // stack: v[b]' = (v[b] ^ v[c]') >>> 24, v[a]', v[c]', v[d]', a, b, c, d, x, y, start
    SWAP1
    // stack: v[a]', v[b]', v[c]', v[d]', a, b, c, d, x, y, start
    DUP2
    // stack: v[b]', v[a]', v[b]', v[c]', v[d]', a, b, c, d, x, y, start
    DUP11
    // stack: y, v[b]', v[a]', v[b]', v[c]', v[d]', a, b, c, d, x, y, start
    ADD
    ADD
    %as_u64
    // stack: v[a]'' = (v[a]' + v[b]' + y) % 2^64, v[b]', v[c]', v[d]', a, b, c, d, x, y, start
    SWAP3
    // stack: v[d]', v[b]', v[c]', v[a]'', a, b, c, d, x, y, start
    DUP4
    // stack: v[a]'', v[d]', v[b]', v[c]', v[a]'', a, b, c, d, x, y, start
    XOR
    %rotr_64(16)
    // stack: v[d]'' = (v[a]'' ^ v[d]') >>> 8, v[b]', v[c]', v[a]'', a, b, c, d, x, y, start
    SWAP2
    // stack: v[c]', v[b]', v[d]'', v[a]'', a, b, c, d, x, y, start
    DUP3
    // stack: v[d]'', v[c]', v[b]', v[d]'', v[a]'', a, b, c, d, x, y, start
    ADD
    %as_u64
    // stack: v[c]'' = (v[c]' + v[d]'') % 2^64, v[b]', v[d]'', v[a]'', a, b, c, d, x, y, start
    DUP1
    // stack: v[c]'', v[c]'', v[b]', v[d]'', v[a]'', a, b, c, d, x, y, start
    SWAP2
    // stack: v[b]', v[c]'', v[c]'', v[d]'', v[a]'', a, b, c, d, x, y, start
    XOR
    %rotr_64(63)
    // stack: v[b]'' = (v[b]' ^ v[c]'') >>> 7, v[c]'', v[d]'', v[a]'', a, b, c, d, x, y, start
    %stack (vb, vc, vd, va, a, b, c, d, x, y, start) -> (start, a, va, start, b, vb, start, c, vc, start, d, vd)
    // stack: start, a, v[a]'', start, b, v[b]'', start, c, v[c]'', start, d, v[d]''
    ADD
    %mstore_kernel_general
    ADD
    %mstore_kernel_general
    ADD
    %mstore_kernel_general
    ADD
    %mstore_kernel_general
%endmacro

%macro call_blake2b_g_function(a, b, c, d, x_idx, y_idx)
    // stack: round, start
    PUSH $y_idx
    DUP2
    // stack: round, y_idx, round, start
    %blake2b_permutation
    // stack: s[y_idx], round, start
    %blake2b_message_addr
    ADD
    %mload_kernel_general
    // stack: m[s[y_idx]], round, start
    PUSH $x_idx
    DUP3
    // stack: round, 2, m[s[y_idx]], round, start
    %blake2b_permutation
    // stack: s[x_idx], m[s[y_idx]], round, start
    %blake2b_message_addr
    ADD
    %mload_kernel_general
    // stack: m[s[x_idx]], m[s[y_idx]], round, start
    %stack (ss: 2, r, s) -> (ss, s, r, s)
    // stack: m[s[x_idx]], m[s[y_idx]], start, round, start
    PUSH $d
    PUSH $c
    PUSH $b
    PUSH $a
    // stack: a, b, c, d, m[s[x_idx]], m[s[y_idx]], start, round, start
    %blake2b_g_function
    // stack: round, start
%endmacro
