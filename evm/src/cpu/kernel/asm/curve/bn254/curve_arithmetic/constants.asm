/// miller_data is defined by
/// (1) taking the binary expansion of N254, the order of the elliptic curve group
/// (2) popping the first and last elements, then appending a 0:
///     exp = bin(N254)[1:-1] + [0]
/// (3) counting the lengths of runs of 1s then 0s in exp, e.g.
///     if exp = 1100010011110, then EXP = [(2,3), (1,2), (4,1)]
/// (4) byte encoding each pair (n,m) as follows:
///     miller_data = [(0x20)n + m for (n,m) in EXP]

global miller_data:
    BYTES 0xdc, 0x22, 0x42, 0x21
    BYTES 0xa1, 0xa4, 0x24, 0x21
    BYTES 0x23, 0x22, 0x64, 0x21
    BYTES 0x62, 0x41, 0x82, 0x24
    BYTES 0x22, 0x24, 0xa1, 0x42
    BYTES 0x25, 0x21, 0x22, 0x61
    BYTES 0x21, 0x44, 0x21, 0x21
    BYTES 0x46, 0x26, 0x41, 0x41
    BYTES 0x41, 0x21, 0x23, 0x25
    BYTES 0x21, 0x64, 0x41, 0x22
    BYTES 0x21, 0x27, 0x41, 0x43
    BYTES 0x22, 0x64, 0x21, 0x62
    BYTES 0x62, 0x22, 0x23, 0x42
    BYTES 0x25


/// final_exp first computes y^a4, y^a2, y^a0
/// representing a4, a2, a0 in *little endian* binary, define
///     EXPS4 = [(a4[i], a2[i], a0[i]) for i in       0..len(a4)]
///     EXPS2 = [       (a2[i], a0[i]) for i in len(a4)..len(a2)]
///     EXPS0 = [               a0[i]  for i in len(a2)..len(a0)]
/// power_data_n is simply a reverse-order byte encoding of EXPSn
///     where (i,j,k) is sent to (100)i + (10)j + k

global power_data_4:
    BYTES 111, 010, 011, 111
    BYTES 110, 101, 001, 100
    BYTES 001, 100, 110, 110
    BYTES 110, 011, 011, 101
    BYTES 011, 101, 101, 111
    BYTES 000, 011, 011, 001
    BYTES 011, 001, 101, 100
    BYTES 100, 000, 010, 100
    BYTES 110, 010, 110, 100
    BYTES 110, 101, 101, 001
    BYTES 001, 110, 110, 110
    BYTES 010, 110, 101, 001
    BYTES 010, 010, 110, 110
    BYTES 110, 010, 101, 110
    BYTES 101, 010, 101, 001
    BYTES 000, 111, 111, 110

global power_data_2:
    BYTES 11, 01, 11, 10
    BYTES 11, 10, 01, 10
    BYTES 00, 01, 10, 11
    BYTES 01, 11, 10, 01
    BYTES 00, 00, 00, 01
    BYTES 10, 01, 01, 10
    BYTES 00, 01, 11, 00
    BYTES 01, 00, 10, 11
    BYTES 11, 00, 11, 10
    BYTES 11, 00, 11, 01
    BYTES 11, 11, 11, 01
    BYTES 01, 00, 00, 11
    BYTES 00, 11, 11, 01
    BYTES 01, 10, 11, 10
    BYTES 11, 10, 10, 00
    BYTES 11, 10

global power_data_0:
    BYTES 0, 1, 1, 0
    BYTES 0, 1, 1, 1
    BYTES 1, 0, 0, 0
    BYTES 1, 0, 0, 1
    BYTES 1, 0, 1, 0
    BYTES 1, 1, 1, 1
    BYTES 0, 0, 1, 1
    BYTES 1, 0, 1, 0
    BYTES 1, 0, 0, 0
    BYTES 0, 0, 1, 1
    BYTES 0, 1, 0, 1
    BYTES 0, 0, 1, 0
    BYTES 0, 0, 1, 0
    BYTES 1, 1, 1, 0
    BYTES 1, 0, 1, 1
    BYTES 0, 0, 1, 0
    BYTES 0
