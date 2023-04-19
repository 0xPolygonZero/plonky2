/// def tate(pairs: List((Curve, TwistedCurve))) -> Fp12:
///     out = 1
///     for P, Q in pairs:
///         out *= miller_loop(P, Q)
///     return bn254_final_exponent(out)

global bn254_tate:
    // stack:       k, inp, out, retdest
    DUP1
    ISZERO
    // stack: end?, k, inp, out, retdest
    %jumpi(bn254_final_exponent)
    // stack:       k, inp, out, retdest
    



    %stack (inp, out) -> (inp, out, bn254_final_exponent, out)
    // stack: inp, out, bn254_final_exponent, out, retdest
    %jump(bn254_miller)
