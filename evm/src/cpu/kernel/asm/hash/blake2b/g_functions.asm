blake2b_g_function:
    // Function to mix two input words, x and y, into the four words indexed by a, b, c, d (which
    // are in the range 0..16) in the internal state.
    // The internal state is stored in memory starting at the address start.
    // stack: a, b, c, d, x, y, start, retdest
    %stack (indices: 4) -> (indices, indices)
    // stack: a, b, c, d, a, b, c, d, x, y, start, retdest
    DUP11
    // stack: start, a, b, c, d, a, b, c, d, x, y, start, retdest
    %stack (start, a, b, c, d) -> (d, start, c, start, b, start, a, start)
    // stack: d, start, c, start, b, start, a, start, a, b, c, d, x, y, start, retdest
    ADD
    %mload_kernel_general
    // stack: v[d], c, start, b, start, a, start, a, b, c, d, x, y, start, retdest
    %stack (vd, remaining: 6) -> (remaining, vd)
    // stack: c, start, b, start, a, start, v[d], a, b, c, d, x, y, start, retdest
    ADD
    %mload_kernel_general
    %stack (vc, remaining: 4) -> (remaining, vc)
    // stack: b, start, a, start, v[c], v[d], a, b, c, d, x, y, start, retdest
    ADD
    %mload_kernel_general
    // stack: v[b], a, start, v[c], v[d], a, b, c, d, x, y, start, retdest
    %stack (vb, remaining: 2) -> (remaining, vb)
    // stack: a, start, v[b], v[c], v[d], a, b, c, d, x, y, start, retdest
    ADD
    %mload_kernel_general
    // stack: v[a], v[b], v[c], v[d], a, b, c, d, x, y, start, retdest
    DUP2
    // stack: v[b], v[a], v[b], v[c], v[d], a, b, c, d, x, y, start, retdest
    DUP10
    // stack: x, v[b], v[a], v[b], v[c], v[d], a, b, c, d, x, y, start, retdest
    ADD
    ADD
    %as_u64
    // stack: v[a]' = (v[a] + v[b] + x) % 2^64, v[b], v[c], v[d], a, b, c, d, x, y, start, retdest
    %stack (a, b, c, d) -> (a, d, a, b, c, d)
    // stack: v[a]', v[d], v[a]', v[b], v[c], v[d], a, b, c, d, x, y, start, retdest
    XOR
    %rotr_64(32)
    // stack: v[d]' = (v[d] ^ v[a]') >>> 32, v[a]', v[b], v[c], v[d], a, b, c, d, x, y, start, retdest
    %stack (top: 4, vd) -> (top)
    // stack: v[d]', v[a]', v[b], v[c], a, b, c, d, x, y, start, retdest
    %stack (d, a, b, c) -> (c, d, a, b, d)
    // stack: v[c], v[d]', v[a]', v[b], v[d]', a, b, c, d, x, y, start, retdest
    ADD
    %as_u64
    // stack: v[c]' = (v[c] + v[d]') % 2^64, v[a]', v[b], v[d]', a, b, c, d, x, y, start, retdest
    %stack (c, a, b, d) -> (b, c, a, c, d)
    // stack: v[b], v[c]', v[a]', v[c]', v[d]', a, b, c, d, x, y, start, retdest
    XOR
    %rotr_64(24)
    // stack: v[b]' = (v[b] ^ v[c]') >>> 24, v[a]', v[c]', v[d]', a, b, c, d, x, y, start, retdest
    SWAP1
    // stack: v[a]', v[b]', v[c]', v[d]', a, b, c, d, x, y, start, retdest
    DUP2
    // stack: v[b]', v[a]', v[b]', v[c]', v[d]', a, b, c, d, x, y, start, retdest
    DUP11
    // stack: y, v[b]', v[a]', v[b]', v[c]', v[d]', a, b, c, d, x, y, start, retdest
    ADD
    ADD
    %as_u64
    // stack: v[a]'' = (v[a]' + v[b]' + y) % 2^64, v[b]', v[c]', v[d]', a, b, c, d, x, y, start, retdest
    SWAP3
    // stack: v[d]', v[b]', v[c]', v[a]'', a, b, c, d, x, y, start, retdest
    DUP4
    // stack: v[a]'', v[d]', v[b]', v[c]', v[a]'', a, b, c, d, x, y, start, retdest
    XOR
    %rotr_64(16)
    // stack: v[d]'' = (v[a]'' ^ v[d]') >>> 8, v[b]', v[c]', v[a]'', a, b, c, d, x, y, start, retdest
    SWAP2
    // stack: v[c]', v[b]', v[d]'', v[a]'', a, b, c, d, x, y, start, retdest
    DUP3
    // stack: v[d]'', v[c]', v[b]', v[d]'', v[a]'', a, b, c, d, x, y, start, retdest
    ADD
    %as_u64
    // stack: v[c]'' = (v[c]' + v[d]'') % 2^64, v[b]', v[d]'', v[a]'', a, b, c, d, x, y, start, retdest
    DUP1
    // stack: v[c]'', v[c]'', v[b]', v[d]'', v[a]'', a, b, c, d, x, y, start, retdest
    SWAP2
    // stack: v[b]', v[c]'', v[c]'', v[d]'', v[a]'', a, b, c, d, x, y, start, retdest
    XOR
    %rotr_64(63)
    // stack: v[b]'' = (v[b]' ^ v[c]'') >>> 7, v[c]'', v[d]'', v[a]'', a, b, c, d, x, y, start, retdest
    %stack (vb, vc, vd, va, a, b, c, d, x, y, start) -> (start, a, va, start, b, vb, start, c, vc, start, d, vd)
    // stack: start, a, v[a]'', start, b, v[b]'', start, c, v[c]'', start, d, v[d]'', retdest
    ADD
    %mstore_kernel_general
    ADD
    %mstore_kernel_general
    ADD
    %mstore_kernel_general
    ADD
    %mstore_kernel_general
    // stack: retdest
    JUMP

