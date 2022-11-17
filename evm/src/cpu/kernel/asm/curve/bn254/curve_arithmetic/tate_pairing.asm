/// def tate(P : [Fp; 2], Q: [Fp2; 2]) -> Fp12:
///     out = miller_loop(P)
///
///     inv = inverse_fp12(out)
///     out = frob_fp12_6(out)
///     out = mul_fp12(out, inv)
///
///     acx = frob_fp12_2(out)
///     out = mul_fp12(acx, out)
///
///     pow = fast_exp(out)
///     out = frob_fp12_3(out)
///     return mul_fp12(out, pow)

global tate:
    // stack:         ptr, out
    PUSH 1
    // stack:      1, ptr, out
    PUSH 100
    // stack: 100, 1, ptr, out
    %mstore_kernel_general


/// def miller_loop(P):
///     out = 1
///     O = P
///     for i in EXP[1:-1]:
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



