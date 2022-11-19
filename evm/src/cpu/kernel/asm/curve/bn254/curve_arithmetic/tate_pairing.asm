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
    %jump(miller_loop)
post_mllr:
    // stack:                          out,            tate_mul1, tate_mul2, tate_mul3, retdest
    PUSH 100 
    // stack:                     100, out,            tate_mul1, tate_mul2, tate_mul3, retdest
    DUP2
    // stack:                out, 100, out,            tate_mul1, tate_mul2, tate_mul3, retdest
    %inverse_fp12
    // stack:                     100, out,            tate_mul1, tate_mul2, tate_mul3, retdest  {100: inv}
    PUSH mul_fp12
    // stack:           mul_fp12, 100, out,            tate_mul1, tate_mul2, tate_mul3, retdest  {100: inv}
    DUP3
    // stack:      out, mul_fp12, 100, out,            tate_mul1, tate_mul2, tate_mul3, retdest  {100: inv}
    %jump(frob_fp12_6)
tate_mul1:
    // stack:                          out,                       tate_mul2, tate_mul3, retdest  {100: inv}
    DUP1
    // stack:                     out, out,                       tate_mul2, tate_mul3, retdest  {100: inv}
    PUSH mul_fp12
    // stack:           mul_fp12, out, out,                       tate_mul2, tate_mul3, retdest  {100: inv}
    PUSH 100
    // stack:      100, mul_fp12, out, out,                       tate_mul2, tate_mul3, retdest  {100: inv}       
    DUP3
    // stack: out, 100, mul_fp12, out, out,                       tate_mul2, tate_mul3, retdest  {100: inv}
    %jump(frob_fp12_2)
tate_mul2: 
    // stack:                          out,                                  tate_mul3, retdest  {100: acc}
    PUSH post_pow
    // stack:                post_pow, out,                                  tate_mul3, retdest  {100: acc}
    PUSH 100
    // stack:           100, post_pow, out,                                  tate_mul3, retdest  {100: acc}
    DUP1
    // stack:      out, 100, post_pow, out,                                  tate_mul3, retdest  {100: acc}
    %jump(power)
post_pow: 
    // stack:                     100, out,                                  tate_mul3, retdest  {100: pow}
    PUSH mul_fp12
    // stack:           mul_fp12, 100, out,                                  tate_mul3, retdest  {100: pow}
    DUP3 
    // stack:      out, mul_fp12, 100, out,                                  tate_mul3, retdest  {100: pow}
    %jump(frob_fp12_3)
tate_mul3:
    // stack:                          out,                                             retdest  {100: pow}
    SWAP1
    JUMP


/// def miller_loop(P, Q):
///     out = 1
///     O = P
///     for i in EXP:
///         out = square_fp12(out)
///         line = tangent(O, Q)
///         out = mul_fp12_sparse(out, line)
///         O += O
///         if i:
///             line = cord(P, O, Q)
///             out = mul_fp12_sparse(out, line)
///             O += P
///     out = square_fp12(out)
///     line = tangent(O, Q)
///     return mul_fp12_sparse(out, line)
///
/// EXP is the binary expansion of the BN254 prime

global miller_loop:
    // stack: ptr, out, retdest

    // stack:      out