call_blake2b_g_function:
    // stack: a, b, c, d, x_idx, y_idx, round, start, retdest
    DUP6
    // stack: y_idx, a, b, c, d, x_idx, y_idx, round, start, retdest
    DUP8
    // stack: round, y_idx, a, b, c, d, x_idx, y_idx, round, start, retdest
    %blake2b_permutation
    // stack: s[y_idx], a, b, c, d, x_idx, y_idx, round, start, retdest
    %blake2b_message_addr
    ADD
    %mload_kernel_general
    // stack: m[s[y_idx]], a, b, c, d, x_idx, y_idx, round, start, retdest
    DUP6
    // stack: x_idx, m[s[y_idx]], a, b, c, d, x_idx, y_idx, round, start, retdest
    DUP9
    // stack: round, x_idx, m[s[y_idx]], a, b, c, d, x_idx, y_idx, round, start, retdest
    %blake2b_permutation
    // stack: s[x_idx], m[s[y_idx]], a, b, c, d, x_idx, y_idx, round, start, retdest
    %blake2b_message_addr
    ADD
    %mload_kernel_general
    // stack: m[s[x_idx]], m[s[y_idx]], a, b, c, d, x_idx, y_idx, round, start, retdest
    %stack (mm: 2, abcd: 4, xy: 2, r, s) -> (abcd, mm, s, r, s)
    // stack: a, b, c, d, m[s[x_idx]], m[s[y_idx]], start, round, start, retdest
    %jump(blake2b_g_function)

global run_g_function_round:
    // stack: round, start, retdest
    PUSH g_function_return_1
    // stack: g_function_return_1, round, start, retdest
    %stack (ret, r, s) -> (0, 4, 8, 12, 0, 1, r, s, ret, r, s)
    // stack: a=0, b=4, c=8, d=12, x_idx=0, y_idx=1, round, start, g_function_return_1, round, start, retdest
    %jump(call_blake2b_g_function)
