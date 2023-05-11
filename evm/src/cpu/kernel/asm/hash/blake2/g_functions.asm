%macro blake2_g_function
    // Function to mix two input words, x and y, into the four words indexed by a, b, c, d (which
    // are in the range 0..16) in the internal state.
    // The internal state is stored in memory starting at the address start.
    // stack: a, b, c, d, x, y, start
    DUP4
    DUP4
    DUP4
    DUP4
    // stack: a, b, c, d, a, b, c, d, x, y, start
    DUP11
    // stack: start, a, b, c, d, a, b, c, d, x, y, start
    ADD
    %mload_current_general
    // stack: v[a], b, c, d, a, b, c, d, x, y, start
    SWAP1
    // stack: b, v[a], c, d, a, b, c, d, x, y, start
    DUP11
    // stack: start, b, v[a], c, d, a, b, c, d, x, y, start
    ADD
    %mload_current_general
    // stack: v[b], v[a], c, d, a, b, c, d, x, y, start
    SWAP2
    // stack: c, v[a], v[b], d, a, b, c, d, x, y, start
    DUP11
    // stack: start, c, v[a], v[b], d, a, b, c, d, x, y, start
    ADD
    %mload_current_general
    // stack: v[c], v[a], v[b], d, a, b, c, d, x, y, start
    SWAP3
    // stack: d, v[a], v[b], v[c], a, b, c, d, x, y, start
    DUP11
    // stack: start, d, v[a], v[b], v[c], a, b, c, d, x, y, start
    ADD
    %mload_current_general
    // stack: v[d], v[a], v[b], v[c], a, b, c, d, x, y, start
    %stack (vd, vs: 3) -> (vs, vd)
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
    %mstore_current_general
    ADD
    %mstore_current_general
    ADD
    %mstore_current_general
    ADD
    %mstore_current_general
%endmacro

%macro call_blake2_g_function(a, b, c, d, x_idx, y_idx)
    // stack: round, start
    PUSH $y_idx
    DUP2
    // stack: round, y_idx, round, start
    %blake2_permutation
    // stack: s[y_idx], round, start
    %blake2_message_addr
    ADD
    %mload_current_general
    // stack: m[s[y_idx]], round, start
    PUSH $x_idx
    DUP3
    // stack: round, 2, m[s[y_idx]], round, start
    %blake2_permutation
    // stack: s[x_idx], m[s[y_idx]], round, start
    %blake2_message_addr
    ADD
    %mload_current_general
    // stack: m[s[x_idx]], m[s[y_idx]], round, start
    %stack (ss: 2, r, s) -> (ss, s, r, s)
    // stack: m[s[x_idx]], m[s[y_idx]], start, round, start
    PUSH $d
    PUSH $c
    PUSH $b
    PUSH $a
    // stack: a, b, c, d, m[s[x_idx]], m[s[y_idx]], start, round, start
    %blake2_g_function
    // stack: round, start
%endmacro

run_g_function_round:
    // stack: round, start, retdest
    %call_blake2_g_function(0, 4, 8, 12, 0, 1)
    %call_blake2_g_function(1, 5, 9, 13, 2, 3)
    %call_blake2_g_function(2, 6, 10, 14, 4, 5)
    %call_blake2_g_function(3, 7, 11, 15, 6, 7)
    %call_blake2_g_function(0, 5, 10, 15, 8, 9)
    %call_blake2_g_function(1, 6, 11, 12, 10, 11)
    %call_blake2_g_function(2, 7, 8, 13, 12, 13)
    %call_blake2_g_function(3, 4, 9, 14, 14, 15)
    %stack (r, s, ret) -> (ret, r, s)
    // stack: retdest, round, start
    JUMP

global run_rounds_g_function:
    // stack: current_round, start, rounds, retdest
    DUP3
    // stack: rounds, current_round, start, rounds, retdest
    DUP2
    // stack: current_round, rounds, current_round, start, rounds, retdest
    EQ
    %jumpi(run_rounds_g_function_end)
    // stack: current_round, start, rounds, retdest
    PUSH run_rounds_g_function_return
    // stack: run_rounds_g_function_return, current_round, start, rounds, retdest
    %stack (ret, r, s) -> (r, s, ret)
    // stack: current_round, start, run_rounds_g_function_return, rounds, retdest
    %jump(run_g_function_round)
run_rounds_g_function_return:
    // stack: round, start, rounds, retdest
    %increment
    // stack: round + 1, start, rounds, retdest
    %jump(run_rounds_g_function)
run_rounds_g_function_end:
    // stack: current_round, start, rounds, retdest
    %pop3
    // stack: retdest
    JUMP
