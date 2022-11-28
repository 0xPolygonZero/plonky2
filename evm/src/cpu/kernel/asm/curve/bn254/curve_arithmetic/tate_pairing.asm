/// def tate(P : [Fp; 2], Q: [Fp2; 2]) -> Fp12:
///     out = miller_loop(P, Q)
///
///     inv = inverse_fp12(out)
///     out = frob_fp12_6(out)
///     out = mul_fp12(out, inv)
///
///     acc = frob_fp12_2(out)
///     out = mul_fp12(out, acc)
///
///     pow = fast_exp(out)
///     out = frob_fp12_3(out) 
///     out = mul_fp12(out, pow)
///
///     return out

global tate:
    // stack:                     ptr, out,                                             retdest
    PUSH tate_mul3   SWAP2 
    // stack:                     out, ptr,                                  tate_mul3, retdest
    PUSH tate_mul2   SWAP2 
    // stack:                     ptr, out,                       tate_mul2, tate_mul3, retdest
    PUSH tate_mul1   SWAP2
    // stack:                     out, ptr,            tate_mul1, tate_mul2, tate_mul3, retdest
    PUSH post_mllr   SWAP2 
    // stack:                     ptr, out, post_mllr, tate_mul1, tate_mul2, tate_mul3, retdest
    %jump(miller_init)
post_mllr:
    // stack:                          out,            tate_mul1, tate_mul2, tate_mul3, retdest
    PUSH 100 
    // stack:                     100, out,            tate_mul1, tate_mul2, tate_mul3, retdest
    DUP2
    // stack:                out, 100, out,            tate_mul1, tate_mul2, tate_mul3, retdest
    %inverse_fp12
    // stack:                     100, out,            tate_mul1, tate_mul2, tate_mul3, retdest  {100: inv}
    DUP2
    // stack:                out, 100, out,            tate_mul1, tate_mul2, tate_mul3, retdest  {100: inv}
    %frob_fp12_6
    // stack:                out, 100, out,            tate_mul1, tate_mul2, tate_mul3, retdest  {100: inv}
    %jump(mul_fp12)
tate_mul1:
    // stack:                          out,                       tate_mul2, tate_mul3, retdest  {100: inv}
    DUP1
    // stack:                     out, out,                       tate_mul2, tate_mul3, retdest  {100: inv}
    PUSH 100
    // stack:                100, out, out,                       tate_mul2, tate_mul3, retdest  {100: inv}       
    DUP2
    // stack:           out, 100, out, out,                       tate_mul2, tate_mul3, retdest  {100: inv}
    %frob_fp12_2
    // stack:                100, out, out,                       tate_mul2, tate_mul3, retdest  {100: inv}
    %jump(mul_fp12)
tate_mul2: 
    // stack:                          out,                                  tate_mul3, retdest  {100: acc}
    PUSH post_pow
    // stack:                post_pow, out,                                  tate_mul3, retdest  {100: acc}
    PUSH 100
    // stack:           100, post_pow, out,                                  tate_mul3, retdest  {100: acc}
    DUP3
    // stack:      out, 100, post_pow, out,                                  tate_mul3, retdest  {100: acc}
    %jump(power)
post_pow: 
    // stack:                     100, out,                                  tate_mul3, retdest  {100: pow}
    DUP2
    // stack:                out, 100, out,                                  tate_mul3, retdest  {100: pow}
    %frob_fp12_3
    // stack:                out, 100, out,                                  tate_mul3, retdest  {100: pow}
    %jump(mul_fp12)
tate_mul3:
    // stack:                          out,                                             retdest  {100: pow}
    SWAP1  JUMP


/// def miller(P, Q):
///     miller_init()
///     miller_loop()
///
/// def miller_init():
///     out = 1
///     O = P
///     times = 62
///
/// def miller_loop():
///     while times:
///         n_m = fetch_times()
///         while n_m > 10:
///             mul_tangent()
///             mul_cord()
///             n_m - 10
///         while n_n:
///             mul_tangent()
///             n_m - 1
///         times -= 1
             
/// Note: miller_data is formed by
/// (1) taking the binary expansion of the BN254 prime p
/// (2) popping the head and appending a 0:
///     exp = bin(p)[1:-1] + [0]
/// (3) counting the lengths of 1s and 0s in exp, e.g.
///     exp = 1100010011110 => EXP = [(2,3), (1,2), (4,1)]
/// (4) encoding each pair (n,m) as 10*n+m:
///     miller_data = [10*n + m for (n,m) in EXP]

miller_init:
    // stack:         ptr, out, retdest
    PUSH 1
    // stack:      1, ptr, out, retdest
    DUP3
    // stack: out, 1, ptr, out, retdest
    %mstore_kernel_general
    // stack:         ptr, out, retdest
    %load_fp6
    // stack:        P, Q, out, retdest
    DUP1  DUP1
    // stack:     O, P, Q, out, retdest
    PUSH 62
    // stack: 62, O, P, Q, out, retdest
    %jump(miller_loop)

miller_loop:
    // stack:        times, O, P, Q, out, retdest
    DUP1
    // stack: times, times, O, P, Q, out, retdest
    mload_kernel_code(exp_runs)
    // stack:    nm, times, O, P, Q, out, retdest
    %jump(miller_step)

miller_step:
    

miller_decr:
    // stack:     times  , O, P, Q, out, retdest
    %sub_const(1)
    // stack:     times-1, O, P, Q, out, retdest
    DUP1  %jumpi(miller_loop)
    // stack:           0, O, P, Q, out, retdest
    %pop3  %pop3  %pop3
    // stack:                       out, retdest
    %jump(post_mllr)


/// def mul_tangent()
///     out = square_fp12(out)
///     line = tangent(O, Q)
///     out = mul_fp12_sparse(out, line)
///     O += O
///
/// def mul_cord()
///     line = cord(O, P, Q)
///     out = mul_fp12_sparse(out, line)
///     O += P

mul_tangent:



/// p1, p2 : [Fp; 2], q : [Fp2; 2]

/// def cord(p1x, p1y, p2x, p2y, qx, qy):
///     return sparse_embed(
///         p1y*p2x - p2y*p1x, 
///         (p2y - p1y) * qx, 
///         (p1x - p2x) * qy,
///     )
    
/// def tangent(px, py, qx, qy):
///     return sparse_embed(
///         -9 + py**2, 
///         (-3*px**2) * qx, 
///         (2*py)     * qy,
///     )
