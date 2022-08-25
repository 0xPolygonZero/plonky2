%macro load_K
    // stack: rnd
    %mul_const(4)  push K_data  add
    // stack: K_data + 4*rnd
    %mload_kernel_code_u32
    // stack: K
%end_macro

K_data:
    // Left
    BYTES 0x00, 0x00, 0x00, 0x00
    BYTES 0x5A, 0x82, 0x79, 0x99
    BYTES 0x6E, 0xD9, 0xEB, 0xA1
    BYTES 0x8F, 0x1B, 0xBC, 0xDC
    BYTES 0xA9, 0x53, 0xFD, 0x4E
    // Right
    BYTES 0x50, 0xA2, 0x8B, 0xE6
    BYTES 0x5C, 0x4D, 0xD1, 0x24
    BYTES 0x6D, 0x70, 0x3E, 0xF3
    BYTES 0x7A, 0x6D, 0x76, 0xE9
    BYTES 0x00, 0x00, 0x00, 0x00


%macro load_s
    // stack: box
    push S_data  add
    // stack: S_data + box
    %mload_kernel_code
    // stack: s
%end_macro

S_data:
    // Left Round 1
    BYTES 11, 14, 15, 12
    BYTES 05, 08, 07, 09
    BYTES 11, 13, 14, 15
    BYTES 06, 07, 09, 08
    // Left Round 2
    BYTES 07, 06, 08, 13 
    BYTES 11, 09, 07, 15 
    BYTES 07, 12, 15, 09 
    BYTES 11, 07, 13, 12
    // Left Round 3
    BYTES 11, 13, 06, 07 
    BYTES 14, 09, 13, 15 
    BYTES 14, 08, 13, 06 
    BYTES 05, 12, 07, 05
    // Left Round 4
    BYTES 11, 12, 14, 15 
    BYTES 14, 15, 09, 08 
    BYTES 09, 14, 05, 06 
    BYTES 08, 06, 05, 12
    // Left Round 5
    BYTES 09, 15, 05, 11
    BYTES 06, 08, 13, 12
    BYTES 05, 12, 13, 14
    BYTES 11, 08, 05, 06

    // Right Round 1
    BYTES 08, 09, 09, 11
    BYTES 13, 15, 15, 05 
    BYTES 07, 07, 08, 11 
    BYTES 14, 14, 12, 06
    // Right Round 2
    BYTES 09, 13, 15, 07 
    BYTES 12, 08, 09, 11 
    BYTES 07, 07, 12, 07
    BYTES 06, 15, 13, 11
    // Right Round 3
    BYTES 09, 07, 15, 11 
    BYTES 08, 06, 06, 14 
    BYTES 12, 13, 05, 14 
    BYTES 13, 13, 07, 05
    // Right Round 4
    BYTES 15, 05, 08, 11 
    BYTES 14, 14, 06, 14 
    BYTES 06, 09, 12, 09 
    BYTES 12, 05, 15, 08
    // Right Round 5
    BYTES 08, 05, 12, 09 
    BYTES 12, 05, 14, 06 
    BYTES 08, 13, 06, 05 
    BYTES 15, 13, 11, 11


%macro load_r
    // stack: box
    push R_data  add
    // stack: R_data + box
    %mload_kernel_code
    // stack: r
%end_macro

R_data:
    // Left Round 1
    BYTES 00, 01, 02, 03
    BYTES 04, 05, 06, 07
    BYTES 08, 09, 10, 11
    BYTES 12, 13, 14, 15
    // Left Round 2
    BYTES 07, 04, 13, 01
    BYTES 10, 06, 15, 03
    BYTES 12, 00, 09, 05
    BYTES 02, 14, 11, 08
    // Left Round 3
    BYTES 03, 10, 14, 04
    BYTES 09, 15, 08, 01
    BYTES 02, 07, 00, 06
    BYTES 13, 11, 05, 12
    // Left Round 4
    BYTES 01, 09, 11, 10
    BYTES 00, 08, 12, 04
    BYTES 13, 03, 07, 15
    BYTES 14, 05, 06, 02
    // Left Round 5
    BYTES 04, 00, 05, 09
    BYTES 07, 12, 02, 10
    BYTES 14, 01, 03, 08
    BYTES 11, 06, 15, 13
    // Right Round 1
    BYTES 05, 14, 07, 00
    BYTES 09, 02, 11, 04
    BYTES 13, 06, 15, 08
    BYTES 01, 10, 03, 12
    // Right Round 2
    BYTES 06, 11, 03, 07
    BYTES 00, 13, 05, 10
    BYTES 14, 15, 08, 12
    BYTES 04, 09, 01, 02
    // Right Round 3
    BYTES 15, 05, 01, 03
    BYTES 07, 14, 06, 09
    BYTES 11, 08, 12, 02
    BYTES 10, 00, 04, 13
    // Right Round 4
    BYTES 08, 06, 04, 01
    BYTES 03, 11, 15, 00
    BYTES 05, 12, 02, 13
    BYTES 09, 07, 10, 14
    // Right Round 5
    BYTES 12, 15, 10, 04
    BYTES 01, 05, 08, 07
    BYTES 06, 02, 13, 14
    BYTES 00, 03, 09, 11
    