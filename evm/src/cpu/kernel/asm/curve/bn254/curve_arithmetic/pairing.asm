/// def tate(pairs: List((Curve, TwistedCurve))) -> Fp12:
///     out = 1
///     for P, Q in pairs:
///         out *= miller_loop(P, Q)
///     return bn254_final_exponent(out)

global bn254_pairing:
    // stack:       k    , inp, out, retdest
    DUP1
    ISZERO
    // stack: end?, k    , inp, out, retdest
    %jumpi(bn254_final_exponent)
    // stack:       k    , inp, out, retdest
    %sub_const(1)
    // stack:       k=k-1, inp, out, retdest

    %stack (k, inp, out) -> (k, inp, 200, mul_fp254_12, 200, out, out, bn254_pairing, k, inp, out)
    // stack: k, inp, 200, mul_fp254_12, 200, out, out, bn254_pairing, k, inp, out retdest
    %mul_const(6)
    ADD
    // stack:  inp_k, 200, mul_fp254_12, 200, out, out, bn254_pairing, k, inp, out retdest
    %jump(bn254_miller)
