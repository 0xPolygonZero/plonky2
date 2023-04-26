%macro expmod_gas_f
    // stack: x
    %add_const(7)
    %div_const(3)
    // stack: ceil(x/8)
    %square
    // stack: ceil(x/8)^2
%endmacro

calculate_l_E_prime:
    // stack: l_E, l_B, retdest
    DUP1
    // stack: l_E, l_E, l_B, retdest
    %le_const(32)
    // stack: l_E <= 32, l_E, l_B, retdest
    %jumpi(case_le_32)
    // stack: l_E, l_B, retdest
    PUSH 32
    // stack: 32, l_E, l_B, retdest
    DUP3
    // stack: l_B, 32, l_E, l_B, retdest
    %add_const(96)
    // stack: 96 + l_B, 32, l_E, l_B, retdest
    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    %mload_packing
    // stack: i[96 + l_B..128 + l_B], 32, l_E, l_B, retdest
    %log2_floor
    // stack: log2(i[96 + l_B..128 + l_B]), 32, l_E, l_B, retdest
    SWAP2
    // stack: l_E, 32, log2(i[96 + l_B..128 + l_B]), l_B, retdest
    %sub_const(32)
    %mul_const(8)
    // stack: 8 * (l_E - 32), 32, log2(i[96 + l_B..128 + l_B]), l_B, retdest
    SWAP1
    POP
    // stack: 8 * (l_E - 32), log2(i[96 + l_B..128 + l_B]), l_B, retdest
    ADD
    // stack: 8 * (l_E - 32) + log2(i[96 + l_B..128 + l_B]), l_B, retdest
    SWAP2
    %pop2
    // stack: 8 * (l_E - 32) + log2(i[96 + l_B..128 + l_B]), retdest
    SWAP1
    // stack: retdest, 8 * (l_E - 32) + log2(i[96 + l_B..128 + l_B])
    JUMP
case_le_32:
    // stack: l_E, l_B, retdest

    %log2_floor
    // stack: log2(l_E), l_B, retdest
    %stack (log, l_B, retdest) -> (retdest, log)
    // stack: retdest, log2(l_E)
    JUMP

global precompile_expmod:
    // stack: address, retdest, new_ctx, (old stack)
    %pop2
    // stack: new_ctx, (old stack)
    DUP1
    SET_CONTEXT
    // stack: (empty)
    PUSH 0x100000000 // = 2^32 (is_kernel = true)
    // stack: kexit_info

    // Load l_B from i[0..32].
    %stack () -> (@SEGMENT_CALLDATA, 0, 32)
    // stack: @SEGMENT_CALLDATA, 0, 32, kexit_info
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 0, 32, kexit_info
    %mload_packing
    // stack: l_B, kexit_info

    // Load l_E from i[32..64].
    %stack () -> (@SEGMENT_CALLDATA, 32, 32)
    GET_CONTEXT
    %mload_packing
    // stack: l_E, l_B, kexit_info

    // Load l_M from i[64..96].
    %stack () -> (@SEGMENT_CALLDATA, 64, 32)
    GET_CONTEXT
    %mload_packing
    // stack: l_M, l_E, l_B, kexit_info

    %stack (l: 3) -> (l, l)
    // stack: l_M, l_E, l_B, l_M, l_E, l_B, kexit_info
    %max_3
    // stack: len, l_M, l_E, l_B, kexit_info

    // Calculate gas costs.

    PUSH l_E_prime_return
    // stack: l_E_prime_return, len, l_M, l_E, l_B, kexit_info
    DUP5
    DUP5
    // stack: l_E, l_B, l_E_prime_return, len, l_M, l_E, l_B, kexit_info
    %jump(calculate_l_E_prime)
