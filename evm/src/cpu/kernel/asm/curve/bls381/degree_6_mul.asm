/// Given inputs:
///     A0 + A1t + A2t^2
///     B0 + B1t + B2t^2
/// Output: 
///     C0 + C1t + C2t^2
///   where
///     C0 = A0B0 + i1(A1B2 + A2B1)
///     C1 = A0B1 + A1B0 + i1(A2B2)
///     C2 = A0B2 + A1B1 + A2B0

global mul_fp381_6:
    // stack:                            inA, inB, out, jumpdest
    PUSH mul_fp381_6_00
    // stack:                       ret, inA, inB, out, jumpdest
    DUP3
    // stack:                 inB0, ret, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:                   B0, ret, inA, inB, out, jumpdest
    DUP6
    // stack:             inA0, B0, ret, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:               A0, B0, ret, inA, inB, out, jumpdest
    %jump(mul_fp381_2)

mul_fp381_6_00:
    // stack:                      A0B0, inA, inB, out, jumpdest
    PUSH mul_fp381_6_01
    // stack:                 ret, A0B0, inA, inB, out, jumpdest
    DUP7
    %add_const(8)
    // stack:           inB2, ret, A0B0, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:             B2, ret, A0B0, inA, inB, out, jumpdest
    DUP10
    %add_const(4)
    // stack:       inA1, B2, ret, A0B0, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:         A1, B2, ret, A0B0, inA, inB, out, jumpdest
    %jump(mul_fp381_2)
    
mul_fp381_6_01:
    // stack:                A1B2, A0B0, inA, inB, out, jumpdest
    PUSH mul_fp381_6_02
    // stack:           ret, A1B2, A0B0, inA, inB, out, jumpdest
    DUP11
    %add_const(4)
    // stack:     inB1, ret, A1B2, A0B0, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:       B1, ret, A1B2, A0B0, inA, inB, out, jumpdest
    DUP14
    %add_const(8)
    // stack: inA2, B1, ret, A1B2, A0B0, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:   A2, B1, ret, A1B2, A0B0, inA, inB, out, jumpdest
    %jump(mul_fp381_2)

mul_fp381_6_02:
    // stack:        A2B1 , A1B2 , A0B0, inA, inB, out, jumpdest
    %add_fp381_2
    // stack:        A2B1 + A1B2 , A0B0, inA, inB, out, jumpdest
    %i1
    // stack:     i1(A2B1 + A1B2), A0B0, inA, inB, out, jumpdest
    %add_fp381_2
    // stack:                        C0, inA, inB, out, jumpdest
    DUP7
    // stack:                  out0, C0, inA, inB, out, jumpdest
    %mstore_bls_fp2
    
    // stack:                            inA, inB, out, jumpdest
    PUSH mul_fp381_6_10
    // stack:                       ret, inA, inB, out, jumpdest
    DUP3
    %add_const(4)
    // stack:                 inB1, ret, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:                   B1, ret, inA, inB, out, jumpdest
    DUP6
    // stack:             inA0, B1, ret, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:               A0, B1, ret, inA, inB, out, jumpdest
    %jump(mul_fp381_2)

mul_fp381_6_10:
    // stack:                      A0B1, inA, inB, out, jumpdest
    PUSH mul_fp381_6_11
    // stack:                 ret, A0B1, inA, inB, out, jumpdest
    DUP7
    // stack:           inB0, ret, A0B1, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:             B0, ret, A0B1, inA, inB, out, jumpdest
    DUP10
    %add_const(4)
    // stack:       inA1, B0, ret, A0B1, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:         A1, B0, ret, A0B1, inA, inB, out, jumpdest
    %jump(mul_fp381_2)

mul_fp381_6_11:
    // stack:                A1B0, A0B1, inA, inB, out, jumpdest
    PUSH mul_fp381_6_12
    // stack:           ret, A1B0, A0B1, inA, inB, out, jumpdest
    DUP11
    %add_const(8)
    // stack:     inB2, ret, A1B0, A0B1, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:       B2, ret, A1B0, A0B1, inA, inB, out, jumpdest
    DUP14
    %add_const(8)
    // stack: inA2, B2, ret, A1B0, A0B1, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:   A2, B2, ret, A1B0, A0B1, inA, inB, out, jumpdest
    %jump(mul_fp381_2)

mul_fp381_6_12:
    // stack:        A2B2  , A1B0, A0B1, inA, inB, out, jumpdest
    %i1
    // stack:     i1(A2B2) , A1B0, A0B1, inA, inB, out, jumpdest
    %add_fp381_2
    // stack:     i1(A2B2) + A1B0, A0B1, inA, inB, out, jumpdest
    %add_fp381_2
    // stack:                        C1, inA, inB, out, jumpdest
    DUP7
    %add_const(4)
    // stack:                  out1, C1, inA, inB, out, jumpdest
    %mstore_bls_fp2
    
    // stack:                            inA, inB, out, jumpdest
    PUSH mul_fp381_6_20
    // stack:                       ret, inA, inB, out, jumpdest
    DUP3
    %add_const(8)
    // stack:                 inB2, ret, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:                   B2, ret, inA, inB, out, jumpdest
    DUP6
    // stack:             inA0, B2, ret, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:               A0, B2, ret, inA, inB, out, jumpdest
    %jump(mul_fp381_2)

mul_fp381_6_20:
    // stack:                      A0B2, inA, inB, out, jumpdest
    PUSH mul_fp381_6_21
    // stack:                 ret, A0B2, inA, inB, out, jumpdest
    DUP7
    %add_const(4)
    // stack:           inB1, ret, A0B2, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:             B1, ret, A0B2, inA, inB, out, jumpdest
    DUP10
    %add_const(4)
    // stack:       inA1, B1, ret, A0B2, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:         A1, B1, ret, A0B2, inA, inB, out, jumpdest
    %jump(mul_fp381_2)

mul_fp381_6_21:
    // stack:                A1B1, A0B2, inA, inB, out, jumpdest
    PUSH mul_fp381_6_22
    // stack:           ret, A1B1, A0B2, inA, inB, out, jumpdest
    DUP11
    // stack:     inB0, ret, A1B1, A0B2, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:       B0, ret, A1B1, A0B2, inA, inB, out, jumpdest
    DUP14
    %add_const(8)
    // stack: inA2, B0, ret, A1B1, A0B2, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:   A2, B0, ret, A1B1, A0B2, inA, inB, out, jumpdest
    %jump(mul_fp381_2)

mul_fp381_6_22:
    // stack:         A2B0 , A1B1, A0B2, inA, inB, out, jumpdest
    %add_fp381_2
    // stack:         A2B2 + A1B1, A0B2, inA, inB, out, jumpdest
    %add_fp381_2
    // stack:                        C2, inA, inB, out, jumpdest
    DUP7
    %add_const(8)
    // stack:                  out2, C2, inA, inB, out, jumpdest
    %mstore_bls_fp2
    
    // stack:                            inA, inB, out, jumpdest
    %pop3
    JUMP
