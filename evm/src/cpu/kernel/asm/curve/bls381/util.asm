// Load a single BLS value, consisting of two terms, from KernelGeneral
%macro mload_bls
    // stack:            offset
    DUP1
    %add_const(1)
    // stack: offset_hi, offset
    %mload_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:    val_hi, offset
    SWAP1
    // stack: offset_lo, val_hi
    %mload_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:    val_lo, val_hi
%endmacro

// Store a single BLS value, consisting of two terms, to KernelGeneral
%macro mstore_bls
    // stack:            offset, val_lo, val_hi
    SWAP1
    // stack:            val_lo, offset, val_hi
    DUP2
    // stack: offset_lo, val_lo, offset, val_hi
    %mstore_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:                    offset, val_hi
    %add_const(1)
    // stack:                 offset_hi, val_hi
    %mstore_kernel(@SEGMENT_KERNEL_GENERAL)
%endmacro

%macro mload_bls_fp2
    // stack:                                        offset
    DUP1
    %add_const(3)
    // stack:                          offset_im_hi, offset
    %mload_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:                             val_im_hi, offset
    SWAP1
    // stack:                             offset, val_im_hi
    DUP1
    %add_const(2)
    // stack:               offset_im_lo, offset, val_im_hi
    %mload_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:                  val_im_lo, offset, val_im_hi
    SWAP1
    // stack:                  offset, val_im_lo, val_im_hi
    DUP1
    %add_const(1)
    // stack:    offset_re_hi, offset, val_im_lo, val_im_hi
    %mload_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:       val_re_hi, offset, val_im_lo, val_im_hi
    SWAP1
    // stack: offset_re_lo, val_re_hi, val_im_lo, val_im_hi
    %mload_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:    val_re_lo, val_re_hi, val_im_lo, val_im_hi
%endmacro

%macro mstore_bls_fp2
    // stack:               offset, val_re_lo, val_re_hi, val_im_lo, val_im_hi
    SWAP3
    // stack:               val_im_lo, val_re_lo, val_re_hi, offset, val_im_hi
    DUP4
    %add_const(2)
    // stack: offset_im_lo, val_im_lo, val_re_lo, val_re_hi, offset, val_im_hi
    %mstore_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:                          val_re_lo, val_re_hi, offset, val_im_hi
    DUP3
    // stack:            offset_re_lo, val_re_lo, val_re_hi, offset, val_im_hi
    %mstore_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:                                     val_re_hi, offset, val_im_hi
    DUP2
    %add_const(1)
    // stack:                       offset_re_hi, val_re_hi, offset, val_im_hi
    %mstore_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:                                                offset, val_im_hi
    %add_const(3)
    // stack:                                          offset_im_hi, val_im_hi
    %mstore_kernel(@SEGMENT_KERNEL_GENERAL)
%endmacro


%macro add_fp381
    // stack:         x0, x1, y0, y1
    PROVER_INPUT(sf::bls381_base::add_hi)
    // stack:     z1, x0, x1, y0, y1
    SWAP4
    // stack:     y1, x0, x1, y0, z1
    PROVER_INPUT(sf::bls381_base::add_lo)
    // stack: z0, y1, x0, x1, y0, z1
    SWAP4
    // stack: y0, y1, x0, x1, z0, z1
    %pop4
    // stack:                 z0, z1
%endmacro

%macro sub_fp381
    // stack:         x0, x1, y0, y1
    PROVER_INPUT(sf::bls381_base::sub_hi)
    // stack:     z1, x0, x1, y0, y1
    SWAP4
    // stack:     y1, x0, x1, y0, z1
    PROVER_INPUT(sf::bls381_base::sub_lo)
    // stack: z0, y1, x0, x1, y0, z1
    SWAP4
    // stack: y0, y1, x0, x1, z0, z1
    %pop4
    // stack:                 z0, z1
%endmacro

%macro mul_fp381
    // stack:         x0, x1, y0, y1
    PROVER_INPUT(sf::bls381_base::mul_hi)
    // stack:     z1, x0, x1, y0, y1
    SWAP4
    // stack:     y1, x0, x1, y0, z1
    PROVER_INPUT(sf::bls381_base::mul_lo)
    // stack: z0, y1, x0, x1, y0, z1
    SWAP4
    // stack: y0, y1, x0, x1, z0, z1
    %pop4
    // stack:                 z0, z1
%endmacro

global test_add_fp381:
    %add_fp381
    %jump(0xdeadbeef)

global test_sub_fp381:
    %sub_fp381
    %jump(0xdeadbeef)

global test_mul_fp381:
    %mul_fp381
    %jump(0xdeadbeef)