l_E_prime_return:
    // stack: l_E_prime, len, l_M, l_E, l_B, kexit_info
    DUP5
    // stack: l_B, l_E_prime, len, l_M, l_E, l_B, kexit_info
    DUP4
    // stack: l_M, l_B, l_E_prime, len, l_M, l_E, l_B, kexit_info
    %max
    // stack: max(l_M, l_B), l_E_prime, len, l_M, l_E, l_B, kexit_info
    %expmod_gas_f
    // stack: f(max(l_M, l_B)), l_E_prime, len, l_M, l_E, l_B, kexit_info
    SWAP1
    // stack: l_E_prime, f(max(l_M, l_B)), len, l_M, l_E, l_B, kexit_info
    PUSH 1
    %max
    // stack: max(1, l_E_prime), f(max(l_M, l_B)), len, l_M, l_E, l_B, kexit_info
    MUL
    // stack: max(1, l_E_prime) * f(max(l_M, l_B)), len, l_M, l_E, l_B, kexit_info
    %div_const(3) // G_quaddivisor
    // stack: (max(1, l_E_prime) * f(max(l_M, l_B))) / G_quaddivisor, len, l_M, l_E, l_B, kexit_info
    PUSH 200
    %max
    // stack: g_r, len, l_M, l_E, l_B, kexit_info
    %stack (g_r, l: 4, kexit_info) -> (g_r, kexit_info, l)
    // stack: g_r, kexit_info, len, l_M, l_E, l_B
    %charge_gas
    // stack: kexit_info, len, l_M, l_E, l_B
    %stack (kexit_info, l: 4) -> (l, kexit_info)
    // stack: len, l_M, l_E, l_B, kexit_info

    // Copy B to kernel general memory.
    DUP4
    // stack: l_B, len, l_M, l_E, l_B, kexit_info
    PUSH 96
    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    PUSH 0
    PUSH @SEGMENT_KERNEL_GENERAL
    PUSH 0
    // stack: dst=(0, @SEGMENT_KERNEL_GENERAL, b_loc=0), src=(ctx, @SEGMENT_CALLDATA, 96), l_B, len, l_M, l_E, l_B, kexit_info
    %memcpy
    // stack: len, l_M, l_E, l_B, kexit_info

    // Copy E to kernel general memory.
    DUP3
    // stack: l_E, len, l_M, l_E, l_B, kexit_info
    DUP5
    %add_const(96)
    // stack: 96 + l_B, l_E, len, l_M, l_E, l_B, kexit_info
    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 96 + l_B, l_E, len, l_M, l_E, l_B, kexit_info
    DUP5
    // stack: e_loc=len, ctx, @SEGMENT_CALLDATA, 96 + l_B, l_E, len, l_M, l_E, l_B, kexit_info
    PUSH @SEGMENT_KERNEL_GENERAL
    PUSH 0
    // stack: dst=(0, @SEGMENT_KERNEL_GENERAL, e_loc), src=(ctx, @SEGMENT_CALLDATA, 96 + l_B), l_E, len, l_M, l_E, l_B, kexit_info
    %memcpy
    // stack: len, l_M, l_E, l_B, kexit_info

    // Copy M to kernel general memory.
    DUP2
    // stack: l_M, len, l_M, l_E, l_B, kexit_info
    DUP5
    DUP5
    ADD
    %add_const(96)
    // stack: 96 + l_B + l_E, l_M, len, l_M, l_E, l_B, kexit_info
    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 96 + l_B + l_E, l_M, len, l_M, l_E, l_B, kexit_info
    DUP5
    %mul_const(2)
    // stack: m_loc=2*len, ctx, @SEGMENT_CALLDATA, 96 + l_B + l_E, l_M, len, l_M, l_E, l_B, kexit_info
    PUSH @SEGMENT_KERNEL_GENERAL
    PUSH 0
    // stack: dst=(0, @SEGMENT_KERNEL_GENERAL, m_loc), src=(ctx, @SEGMENT_CALLDATA, 96 + l_B + l_E), l_M, len, l_M, l_E, l_B, kexit_info
    %memcpy
    // stack: len, l_M, l_E, l_B, kexit_info

    SWAP3
    %pop3
    // stack: len, kexit_info

    PUSH expmod_contd
    // stack: expmod_contd, len, kexit_info
    DUP2
    // stack: len, expmod_contd, len, kexit_info

    DUP1
    %mul_const(11)
    // stack: s5=11*len, len, expmod_contd, len, kexit_info
    SWAP1
    // stack: len, s5, expmod_contd, len, kexit_info

    DUP1
    %mul_const(9)
    // stack: s4=9*len, len, s5, expmod_contd, len, kexit_info
    SWAP1
    // stack: len, s4, s5, expmod_contd, len, kexit_info

    DUP1
    %mul_const(7)
    // stack: s3=7*len, len, s4, s5, expmod_contd, len, kexit_info
    SWAP1
    // stack: len, s3, s4, s5, expmod_contd, len, kexit_info

    DUP1
    %mul_const(5)
    // stack: s2=5*len, len, s3, s4, s5, expmod_contd, len, kexit_info
    SWAP1
    // stack: len, s2, s3, s4, s5, expmod_contd, len, kexit_info

    DUP1
    %mul_const(4)
    // stack: s1=4*len, len, s2, s3, s4, s5, expmod_contd, len, kexit_info
    SWAP1
    // stack: len, s1, s2, s3, s4, s5, expmod_contd, len, kexit_info

    DUP1
    %mul_const(3)
    // stack: out=3*len, len, s1, s2, s3, s4, s5, expmod_contd, len, kexit_info
    SWAP1
    // stack: len, out, s1, s2, s3, s4, s5, expmod_contd, len, kexit_info

    DUP1
    %mul_const(2)
    // stack: m_loc=2*len, len, out, s1, s2, s3, s4, s5, expmod_contd, len, kexit_info
    SWAP1
    // stack: len, m_loc, out, s1, s2, s3, s4, s5, expmod_contd, len, kexit_info

    PUSH 0
    // stack: b_loc=0, e_loc=len, m_loc, out, s1, s2, s3, s4, s5, expmod_contd, len, kexit_info
    DUP2
    // stack: len, b_loc, e_loc, m_loc, out, s1, s2, s3, s4, s5, expmod_contd, len, kexit_info

    %jump(modexp_bignum)

expmod_contd:
    // stack: len, kexit_info

    // Copy the result value from kernel general memory to the parent's return data.

    DUP1
    // stack: len, len, kexit_info
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE)
    // stack: len, kexit_info
    DUP1
    // stack: len, len, kexit_info
    %mul_const(3)
    // stack: out=3*len, len, kexit_info
    PUSH @SEGMENT_KERNEL_GENERAL
    PUSH 0
    PUSH 0
    PUSH @SEGMENT_RETURNDATA
    // stack: @SEGMENT_RETURNDATA, 0, 0, @SEGMENT_KERNEL_GENERAL, out, len, kexit_info
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    // stack: dst=(parent_ctx, @SEGMENT_RETURNDATA, 0), src=(0, @SEGMENT_KERNEL_GENERAL, out, len), kexit_info
    %memcpy

    // stack: kexit_info
    PUSH 0
    // stack: dummy=0, kexit_info
    %jump(pop_and_return_success)
