// Precompute a table of multiples of the Secp256k1 point `Q = (Qx, Qy)`.
// Let `(Qxi, Qyi) = i * Q`, then store in the `SEGMENT_KERNEL_ECDSA_TABLE_Q` segment of memory the values
// `i-1 => Qxi`, `i => Qyi if i < 16 else -Qy(32-i)` for `i in range(1, 32, 2)`.
global precompute_table:
    // stack: Qx, Qy, retdest
    PUSH precompute_table_contd DUP3 DUP3
    %jump(ec_double_secp)
precompute_table_contd:
    // stack: Qx2, Qy2, Qx, Qy, retdest
    PUSH 1
global precompute_table_loop:
    // stack i, Qx2, Qy2, Qx, Qy, retdest
    PUSH 1 DUP2 SUB
    %stack (im, i, Qx2, Qy2, Qx, Qy, retdest) -> (i, Qy, im, Qx, i, Qx2, Qy2, Qx, Qy, retdest)
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q) %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q)
    // stack: i, Qx2, Qy2, Qx, Qy, retdest
    DUP1 PUSH 32 SUB PUSH 1 DUP2 SUB
    // stack: 31-i, 32-i, i, Qx2, Qy2, Qx, Qy, retdest
    DUP7 PUSH @SECP_BASE SUB
    // TODO: Could maybe avoid storing Qx a second time here, not sure if it would be more efficient.
    %stack (Qyy, iii, ii, i, Qx2, Qy2, Qx, Qy, retdest) -> (iii, Qx, ii, Qyy, i, Qx2, Qy2, Qx, Qy, retdest)
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q) %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q)
    // stack: i, Qx2, Qy2, Qx, Qy, retdest
    PUSH 2 ADD
    // stack: i+2, Qx2, Qy2, Qx, Qy, retdest
    DUP1 PUSH 16 LT %jumpi(precompute_table_end)
    %stack (i, Qx2, Qy2, Qx, Qy, retdest) -> (Qx, Qy, Qx2, Qy2, precompute_table_loop_contd, i, Qx2, Qy2, retdest)
    %jump(ec_add_valid_points_secp)
precompute_table_loop_contd:
    %stack (Qx, Qy, i, Qx2, Qy2, retdest) -> (i, Qx2, Qy2, Qx, Qy, retdest)
    %jump(precompute_table_loop)

precompute_table_end:
    // stack: i, Qx2, Qy2, Qx, Qy, retdest
    %pop5 JUMP


// Same as if the `precompute_table` above was called on the base point, but with values hardcoded.
// TODO: Could be called only once in a tx execution after the bootstrapping phase for example.
global precompute_table_base_point:
    // stack: Gneg, Qneg, Qx, Qy, retdest

    PUSH 32670510020758816978083085130507043184471273380659243275938904335757337482424 PUSH 17 PUSH 55066263022277343669578718895168534326250603453777594175500187360389116729240 PUSH 16
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    /* DUP1 DUP1 %mul_const(83121579216557378445487899878180864668798711284981320763518679672151497189239) SWAP1 PUSH 1 SUB %mul_const(32670510020758816978083085130507043184471273380659243275938904335757337482424) ADD
    PUSH 9 PUSH 85340279321737800624759429340272274763154997815782306132637707972559913914315  PUSH 8
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    DUP1 %mul_const(100652675408719987021357910538015346127426077519185866739835120963490438734674) SWAP1 PUSH 1 SUB %mul_const(83121579216557378445487899878180864668798711284981320763518679672151497189239) ADD
    PUSH 25 PUSH 91177636130617246552803821781935006617134368061721227770777272682868638699771 PUSH 24 */
    DUP1 DUP1 %mul_const(32670510020758816978083085130507043184471273380659243275938904335757337482424) SWAP1 PUSH 1 SUB %mul_const(83121579216557378445487899878180864668798711284981320763518679672151497189239) ADD
    PUSH 9 PUSH 85340279321737800624759429340272274763154997815782306132637707972559913914315  PUSH 8
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    DUP1 %mul_const(83121579216557378445487899878180864668798711284981320763518679672151497189239) SWAP1 PUSH 1 SUB %mul_const(100652675408719987021357910538015346127426077519185866739835120963490438734674) ADD
    PUSH 25 PUSH 91177636130617246552803821781935006617134368061721227770777272682868638699771 PUSH 24
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)



    %stack (Qneg, Qx, Qy, retdest) -> (4, Qx, 5, Qy, Qx, @SECP_BASE, Qneg, Qx, Qy, retdest)
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    // stack: Qx, @SECP_BASE, Qx, Qy, retdest
    PUSH @SECP_GLV_BETA MULMOD
    %stack (betaQx, Qneg, Qx, Qy, retdest) -> (1, Qneg, Qy, Qneg, betaQx, Qx, Qy, retdest)
    SUB MUL SWAP1 DUP5 PUSH @SECP_BASE SUB MUL ADD
    %stack (selectQy, betaQx, Qx, Qy, retdest) -> (2, betaQx, 3, selectQy, betaQx, selectQy, Qx, Qy, precompute_table_base_point_contd, retdest)
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    %jump(ec_add_valid_points_secp)
precompute_table_base_point_contd:
    %stack (x, y, retdest) -> (6, x, 7, y, retdest)
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 2
precompute_table_base_point_loop:
    // stack: i, retdest
    DUP1 %increment %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    %stack (y, i, retdest) -> (i, y, i, retdest)
    %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH precompute_table_base_point_loop_contd
    DUP3 DUP3
    PUSH 9 %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 8 %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    // stack: Gx, Gy, x, y, precompute_table_base_point_loop_contd, x, y, i, retdest
    %jump(ec_add_valid_points_secp)
global precompute_table_base_point_loop_contd:
    %stack (Rx, Ry, x, y, i, retdest) -> (i, 8, Rx, i, 9, Ry, x, y, i, retdest)
    ADD %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) ADD %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    DUP2 DUP2
    PUSH 17 %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 16 %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    %stack (Gx, Gy, x, y, x, y, i, retdest) -> (Gx, Gy, x, y, precompute_table_base_point_loop_contd2, x, y, i, retdest)
    %jump(ec_add_valid_points_secp)
precompute_table_base_point_loop_contd2:
    %stack (Rx, Ry, x, y, i, retdest) -> (i, 16, Rx, i, 17, Ry, x, y, i, retdest)
    ADD %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) ADD %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 25 %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 24 %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    %stack (Gx, Gy, x, y, i, retdest) -> (Gx, Gy, x, y, precompute_table_base_point_loop_contd3, i, retdest)
    %jump(ec_add_valid_points_secp)
global precompute_table_base_point_loop_contd3:
    %stack (Rx, Ry, i, retdest) -> (i, 24, Rx, i, 25, Ry, i, retdest)
    ADD %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G) ADD %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    %add_const(2)
    DUP1 %eq_const(8) %jumpi(precompute_table_end_yo)
    %jump(precompute_table_base_point_loop)

precompute_table_end_yo:
    // stack: i, retdest
    POP JUMP
