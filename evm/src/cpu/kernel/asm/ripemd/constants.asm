global k_data:
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

global s_data:
    // Left Round 0
    BYTES 11, 14, 15, 12
    BYTES 05, 08, 07, 09
    BYTES 11, 13, 14, 15
    BYTES 06, 07, 09, 08
    // Left Round 1
    BYTES 07, 06, 08, 13 
    BYTES 11, 09, 07, 15 
    BYTES 07, 12, 15, 09 
    BYTES 11, 07, 13, 12
    // Left Round 2
    BYTES 11, 13, 06, 07 
    BYTES 14, 09, 13, 15 
    BYTES 14, 08, 13, 06 
    BYTES 05, 12, 07, 05
    // Left Round 3
    BYTES 11, 12, 14, 15 
    BYTES 14, 15, 09, 08 
    BYTES 09, 14, 05, 06 
    BYTES 08, 06, 05, 12
    // Left Round 4
    BYTES 09, 15, 05, 11
    BYTES 06, 08, 13, 12
    BYTES 05, 12, 13, 14
    BYTES 11, 08, 05, 06
    // Right Round 0
    BYTES 08, 09, 09, 11
    BYTES 13, 15, 15, 05 
    BYTES 07, 07, 08, 11 
    BYTES 14, 14, 12, 06
    // Right Round 1
    BYTES 09, 13, 15, 07 
    BYTES 12, 08, 09, 11 
    BYTES 07, 07, 12, 07
    BYTES 06, 15, 13, 11
    // Right Round 2
    BYTES 09, 07, 15, 11 
    BYTES 08, 06, 06, 14 
    BYTES 12, 13, 05, 14 
    BYTES 13, 13, 07, 05
    // Right Round 3
    BYTES 15, 05, 08, 11 
    BYTES 14, 14, 06, 14 
    BYTES 06, 09, 12, 09 
    BYTES 12, 05, 15, 08
    // Right Round 4
    BYTES 08, 05, 12, 09 
    BYTES 12, 05, 14, 06 
    BYTES 08, 13, 06, 05 
    BYTES 15, 13, 11, 11

global r_data:
    // Left Round 0
    BYTES 00, 04, 08, 12
    BYTES 16, 20, 24, 28
    BYTES 32, 36, 40, 44
    BYTES 48, 52, 56, 60
    // Left Round 1
    BYTES 28, 16, 52, 04
    BYTES 40, 24, 60, 12
    BYTES 48, 00, 36, 20
    BYTES 08, 56, 44, 32
    // Left Round 2
    BYTES 12, 40, 56, 16
    BYTES 36, 60, 32, 04
    BYTES 08, 28, 00, 24
    BYTES 52, 44, 20, 48
    // Left Round 3
    BYTES 04, 36, 44, 40
    BYTES 00, 32, 48, 16
    BYTES 52, 12, 28, 60
    BYTES 56, 20, 24, 08
    // Left Round 4
    BYTES 16, 00, 20, 36
    BYTES 28, 48, 08, 40
    BYTES 56, 04, 12, 32
    BYTES 44, 24, 60, 52
    // Right Round 0
    BYTES 20, 56, 28, 00
    BYTES 36, 08, 44, 16
    BYTES 52, 24, 60, 32
    BYTES 04, 40, 12, 48
    // Right Round 1
    BYTES 24, 44, 12, 28
    BYTES 00, 52, 20, 40
    BYTES 56, 60, 32, 48
    BYTES 16, 36, 04, 08
    // Right Round 2
    BYTES 60, 20, 04, 12
    BYTES 28, 56, 24, 36
    BYTES 44, 32, 48, 08
    BYTES 40, 00, 16, 52
    // Right Round 3
    BYTES 32, 24, 16, 04
    BYTES 12, 44, 60, 00
    BYTES 20, 48, 08, 52
    BYTES 36, 28, 40, 56
    // Right Round 4
    BYTES 48, 60, 40, 16
    BYTES 04, 20, 32, 28
    BYTES 24, 08, 52, 56
    BYTES 00, 12, 36, 44