%macro add_fp381_2
    // stack: x_re, x_im, y_re, y_im
    %stack (x_re: 2, x_im: 2, y_re: 2, y_im: 2) -> (y_im, x_im, y_re, x_re)
    // stack: y_im, x_im, y_re, x_re
    %add_fp381
    // stack:       z_im, y_re, x_re
    %stack (z_im: 2, y_re: 2, x_re: 2) -> (x_re, y_re, z_im)
    // stack:       x_re, y_re, z_im
    %add_fp381
    // stack:             z_re, z_im
%endmacro

%macro sub_fp381_2
    // stack: x_re, x_im, y_re, y_im
    %stack (x_re: 2, x_im: 2, y_re: 2, y_im: 2) -> (x_im, y_im, y_re, x_re)
    // stack: x_im, y_im, y_re, x_re
    %sub_fp381
    // stack:       z_im, y_re, x_re
    %stack (z_im: 2, y_re: 2, x_re: 2) -> (x_re, y_re, z_im)
    // stack:       x_re, y_re, z_im
    %sub_fp381
    // stack:             z_re, z_im
%endmacro

global test_add_fp381_2:
    %add_fp381_2
    %jump(0xdeadbeef)

global test_sub_fp381_2:
    %sub_fp381_2
    %jump(0xdeadbeef)


global mul_fp381_2:
    // stack:                          x_re, x_im, y_re, y_im, jumpdest
    DUP4
    DUP4
    // stack:                    x_im, x_re, x_im, y_re, y_im, jumpdest
    DUP8
    DUP8
    // stack:              y_re, x_im, x_re, x_im, y_re, y_im, jumpdest
    DUP12
    DUP12
    // stack:        y_im, y_re, x_im, x_re, x_im, y_re, y_im, jumpdest
    DUP8
    DUP8
    // stack: x_re , y_im, y_re, x_im, x_re, x_im, y_re, y_im, jumpdest
    %mul_fp381
    // stack: x_re * y_im, y_re, x_im, x_re, x_im, y_re, y_im, jumpdest
    %stack (v: 2, y_re: 2, x_im: 2) ->  (x_im, y_re, v)
    // stack:  x_im , y_re, x_re*y_im, x_re, x_im, y_re, y_im, jumpdest
    %mul_fp381
    // stack:  x_im * y_re, x_re*y_im, x_re, x_im, y_re, y_im, jumpdest
    %add_fp381
    // stack:                    z_im, x_re, x_im, y_re, y_im, jumpdest
    %stack (z_im: 2, x_re: 2, x_im: 2, y_re: 2, y_im: 2) -> (x_im, y_im, y_re, x_re, z_im)
    // stack:                   x_im , y_im, y_re, x_re, z_im, jumpdest
    %mul_fp381
    // stack:                   x_im * y_im, y_re, x_re, z_im, jumpdest
    %stack (v: 2, y_re: 2, x_re: 2) -> (x_re, y_re, v)
    // stack:                    x_re , y_re, x_im*y_im, z_im, jumpdest
    %mul_fp381
    // stack:                    x_re * y_re, x_im*y_im, z_im, jumpdest
    %sub_fp381
    // stack:                                      z_re, z_im, jumpdest
    %stack (z_re: 2, z_im: 2, jumpdest) -> (jumpdest, z_re, z_im)
    JUMP


%macro i1
    // stack:             x_re, x_im
    %stack (x_re: 2, x_im: 2) -> (x_re, x_im, x_im, x_re)
    // stack: x_re, x_im, x_im, x_re
    %add_fp381
    // stack:       z_im, x_im, x_re
    %stack (z_im: 2, x_im: 2, x_re: 2) -> (x_re, x_im, z_im)
    // stack:       x_re, x_im, z_im
    %sub_fp381
    // stack:             z_re, z_im
%endmacro


global add_fp381_6:
    // stack:           inA, inB, out, jumpdest  { out: [ 0,  0,  0,  0,  0,  0 ] }
    %add_term_kernel                          
    // stack:           inA, inB, out, jumpdest  { out: [C0,  0,  0,  0,  0,  0 ] }
    %add_term_kernel(2)                       
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1,  0,  0,  0,  0 ] }
    %add_term_kernel(4)                       
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1, C2,  0,  0,  0 ] }
    %add_term_kernel(6)                       
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1, C2, C3,  0,  0 ] }
    %add_term_kernel(8)                       
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1, C2, C3, C4,  0 ] }
    %add_term_kernel(10)                      
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1, C2, C3, C4, C5 ] }
    %pop3
    JUMP

