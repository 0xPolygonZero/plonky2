/// def tate(P : [Fp; 2], Q: [Fp2; 2]) -> Fp12:
///     out = miller_loop(P, Q)
///
///     inv = inverse_fp12(out)
///     out = frob_fp12_6(out)
///     out = mul_fp12(out, inv)
///
///     acc = frob_fp12_2_(out)
///     out = mul_fp12(out, acc)
///
///     pow = fast_exp(out)
///     out = frob_fp12_3(out) 
///     out = mul_fp12(out, pow)
///
///     return out

global tate:
    // stack:           ptr, out,            retdest
    PUSH post_mllr   SWAP2   SWAP1
    // stack:           ptr, out, post_mllr, retdest
    %jump(miller_init)
global post_mllr:
    // stack:                           out, retdest
    PUSH tate_inv
    // stack:                 tate_inv, out, retdest
    PUSH 100 
    // stack:            100, tate_inv, out, retdest
    DUP3 
    // stack:       out, 100, tate_inv, out, retdest
    %jump(inverse_fp12)
tate_inv:
    // stack:                           out, retdest  {100: inv}
    PUSH tate_mul1
    // stack:                tate_mul1, out, retdest  {100: inv}
    DUP2
    // stack:           out, tate_mul1, out, retdest  {100: inv}
    PUSH 100 
    // stack:      100, out, tate_mul1, out, retdest  {100: inv}
    DUP2
    // stack: out, 100, out, tate_mul1, out, retdest  {100: inv}
    %frob_fp12_6
    // stack: out, 100, out, tate_mul1, out, retdest  {100: inv}
    %jump(mul_fp12)
tate_mul1:
    // stack:                           out, retdest  {100: inv}
    PUSH tate_mul2
    // stack:                tate_mul2, out, retdest  {100: inv}
    DUP2
    // stack:           out, tate_mul2, out, retdest  {100: inv}
    PUSH 100
    // stack:      100, out, tate_mul2, out, retdest  {100: inv}       
    DUP2
    // stack: out, 100, out, tate_mul2, out, retdest  {100: inv}
    %frob_fp12_2_
    // stack: out, 100, out, tate_mul2, out, retdest  {100: inv} 
    %jump(mul_fp12)
tate_mul2: 
    // stack:                           out, retdest  {100: acc}
    PUSH post_pow
    // stack:                 post_pow, out, retdest  {100: acc}
    PUSH 100
    // stack:            100, post_pow, out, retdest  {100: acc}
    DUP3
    // stack:       out, 100, post_pow, out, retdest  {100: acc}
    %jump(power)
post_pow: 
    // stack:                           out, retdest  {100: pow}
    PUSH 100
    // stack:                      100, out, retdest  {100: pow}
    DUP2
    // stack:                 out, 100, out, retdest  {100: pow}
    %frob_fp12_3
    // stack:                 out, 100, out, retdest  {100: pow}
    %jump(mul_fp12)