g_function_return_1:
    // stack: round, start, retdest
    PUSH g_function_return_2
    // stack: g_function_return_2, round, start, retdest
    %stack (ret, r, s) -> (1, 5, 9, 13, 2, 3, r, s, ret, r, s)
    // stack: a=1, b=5, c=9, d=13, x_idx=2, y_idx=3, round, start, g_function_return_2, round, start, retdest
    %jump(call_blake2b_g_function)
g_function_return_2:
    // stack: round, start, retdest
    PUSH g_function_return_3
    // stack: g_function_return_3, round, start, retdest
    %stack (ret, r, s) -> (2, 6, 10, 14, 4, 5, r, s, ret, r, s)
    // stack: a=2, b=6, c=10, d=14, x_idx=4, y_idx=5, round, start, g_function_return_3, round, start, retdest
    %jump(call_blake2b_g_function)
g_function_return_3:
    // stack: round, start, retdest
    PUSH g_function_return_4
    // stack: g_function_return_4, round, start, retdest
    %stack (ret, r, s) -> (3, 7, 11, 15, 6, 7, r, s, ret, r, s)
    // stack: a=3, b=7, c=11, d=15, x_idx=6, y_idx=7, round, start, g_function_return_4, round, start, retdest
    %jump(call_blake2b_g_function)
g_function_return_4:
    // stack: round, start, retdest
    PUSH g_function_return_5
    // stack: g_function_return_5, round, start, retdest
    %stack (ret, r, s) -> (0, 5, 10, 15, 8, 9, r, s, ret, r, s)
    // stack: a=0, b=5, c=10, d=15, x_idx=8, y_idx=9, round, start, g_function_return_5, round, start, retdest
    %jump(call_blake2b_g_function)
g_function_return_5:
    // stack: round, start, retdest
    PUSH g_function_return_6
    // stack: g_function_return_6, round, start, retdest
    %stack (ret, r, s) -> (1, 6, 11, 12, 10, 11, r, s, ret, r, s)
    // stack: a=1, b=6, c=11, d=12, x_idx=10, y_idx=11, round, start, g_function_return_6, round, start, retdest
    %jump(call_blake2b_g_function)
g_function_return_6:
    // stack: round, start, retdest
    PUSH g_function_return_7
    // stack: g_function_return_7, round, start, retdest
    %stack (ret, r, s) -> (2, 7, 8, 13, 12, 13, r, s, ret, r, s)
    // stack: a=2, b=7, c=8, d=13, x_idx=12, y_idx=13, round, start, g_function_return_7, round, start, retdest
    %jump(call_blake2b_g_function)
g_function_return_7:
    // stack: round, start, retdest
    PUSH g_function_return_8
    // stack: g_function_return_8, round, start, retdest
    %stack (ret, r, s) -> (3, 4, 9, 14, 14, 15, r, s, ret, r, s)
    // stack: a=3, b=4, c=9, d=14, x_idx=14, y_idx=15, round, start, g_function_return_8, round, start, retdest
    %jump(call_blake2b_g_function)
g_function_return_8:
    // stack: round, start, retdest
    SWAP1
    // stack: start, round, retdest
    SWAP2
    // stack: retdest, round, start
    JUMP


global run_12_rounds_g_function:
    // stack: start, retdest
    PUSH 0
    // stack: round=0, start, retdest
run_next_round_g_function:
    // stack: round, start, retdest
    PUSH run_g_function_round_return
    // stack: run_g_function_round_return, round, start, retdest
    SWAP2
    // stack: start, round, run_g_function_round_return, retdest
    SWAP1
    // stack: round, start, run_g_function_round_return, retdest
    %jump(run_g_function_round)
run_next_round_g_function_return:
    // stack: round, start, retdest
    %increment
    // stack: round+1, start, retdest
    DUP1
    // stack: round+1, round+1, start, retdest
    %lt_const(12)
    // stack: round+1 < 12, round+1, start, retdest
    %jumpi(run_next_round_g_function)
    // stack: round+1, start, retdest
    %pop2
    // stack: retdest
    JUMP