global sub_fp381_6:
    // stack:           inA, inB, out, jumpdest  { out: [ 0,  0,  0,  0,  0,  0 ] }
    %sub_term_kernel                          
    // stack:           inA, inB, out, jumpdest  { out: [C0,  0,  0,  0,  0,  0 ] }
    %sub_term_kernel(2)                       
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1,  0,  0,  0,  0 ] }
    %sub_term_kernel(4)                       
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1, C2,  0,  0,  0 ] }
    %sub_term_kernel(6)                       
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1, C2, C3,  0,  0 ] }
    %sub_term_kernel(8)                       
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1, C2, C3, C4,  0 ] }
    %sub_term_kernel(10)                      
    // stack:           inA, inB, out, jumpdest  { out: [C0, C1, C2, C3, C4, C5 ] }
    %pop3
    JUMP

%macro add_term_kernel
    // stack:           inA, inB, out, jumpdest
    DUP2
    // stack:     inB0, inA, inB, out, jumpdest
    %mload_bls
    // stack:       B0, inA, inB, out, jumpdest
    DUP3
    // stack: inA0, B0, inA, inB, out, jumpdest
    %mload_bls
    // stack:   A0, B0, inA, inB, out, jumpdest
    %add_fp381
    // stack:       C0, inA, inB, out, jumpdest
    DUP5
    // stack: out0, C0, inA, inB, out, jumpdest
    %mstore_bls
%endmacro

%macro add_term_kernel(n)
    // stack:           inA, inB, out, jumpdest
    DUP2
    %add_const($n)
    // stack:     inBn, inA, inB, out, jumpdest
    %mload_bls
    // stack:       Bn, inA, inB, out, jumpdest
    DUP3
    %add_const($n)
    // stack: inAn, Bn, inA, inB, out, jumpdest
    %mload_bls
    // stack:   An, Bn, inA, inB, out, jumpdest
    %add_fp381
    // stack:       Cn, inA, inB, out, jumpdest
    DUP5
    %add_const($n)
    // stack: outn, Cn, inA, inB, out, jumpdest
    %mstore_bls
%endmacro

%macro sub_term_kernel
    // stack:           inA, inB, out, jumpdest
    DUP2
    // stack:     inB0, inA, inB, out, jumpdest
    %mload_bls
    // stack:       B0, inA, inB, out, jumpdest
    DUP3
    // stack: inA0, B0, inA, inB, out, jumpdest
    %mload_bls
    // stack:   A0, B0, inA, inB, out, jumpdest
    %sub_fp381
    // stack:       C0, inA, inB, out, jumpdest
    DUP5
    // stack: out0, C0, inA, inB, out, jumpdest
    %mstore_bls
%endmacro

%macro sub_term_kernel(n)
    // stack:           inA, inB, out, jumpdest
    DUP2
    %add_const($n)
    // stack:     inBn, inA, inB, out, jumpdest
    %mload_bls
    // stack:       Bn, inA, inB, out, jumpdest
    DUP3
    %add_const($n)
    // stack: inAn, Bn, inA, inB, out, jumpdest
    %mload_bls
    // stack:   An, Bn, inA, inB, out, jumpdest
    %sub_fp381
    // stack:       Cn, inA, inB, out, jumpdest
    DUP5
    %add_const($n)
    // stack: outn, Cn, inA, inB, out, jumpdest
    %mstore_bls
%endmacro

global add_fp381_6_sh:
    // stack:             inA, inB, out, jumpdest
    DUP1
    // stack:       inA0, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:         A0, inA, inB, out, jumpdest
    DUP6
    %add_const(8)
    // stack:  inB2 , A0, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:    B2 , A0, inA, inB, out, jumpdest
    %i1
    // stack: i1(B2), A0, inA, inB, out, jumpdest
    %add_fp381_2
    // stack:         C0, inA, inB, out, jumpdest
    DUP7
    // stack:   out0, C0, inA, inB, out, jumpdest
    %mstore_bls_fp2
    // stack:             inA, inB, out, jumpdest
    DUP1
    %add_const(4)
    // stack:       inA1, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:         A1, inA, inB, out, jumpdest
    DUP6
    // stack:   inB0, A1, inA, inB, out, jumpdest
    %mload_bls_fp2
    // stack:     B0, A1, inA, inB, out, jumpdest
    %add_fp381_2
    // stack:         C1, inA, inB, out, jumpdest
    DUP7
    %add_const(4)
    // stack:   out1, C1, inA, inB, out, jumpdest
    %mstore_bls_fp2
    // stack:             inA, inB, out, jumpdest
    %add_const(8)
    // stack:            inA2, inB, out, jumpdest
    %mload_bls_fp2
    // stack:              A2, inB, out, jumpdest
    DUP5
    %add_const(4)
    // stack:        inB1, A2, inB, out, jumpdest
    %mload_bls_fp2
    // stack:          B1, A2, inB, out, jumpdest
    %add_fp381_2
    // stack:              C2, inB, out, jumpdest
    DUP6
    %add_const(8)
    // stack:        out2, C2, inB, out, jumpdest
    %mstore_bls_fp2
    // stack:                  inB, out, jumpdest
    %pop2
    JUMP
