/// Given inputs:
///     A0 + A1z
///     B0 + B1z
/// Output: 
///     C0 + C1z
///   where
///     C0 = H0 + sh(H1)
///     C1 = H01 - (H0 + H1)
///     H0 = A0B0
///     H1 = A1B1
///     H01 = (A0 + A1)(B0 + B1)

global mul_fp381_12:
    // stack:                        inA, inB, out, jumpdest
    %stack (inA, inB) -> (inA, inB, 100, mul_fp381_12_h0, inA, inB)
    // stack:  inA0, inB0, 100, ret, inA, inB, out, jumpdest
    %jump(mul_fp381_6)
mul_fp381_12_h0:
    // stack:                        inA, inB, out, jumpdest  { 100: H0 }
    %stack () -> (112, mul_fp381_12_h1)
    // stack:              112, ret, inA, inB, out, jumpdest  { 100: H0 }
    DUP4
    %add_const(12)
    // stack:        inB1, 112, ret, inA, inB, out, jumpdest  { 100: H0 }
    DUP4
    %add_const(12)
    // stack:  inA1, inB1, 112, ret, inA, inB, out, jumpdest  { 100: H0 }
    %jump(mul_fp381_6)
mul_fp381_12_h1:
    // stack:                        inA, inB, out, jumpdest  { 100: H0, 112: H1 }
    %stack () -> (100, 112, 124, mul_fp381_12_h0h1)
    // stack:    100, 112, 124, ret, inA, inB, out, jumpdest  { 100: H0, 112: H1 }
    %jump(add_fp381_6)
mul_fp381_12_h0h1:
    // stack:                        inA, inB, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1 }
    %stack (inA) -> (inA, inA, 136, mul_fp381_12_a01)
    // stack:       inA , inA , 136, ret, inB, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1 }
    %add_const(12)
    // stack:       inA1, inA0, 136, ret, inB, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1 }
    %jump(add_fp381_6)
mul_fp381_12_a01:
    // stack:                             inB, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1, 136: A0+A1 }
    %stack (inB) -> (inB, inB, 148, mul_fp381_12_b01)
    // stack:            inB , inB , 148, ret, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1, 136: A0+A1 }
    %add_const(12)
    // stack:            inB1, inB0, 148, ret, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1, 136: A0+A1 }
    %jump(add_fp381_6)
mul_fp381_12_b01:
    // stack:                                  out, jumpdest  { 100: H0, 112: H1, 124: H0+H1, 136: A0+A1, 148: B0+B1 }
    %stack (out) -> (out, mul_fp381_12_h01, out)
    %add_const(12)
    // stack:                       out1, ret, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1, 136: A0+A1, 148: B0+B1 }
    %stack () -> (136, 148)
    // stack:             136, 148, out1, ret, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1, 136: A0+A1, 148: B0+B1 }
    %jump(mul_fp381_6)
mul_fp381_12_h01:
    // stack:                                  out, jumpdest  { 100: H0, 112: H1, 124: H0+H1, out1: H01 }
    %stack (out) -> (out, mul_fp381_12_c1, out)
    %add_const(12)
    // stack:                       out1, ret, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1, out1: H01 }
    %stack (out1) -> (out1, 124, out1)
    // stack:            out1, 124, out1, ret, out, jumpdest  { 100: H0, 112: H1, 124: H0+H1, out1: H01 }
    %jump(sub_fp381_6)
mul_fp381_12_c1:
    // stack:                                  out, jumpdest  { 100: H0, 112: H1, out1: C1 }
    %stack () -> (100, 112)
    // stack:                       100, 112, out0, jumpdest  { 100: H0, out0: sh(H1), out1: C1 }
    %jump(add_fp381_6_sh)
