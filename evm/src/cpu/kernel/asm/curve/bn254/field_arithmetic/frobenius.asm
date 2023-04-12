// frob_fp12 tests

global test_frob_fp254_12_1:
    // stack:         ptr
    %frob_fp254_12_1
    // stack:         ptr
    %jump(0xdeadbeef)

global test_frob_fp254_12_2:
    // stack:         ptr 
    DUP1
    // stack:    ptr, ptr
    %frob_fp254_12_2_
    // stack:         ptr
    %jump(0xdeadbeef)

global test_frob_fp254_12_3:
    // stack:         ptr
    %frob_fp254_12_3
    // stack:         ptr
    %jump(0xdeadbeef)

global test_frob_fp254_12_6:
    // stack:         ptr
    %frob_fp254_12_6
    // stack:         ptr
    %jump(0xdeadbeef)


/// def frob_fp254_12_n(f, f'):
///     g  =             frob_fp254_6(n, f )
///     g' = FROB_z[n] * frob_fp254_6(n, f')
///     return g, g'

%macro frob_fp254_12_1
    // stack:           ptr
    DUP1
    // stack:      ptr, ptr 
    %load_fp254_6
    // stack:        f, ptr
    %frob_fp254_6_1
    // stack:        g, ptr
    DUP7
    // stack:   ptr, g, ptr
    %store_fp254_6
    // stack:           ptr
    DUP1  %add_const(6)
    // stack:     ptr', ptr
    %load_fp254_6
    // stack:       f', ptr
    %frobz_1
    // stack:       g', ptr
    DUP7  %add_const(6)
    // stack: ptr', g', ptr
    %store_fp254_6
    // stack:           ptr
%endmacro 

// Note: this is the only one with distinct input and output pointers
%macro frob_fp254_12_2_
    // stack:           ptr , out
    DUP1
    // stack:      ptr, ptr , out
    %load_fp254_6
    // stack:        f, ptr , out
    %frob_fp254_6_2
    // stack:        g, ptr , out
    DUP8
    // stack:   out, g, ptr , out
    %store_fp254_6 
    // stack:           ptr , out
    %add_const(6)
    // stack:           ptr', out
    %load_fp254_6
    // stack:             f', out
    %frobz_2
    // stack:             g', out
    DUP7  %add_const(6)
    // stack:       out', g', out
    %store_fp254_6
    // stack:                 out
%endmacro 

%macro frob_fp254_12_3
    // stack:           ptr
    DUP1
    // stack:      ptr, ptr 
    %load_fp254_6
    // stack:        f, ptr
    %frob_fp254_6_3
    // stack:        g, ptr
    DUP7
    // stack:   ptr, g, ptr
    %store_fp254_6
    // stack:           ptr
    DUP1  %add_const(6)
    // stack:     ptr', ptr
    %load_fp254_6
    // stack:       f', ptr
    %frobz_3
    // stack:       g', ptr
    DUP7  %add_const(6)
    // stack: ptr', g', ptr
    %store_fp254_6
    // stack:           ptr
%endmacro

%macro frob_fp254_12_6
    // stack:           ptr
    DUP1  %add_const(6)
    // stack:     ptr', ptr
    %load_fp254_6
    // stack:       f', ptr
    %frobz_6
    // stack:       g', ptr
    DUP7  %add_const(6)
    // stack: ptr', g', ptr
    %store_fp254_6
    // stack:           ptr
%endmacro

// frob_fp12 tests

global test_frob_fp254_6_1:
    // stack:         ptr
    %frob_fp254_6_1
    // stack:         ptr
    %jump(0xdeadbeef)

global test_frob_fp254_6_2:
    // stack:         ptr 
    %frob_fp254_6_2
    // stack:         ptr
    %jump(0xdeadbeef)

global test_frob_fp254_6_3:
    // stack:         ptr
    %frob_fp254_6_3
    // stack:         ptr
    %jump(0xdeadbeef)


/// let Z` denote the complex conjugate of Z

/// def frob_fp254_6_n(C0, C1, C2):
///     if n%2:
///         D0, D1, D2 = C0`, FROB_T1[n] * C1`, FROB_T2[n] * C2`
///     else: 
///         D0, D1, D2 = C0 , FROB_T1[n] * C1 , FROB_T2[n] * C2
///     return D0, D1, D2 

