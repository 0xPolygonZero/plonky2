/// The input to the pairing script is a list of points
///     P_i = n_i*G: Curve, Q_i = m_i*H: TwistedCurve
/// where G, H are the respective generators, such that
///     sum_i n_i*m_i = 0
/// and therefore, due to bilinearity of the pairing:
///     prod_i e(P_i, Q_i) 
///   = prod_i e(n_i G, m_i H) 
///   = prod_i e(G,H)^{n_i * m_i} 
///   = e(G,H)^{sum_i n_i * m_i}
///   = e(G,H)^0
///   = 1: Fp12 

/// def bn254_pairing(pairs: List((Curve, TwistedCurve))) -> Bool:
///     
///     for P, Q in pairs:
///         if not (P.is_valid and Q.is_valid):
///             return @U256_MAX
///     
///     out = 1
///     for P, Q in pairs:
///         if P != 0 and Q != 0:
///             out *= miller_loop(P, Q)
///
///     result = bn254_final_exponent(out)
///     return result == unit_fp12

/// The following is a key to this API
/// 
/// - k is the number of inputs
/// - each input given by a pair of points, one on the curve and one on the twisted curve
/// - each input consists of 6 stack terms---2 for the curve point and 4 for the twisted curve point
/// - the inputs are presumed to be placed on the kernel contiguously
/// - the output (as defined above) is an Fp12 element
/// - out and inp are the BnPairing segment offsets for the output element and input
/// - the assembly code currently uses offsets 0-78 for scratch space

global bn254_pairing:
    // stack: k, inp, out, retdest 
    DUP1

bn254_input_check:
    // stack:       j    , k, inp 
    DUP1
    ISZERO
    // stack: end?, j    , k, inp
    %jumpi(bn254_pairing_start)
    // stack:       j    , k, inp
    %sub_const(1)
    // stack:       j=j-1, k, inp

    %stack (j, k, inp) -> (j, inp, j, k, inp)
    // stack:        j, inp, j, k, inp
    %mul_const(6)
    ADD
    // stack:  inp_j=inp+6j, j, k, inp
    DUP1
    // stack:  inp_j, inp_j, j, k, inp
    %load_fp254_2
    // stack:    P_j, inp_j, j, k, inp
    %bn_check
    // stack: valid?, inp_j, j, k, inp
    ISZERO
    %jumpi(bn_pairing_invalid_input)
    // stack:         inp_j, j, k, inp
    DUP1
    // stack: inp_j , inp_j, j, k, inp
    %add_const(2)
    // stack: inp_j', inp_j, j, k, inp
    %load_fp254_4
    // stack:    Q_j, inp_j, j, k, inp
    %bn_check_twisted
    // stack: valid?, inp_j, j, k, inp
    ISZERO
    %jumpi(bn_pairing_invalid_input)
    // stack:         inp_j, j, k, inp
    POP
    %jump(bn254_input_check)

bn_pairing_invalid_input:
    // stack:  inp_j, j, k, inp, out, retdest
    %stack (inp_j, j, k, inp, out, retdest) -> (retdest, @U256_MAX)
    JUMP

bn254_pairing_start:
    // stack:      0, k, inp, out,                   retdest
    %stack (j, k, inp, out) -> (out, 1, k, inp, out, bn254_pairing_output_validation, out)
    // stack: out, 1, k, inp, out, bn254_pairing_output_validation, out, retdest
    %mstore_bn254_pairing
    // stack:         k, inp, out, bn254_pairing_output_validation, out, retdest

bn254_pairing_loop:
    // stack:               k, inp, out, bn254_pairing_output_validation, out, retdest
    DUP1
    ISZERO
    // stack:         end?, k, inp, out, bn254_pairing_output_validation, out, retdest
    %jumpi(bn254_final_exponent)
    // stack:               k, inp, out, bn254_pairing_output_validation, out, retdest
    %sub_const(1)
    // stack:           k=k-1, inp, out, bn254_pairing_output_validation, out, retdest
    %stack (k, inp) -> (k, inp, k, inp)
    // stack:       k, inp, k, inp, out, bn254_pairing_output_validation, out, retdest
    %mul_const(6)
    ADD
    // stack:        inp_k, k, inp, out, bn254_pairing_output_validation, out, retdest
    DUP1
    %load_fp254_6
    // stack:  P, Q, inp_k, k, inp, out, bn254_pairing_output_validation, out, retdest
    %neutral_input
    // stack: skip?, inp_k, k, inp, out, bn254_pairing_output_validation, out, retdest
    %jumpi(bn_skip_input)
    // stack:        inp_k, k, inp, out, bn254_pairing_output_validation, out, retdest
    %stack (inp_k, k, inp, out) -> (bn254_miller, inp_k, 0, mul_fp254_12, 0, out, out, bn254_pairing_loop, k, inp, out)
    // stack: bn254_miller,                       inp_k, 0, 
    //        mul_fp254_12,                    0, out, out, 
    //        bn254_pairing_loop,              k, inp, out, 
    //        bn254_pairing_output_validation, out, retdest
    JUMP

bn_skip_input:
    // stack: inp_k, k, inp, out, bn254_pairing_output_validation, out, retdest
    POP
    // stack:        k, inp, out, bn254_pairing_output_validation, out, retdest
    %jump(bn254_pairing_loop)


bn254_pairing_output_validation:
    // stack:        out, retdest
    PUSH 1
    // stack: check, out, retdest
    %check_output_term
    %check_output_term(1)
    %check_output_term(2)
    %check_output_term(3)
    %check_output_term(4)
    %check_output_term(5)
    %check_output_term(6)
    %check_output_term(7)
    %check_output_term(8)
    %check_output_term(9)
    %check_output_term(10)
    %check_output_term(11)
    // stack: check, out, retdest
    %stack (check, out, retdest) -> (retdest, check)
    JUMP

%macro check_output_term
    // stack:          check, out
    DUP2
    // stack:    out0, check, out
    %mload_bn254_pairing
    // stack:      f0, check, out
    %eq_const(1)
    // stack:  check0, check, out
    MUL
    // stack:          check, out
%endmacro

%macro check_output_term(j)
    // stack:          check, out
    DUP2
    %add_const($j)
    // stack:    outj, check, out
    %mload_bn254_pairing
    // stack:      fj, check, out
    ISZERO
    // stack:  checkj, check, out
    MUL
    // stack:          check, out
%endmacro

%macro neutral_input
    // stack: P      , Q
    ISZERO
    SWAP1
    ISZERO
    MUL
    // stack: P==0,    Q
    SWAP4
    // stack: Q   , P==0
    ISZERO
    SWAP1
    ISZERO
    MUL
    SWAP1
    ISZERO
    MUL
    SWAP1
    ISZERO
    MUL
    // stack: Q==0, P==0
    OR
    // stack: Q==0||P==0
%endmacro