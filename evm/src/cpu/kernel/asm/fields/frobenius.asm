/// def frob_fp6(n, C0, C1, C2):
///     if n%2:
///         D0, D1, D2 = C0`, FROB_t1[n] * C1`, FROB_t2[n] * C2`
///     else: 
///         D0, D1, D2 = C0 , FROB_t1[n] * C1 , FROB_t2[n] * C2
///     return D0, D1, D2 

%macro frob_fp6_1
    // stack: C0 , C1 , C2
    %conj
    // stack: D0 , C1 , C2
    %swap_fp2_hole_2
    // stack: C2 , C1 , D0
    %conj
    // stack: C2`, C1 , D0
    PUSH 0x2c145edbe7fd8aee9f3a80b03b0b1c923685d2ea1bdec763c13b4711cd2b8126
    PUSH 0x5b54f5e64eea80180f3c0b75a181e84d33365f7be94ec72848a1f55921ea762
    %mul_fp2
    // stack: D2 , C1 , D0
    %swap_fp2_hole_2
    // stack: D0 , C1 , D2
    %swap_fp2
    // stack: C1 , D0 , D2
    %conj
    // stack: C1`, D0 , D2
    PUSH 0x16c9e55061ebae204ba4cc8bd75a079432ae2a1d0b7c9dce1665d51c640fcba2
    PUSH 0x2fb347984f7911f74c0bec3cf559b143b78cc310c2c3330c99e39557176f553d
    %mul_fp2
    // stack: D1 , D0 , D2
    %swap_fp2
    // stack: D0 , D1 , D2
%endmacro

%macro frob_fp6_2
    // stack: C0, C1, C2
    %swap_fp2_hole_2
    // stack: C2, C1, C0
    PUSH 0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe
    %mul_fp_fp2
    // stack: D2, C1, C0
    %swap_fp2_hole_2
    // stack: C0, C1, D2
    %swap_fp2
    // stack: C1, C0, D2
    PUSH 0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd48
    %mul_fp_fp2
    // stack: D1, C0, D2
    %swap_fp2
    // stack: D0, D1, D2
%endmacro

%macro frob_fp6_3
    // stack: C0 , C1 , C2
    %conj
    // stack: D0 , C1 , C2
    %swap_fp2_hole_2
    // stack: C2 , C1 , D0
    %conj
    // stack: C2`, C1 , D0
    PUSH 0x23d5e999e1910a12feb0f6ef0cd21d04a44a9e08737f96e55fe3ed9d730c239f
    PUSH 0xbc58c6611c08dab19bee0f7b5b2444ee633094575b06bcb0e1a92bc3ccbf066
    %mul_fp2
    // stack: D2 , C1 , D0
    %swap_fp2_hole_2
    // stack: D0 , C1 , D2
    %swap_fp2
    // stack: C1 , D0 , D2
    %conj
    // stack: C1`, D0 , D2
    PUSH 0x4f1de41b3d1766fa9f30e6dec26094f0fdf31bf98ff2631380cab2baaa586de
    PUSH 0x856e078b755ef0abaff1c77959f25ac805ffd3d5d6942d37b746ee87bdcfb6d
    %mul_fp2
    // stack: D1 , D0 , D2
    %swap_fp2
    // stack: D0 , D1 , D2
%endmacro


/// def Fp12_frob(n, f, f'):
/// return                frob_fp6(n, f ),
///           FROB_z[n] * frob_fp6(n, f')

global frob_fp12_1:


global frob_fp12_2:


global frob_fp12_3:


global frob_fp12_6:
    
