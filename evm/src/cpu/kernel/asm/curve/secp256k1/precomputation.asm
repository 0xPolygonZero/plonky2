// Initial stack: Gneg, Qneg, Qx, Qy, retdest
// Compute a*G ± b*phi(G) + c*Q ± d*phi(Q) for a,b,c,d in {0,1}^4 and store its x-coordinate at location `2*(8a+4b+2c+d)` and its y-coordinate at location `2*(8a+4b+2c+d)+1` in the SEGMENT_ECDSA_TABLE segment.
global secp_precompute_table:
    // First store G, ± phi(G), G ± phi(G)
    // Use Gneg for the ±, e.g., ±phi(G) is computed as `Gneg * (-phi(G)) + (1-Gneg)*phi(G)` (note only the y-coordinate needs to be filtered).
    // stack: Gneg, Qneg, Qx, Qy, retdest
    PUSH 32670510020758816978083085130507043184471273380659243275938904335757337482424 PUSH 17 PUSH 55066263022277343669578718895168534326250603453777594175500187360389116729240 PUSH 16
    %mstore_current(@SEGMENT_ECDSA_TABLE) %mstore_current(@SEGMENT_ECDSA_TABLE)

    DUP1 DUP1 %mul_const(32670510020758816978083085130507043184471273380659243275938904335757337482424) SWAP1 PUSH 1 SUB %mul_const(83121579216557378445487899878180864668798711284981320763518679672151497189239) ADD
    PUSH 9 PUSH 85340279321737800624759429340272274763154997815782306132637707972559913914315  PUSH 8
    %mstore_current(@SEGMENT_ECDSA_TABLE) %mstore_current(@SEGMENT_ECDSA_TABLE)

    DUP1 DUP1 %mul_const(83121579216557378445487899878180864668798711284981320763518679672151497189239) SWAP1 PUSH 1 SUB %mul_const(100652675408719987021357910538015346127426077519185866739835120963490438734674) ADD
    PUSH 25
    %mstore_current(@SEGMENT_ECDSA_TABLE)

    DUP1 %mul_const(91177636130617246552803821781935006617134368061721227770777272682868638699771) SWAP1 PUSH 1 SUB %mul_const(66837770201594535779099350687042404727408598709762866365333192677982385899440) ADD
    PUSH 24
    %mstore_current(@SEGMENT_ECDSA_TABLE)

    // Then store Q, ±phi(Q), Q ± phi(Q)
    %stack (Qneg, Qx, Qy, retdest) -> (4, Qx, 5, Qy, Qx, @SECP_BASE, Qneg, Qx, Qy, retdest)
    %mstore_current(@SEGMENT_ECDSA_TABLE) %mstore_current(@SEGMENT_ECDSA_TABLE)
    // stack: Qx, @SECP_BASE, Qx, Qy, retdest
    PUSH @SECP_GLV_BETA MULMOD
    %stack (betaQx, Qneg, Qx, Qy, retdest) -> (Qneg, Qy, Qneg, betaQx, Qx, Qy, retdest)
    MUL SWAP1 PUSH 1 SUB
    // stack: 1-Qneg, Qneg*Qy, betaQx, Qx, Qy, retdest
    DUP5 PUSH @SECP_BASE SUB MUL ADD
    %stack (selectQy, betaQx, Qx, Qy, retdest) -> (2, betaQx, 3, selectQy, betaQx, selectQy, Qx, Qy, precompute_table_contd, retdest)
    %mstore_current(@SEGMENT_ECDSA_TABLE) %mstore_current(@SEGMENT_ECDSA_TABLE)
    %jump(secp_add_valid_points_no_edge_case)
precompute_table_contd:
    %stack (x, y, retdest) -> (6, x, 7, y, retdest)
    %mstore_current(@SEGMENT_ECDSA_TABLE) %mstore_current(@SEGMENT_ECDSA_TABLE)
    PUSH 2
// Use a loop to store a*G ± b*phi(G) + c*Q ± d*phi(Q) for a,b,c,d in {0,1}^4.
precompute_table_loop:
    // stack: i, retdest
    DUP1 %increment %mload_current(@SEGMENT_ECDSA_TABLE)
    %stack (y, i, retdest) -> (i, y, i, retdest)
    %mload_current(@SEGMENT_ECDSA_TABLE)
    PUSH precompute_table_loop_contd
    DUP3 DUP3
    PUSH 9 %mload_current(@SEGMENT_ECDSA_TABLE)
    PUSH 8 %mload_current(@SEGMENT_ECDSA_TABLE)
    // stack: Gx, Gy, x, y, precompute_table_loop_contd, x, y, i, retdest
    %jump(secp_add_valid_points)
precompute_table_loop_contd:
    %stack (Rx, Ry, x, y, i, retdest) -> (i, 8, Rx, i, 9, Ry, x, y, i, retdest)
    ADD %mstore_current(@SEGMENT_ECDSA_TABLE) ADD %mstore_current(@SEGMENT_ECDSA_TABLE)
    DUP2 DUP2
    PUSH 17 %mload_current(@SEGMENT_ECDSA_TABLE)
    PUSH 16 %mload_current(@SEGMENT_ECDSA_TABLE)
    %stack (Gx, Gy, x, y, x, y, i, retdest) -> (Gx, Gy, x, y, precompute_table_loop_contd2, x, y, i, retdest)
    %jump(secp_add_valid_points)
precompute_table_loop_contd2:
    %stack (Rx, Ry, x, y, i, retdest) -> (i, 16, Rx, i, 17, Ry, x, y, i, retdest)
    ADD %mstore_current(@SEGMENT_ECDSA_TABLE) ADD %mstore_current(@SEGMENT_ECDSA_TABLE)
    PUSH 25 %mload_current(@SEGMENT_ECDSA_TABLE)
    PUSH 24 %mload_current(@SEGMENT_ECDSA_TABLE)
    %stack (Gx, Gy, x, y, i, retdest) -> (Gx, Gy, x, y, precompute_table_loop_contd3, i, retdest)
    %jump(secp_add_valid_points)
precompute_table_loop_contd3:
    %stack (Rx, Ry, i, retdest) -> (i, 24, Rx, i, 25, Ry, i, retdest)
    ADD %mstore_current(@SEGMENT_ECDSA_TABLE) ADD %mstore_current(@SEGMENT_ECDSA_TABLE)
    %add_const(2)
    DUP1 %eq_const(8) %jumpi(precompute_table_end)
    %jump(precompute_table_loop)

precompute_table_end:
    // stack: i, retdest
    POP JUMP