%macro frob_fp254_6_1
    // stack: C0 , C1 , C2
    %conj_fp254_2
    // stack: D0 , C1 , C2
    %stack (x: 2, a: 2, y:2) -> (y, a, x)
    // stack: C2 , C1 , D0
    %conj_fp254_2
    // stack: C2`, C1 , D0
    %frobt2_1
    // stack: D2 , C1 , D0
    %stack (x: 2, a: 2, y:2) -> (y, a, x)
    // stack: D0 , C1 , D2
    %stack (x: 2, y: 2) -> (y, x)
    // stack: C1 , D0 , D2
    %conj_fp254_2
    // stack: C1`, D0 , D2
    %frobt1_1
    // stack: D1 , D0 , D2
    %stack (x: 2, y: 2) -> (y, x)
    // stack: D0 , D1 , D2
%endmacro

%macro frob_fp254_6_2
    // stack: C0, C1, C2
    %stack (x: 2, a: 2, y:2) -> (y, a, x)
    // stack: C2, C1, C0
    %frobt2_2
    // stack: D2, C1, C0
    %stack (x: 2, a: 2, y:2) -> (y, a, x)
    // stack: C0, C1, D2
    %stack (x: 2, y: 2) -> (y, x)
    // stack: C1, C0, D2
    %frobt1_2
    // stack: D1, C0, D2
    %stack (x: 2, y: 2) -> (y, x)
    // stack: D0, D1, D2
%endmacro

%macro frob_fp254_6_3
    // stack: C0 , C1 , C2
    %conj_fp254_2
    // stack: D0 , C1 , C2
    %stack (x: 2, a: 2, y:2) -> (y, a, x)
    // stack: C2 , C1 , D0
    %conj_fp254_2
    // stack: C2`, C1 , D0
    %frobt2_3
    // stack: D2 , C1 , D0
    %stack (x: 2, a: 2, y:2) -> (y, a, x)
    // stack: D0 , C1 , D2
    %stack (x: 2, y: 2) -> (y, x)
    // stack: C1 , D0 , D2
    %conj_fp254_2
    // stack: C1`, D0 , D2
    %frobt1_3
    // stack: D1 , D0 , D2
    %stack (x: 2, y: 2) -> (y, x)
    // stack: D0 , D1 , D2
%endmacro


%macro frobz_1
    %frob_fp254_6_1
    PUSH 0x246996f3b4fae7e6a6327cfe12150b8e747992778eeec7e5ca5cf05f80f362ac
    PUSH 0x1284b71c2865a7dfe8b99fdd76e68b605c521e08292f2176d60b35dadcc9e470
    %scale_fp254_6
%endmacro

%macro frobz_2
    %frob_fp254_6_2
    PUSH 0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd49
    %scale_re_fp254_6
%endmacro

%macro frobz_3
    %frob_fp254_6_3
    PUSH 0xabf8b60be77d7306cbeee33576139d7f03a5e397d439ec7694aa2bf4c0c101
    PUSH 0x19dc81cfcc82e4bbefe9608cd0acaa90894cb38dbe55d24ae86f7d391ed4a67f
    %scale_fp254_6
%endmacro

%macro frobz_6
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd46
    %scale_re_fp254_6
%endmacro


%macro frobt1_1
    PUSH 0x16c9e55061ebae204ba4cc8bd75a079432ae2a1d0b7c9dce1665d51c640fcba2
    PUSH 0x2fb347984f7911f74c0bec3cf559b143b78cc310c2c3330c99e39557176f553d
    %mul_fp254_2
%endmacro

%macro frobt2_1
    PUSH 0x2c145edbe7fd8aee9f3a80b03b0b1c923685d2ea1bdec763c13b4711cd2b8126
    PUSH 0x5b54f5e64eea80180f3c0b75a181e84d33365f7be94ec72848a1f55921ea762
    %mul_fp254_2
%endmacro

%macro frobt1_2
    PUSH 0x30644e72e131a0295e6dd9e7e0acccb0c28f069fbb966e3de4bd44e5607cfd48
    %scale_fp254_2
%endmacro

%macro frobt2_2
    PUSH 0x59e26bcea0d48bacd4f263f1acdb5c4f5763473177fffffe
    %scale_fp254_2
%endmacro


%macro frobt1_3
    PUSH 0x4f1de41b3d1766fa9f30e6dec26094f0fdf31bf98ff2631380cab2baaa586de
    PUSH 0x856e078b755ef0abaff1c77959f25ac805ffd3d5d6942d37b746ee87bdcfb6d
    %mul_fp254_2
%endmacro

%macro frobt2_3
    PUSH 0x23d5e999e1910a12feb0f6ef0cd21d04a44a9e08737f96e55fe3ed9d730c239f
    PUSH 0xbc58c6611c08dab19bee0f7b5b2444ee633094575b06bcb0e1a92bc3ccbf066
    %mul_fp254_2
%endmacro
