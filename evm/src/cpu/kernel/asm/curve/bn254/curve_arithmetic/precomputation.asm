// Precompute a table of multiples of the BN254 point `Q = (Qx, Qy)`.
// Let `(Qxi, Qyi) = i * Q`, then store in the `SEGMENT_BN_TABLE_Q` segment of memory the values
// `i-1 => Qxi`, `i => Qyi if i < 16 else -Qy(32-i)` for `i in range(1, 32, 2)`.
global bn_precompute_table:
    // stack: Qx, Qy, retdest
    PUSH precompute_table_contd DUP3 DUP3
    %jump(bn_double)
precompute_table_contd:
    // stack: Qx2, Qy2, Qx, Qy, retdest
    PUSH 1
bn_precompute_table_loop:
    // stack i, Qx2, Qy2, Qx, Qy, retdest
    PUSH 1 DUP2 SUB
    %stack (im, i, Qx2, Qy2, Qx, Qy, retdest) -> (i, Qy, im, Qx, i, Qx2, Qy2, Qx, Qy, retdest)
    %mstore_current(@SEGMENT_BN_TABLE_Q) %mstore_current(@SEGMENT_BN_TABLE_Q)
    // stack: i, Qx2, Qy2, Qx, Qy, retdest
    DUP1 PUSH 32 SUB PUSH 1 DUP2 SUB
    // stack: 31-i, 32-i, i, Qx2, Qy2, Qx, Qy, retdest
    DUP7 PUSH @BN_BASE SUB
    // TODO: Could maybe avoid storing Qx a second time here, not sure if it would be more efficient.
    %stack (Qyy, iii, ii, i, Qx2, Qy2, Qx, Qy, retdest) -> (iii, Qx, ii, Qyy, i, Qx2, Qy2, Qx, Qy, retdest)
    %mstore_current(@SEGMENT_BN_TABLE_Q) %mstore_current(@SEGMENT_BN_TABLE_Q)
    // stack: i, Qx2, Qy2, Qx, Qy, retdest
    PUSH 2 ADD
    // stack: i+2, Qx2, Qy2, Qx, Qy, retdest
    DUP1 PUSH 16 LT %jumpi(precompute_table_end)
    %stack (i, Qx2, Qy2, Qx, Qy, retdest) -> (Qx, Qy, Qx2, Qy2, precompute_table_loop_contd, i, Qx2, Qy2, retdest)
    %jump(bn_add_valid_points)
precompute_table_loop_contd:
    %stack (Qx, Qy, i, Qx2, Qy2, retdest) -> (i, Qx2, Qy2, Qx, Qy, retdest)
    %jump(bn_precompute_table_loop)

precompute_table_end:
    // stack: i, Qx2, Qy2, Qx, Qy, retdest
    %pop5 JUMP
