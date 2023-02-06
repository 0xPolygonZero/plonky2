/// def tate(P: Curve, Q: TwistedCurve) -> Fp12:
///     out = miller_loop(P, Q)
///     return make_invariant(P, Q)
global bn254_tate:
    // stack:                      inp, out, retdest
    %stack (inp, out) -> (inp, out, make_invariant, out)
    // stack: inp, out, make_invariant, out, retdest
    %jump(bn254_miller)


/// def make_invariant(y: Fp12):
///     y = first_exp(y)
///     y = second_exp(y)
///     return final_exponentiation(y)
make_invariant:

/// map t to t^(p^6 - 1) via 
///     def first_exp(t):
///         return t.frob(6) / t
    // stack:                      out, retdest  {out: y}
    %stack (out) -> (out, 100, first_exp, out)         
    // stack: out, 100, first_exp, out, retdest  {out: y}
    %jump(inv_fp254_12)
first_exp:
    // stack:                             out, retdest  {out: y  , 100: y^-1}
    %frob_fp254_12_6
    // stack:                             out, retdest  {out: y_6, 100: y^-1}
    %stack (out) -> (out, 100, out, second_exp, out)
    // stack:  out, 100, out, second_exp, out, retdest  {out: y_6, 100: y^-1}
    %jump(mul_fp254_12)

/// map t to t^(p^2 + 1) via 
///     def second_exp(t):
///         return t.frob(2) * t
second_exp:
    // stack:                                      out, retdest  {out: y}
    %stack (out) -> (out, 100, out, out, bn254_final_exp, out)
    // stack: out, 100, out, out, bn254_final_exp, out, retdest  {out: y}
    %frob_fp254_12_2_
    // stack:      100, out, out, bn254_final_exp, out, retdest  {out: y, 100: y_2}
    %jump(mul_fp254_12)
