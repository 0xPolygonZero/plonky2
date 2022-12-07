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
    DUP7
    PUSH 115792089237316195423570985008687907853269984665640564039457584007908834671663
    SUB
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


global precompute_table_base_point:
    // stack: retdest
    PUSH 0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798 PUSH 0 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8 PUSH 1 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xf9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9 PUSH 2 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x388f7b0f632de8140fe337e62a37f3566500a99934c2231b6cb9fd7584b8e672 PUSH 3 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x2f8bde4d1a07209355b4a7250a5c5128e88b84bddc619ab7cba8d569b240efe4 PUSH 4 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xd8ac222636e5e3d6d4dba9dda6c9c426f788271bab0d6840dca87d3aa6ac62d6 PUSH 5 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x5cbdf0646e5db4eaa398f365f2ea7a0e3d419b7e0330e39ce92bddedcac4f9bc PUSH 6 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x6aebca40ba255960a3178d6d861a54dba813d0b813fde7b5a5082628087264da PUSH 7 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xacd484e2f0c7f65309ad178a9f559abde09796974c57e714c35f110dfc27ccbe PUSH 8 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xcc338921b0a7d9fd64380971763b61e9add888a4375f8e0f05cc262ac64f9c37 PUSH 9 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x774ae7f858a9411e5ef4246b70c65aac5649980be5c17891bbec17895da008cb PUSH 10 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xd984a032eb6b5e190243dd56d7b7b365372db1e2dff9d6a8301d74c9c953c61b PUSH 11 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xf28773c2d975288bc7d1d205c3748651b075fbc6610e58cddeeddf8f19405aa8 PUSH 12 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xab0902e8d880a89758212eb65cdaf473a1a06da521fa91f29b5cb52db03ed81  PUSH 13 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xd7924d4f7d43ea965a465ae3095ff41131e5946f3c85f79e44adbcf8e27e080e PUSH 14 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x581e2872a86c72a683842ec228cc6defea40af2bd896d3a5c504dc9ff6a26b58 PUSH 15 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xd7924d4f7d43ea965a465ae3095ff41131e5946f3c85f79e44adbcf8e27e080e PUSH 16 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xa7e1d78d57938d597c7bd13dd733921015bf50d427692c5a3afb235f095d90d7 PUSH 17 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xf28773c2d975288bc7d1d205c3748651b075fbc6610e58cddeeddf8f19405aa8 PUSH 18 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xf54f6fd17277f5768a7ded149a3250b8c5e5f925ade056e0d64a34ac24fc0eae PUSH 19 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x774ae7f858a9411e5ef4246b70c65aac5649980be5c17891bbec17895da008cb PUSH 20 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x267b5fcd1494a1e6fdbc22a928484c9ac8d24e1d20062957cfe28b3536ac3614 PUSH 21 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xacd484e2f0c7f65309ad178a9f559abde09796974c57e714c35f110dfc27ccbe PUSH 22 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x33cc76de4f5826029bc7f68e89c49e165227775bc8a071f0fa33d9d439b05ff8 PUSH 23 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x5cbdf0646e5db4eaa398f365f2ea7a0e3d419b7e0330e39ce92bddedcac4f9bc PUSH 24 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x951435bf45daa69f5ce8729279e5ab2457ec2f47ec02184a5af7d9d6f78d9755 PUSH 25 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x2f8bde4d1a07209355b4a7250a5c5128e88b84bddc619ab7cba8d569b240efe4 PUSH 26 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x2753ddd9c91a1c292b24562259363bd90877d8e454f297bf235782c459539959 PUSH 27 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xf9308a019258c31049344f85f89d5229b531c845836f99b08601f113bce036f9 PUSH 28 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xc77084f09cd217ebf01cc819d5c80ca99aff5666cb3ddce4934602897b4715bd PUSH 29 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798 PUSH 30 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 0xb7c52588d95c3b9aa25b0403f1eef75702e84bb7597aabe663b82f6f04ef2777 PUSH 31 %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    JUMP

