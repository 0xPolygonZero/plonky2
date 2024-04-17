// Mod 16 to the range [1, 16].
%macro mod_16
    // stack: x
    %mod_const(16)
    DUP1 %jumpi(%%after)
    POP PUSH 16
%%after:
%endmacro

// Load bytes, packing 16 bytes into each limb, and store limbs on the stack.
// We pass around total_num_limbs and len for conveience, because we can't access them from the stack
// if they're hidden behind the variable number of limbs.
mload_bytes_as_limbs:
    // stack: addr, num_bytes, retdest, total_num_limbs, len, ..limbs
    DUP2
    // stack: num_bytes, addr, num_bytes, retdest, total_num_limbs, len, ..limbs
    %mod_16
    // stack: min(16, num_bytes), addr, num_bytes, retdest, total_num_limbs, len, ..limbs
    DUP2
    // stack: addr, min(16, num_bytes), addr, num_bytes, retdest, total_num_limbs, len, ..limbs
    %mload_packing
    // stack: new_limb, addr, num_bytes, retdest, total_num_limbs, len, ..limbs
    %stack (new, addr, numb, ret, tot, len) -> (numb, addr, ret, tot, len, new)
    // stack: num_bytes, addr, retdest, total_num_limbs, len, new_limb, ..limbs
    DUP1
    %mod_16
    // stack: num_bytes%16, num_bytes, addr, retdest, total_num_limbs, len, new_limb, ..limbs
    DUP1 SWAP2
    SUB
    // stack: num_bytes_new, num_bytes%16, addr, retdest, total_num_limbs, len, new_limb, ..limbs
    DUP1
    ISZERO
    %jumpi(mload_bytes_return)
    SWAP1
    // stack: num_bytes%16, num_bytes_new, addr, retdest, total_num_limbs, len, new_limb, ..limbs
    DUP3 // addr
    ADD // increment offset
    // stack: addr_new, num_bytes_new, addr, retdest, total_num_limbs, len, new_limb, ..limbs
    SWAP2 POP
    // stack: num_bytes_new, addr_new, retdest, total_num_limbs, len, new_limb, ..limbs
    SWAP1
    %jump(mload_bytes_as_limbs)
mload_bytes_return:
    // stack: num_bytes_new, num_bytes%16, addr, retdest, total_num_limbs, len, new_limb, ..limbs
    %pop3
    // stack: retdest, total_num_limbs, len, ..limbs
    JUMP

%macro mload_bytes_as_limbs
    %stack (addr, num_bytes, total_num_limbs) -> (addr, num_bytes, %%after, total_num_limbs)
    %jump(mload_bytes_as_limbs)
%%after:
%endmacro

store_limbs:
    // stack: offset, retdest, num_limbs, limb[num_limbs - 1], ..limb[0]
    DUP3
    // stack: num_limbs, offset, retdest, num_limbs, limb[num_limbs - 1], ..limb[0]
    ISZERO
    %jumpi(store_limbs_return)
    // stack: offset, retdest, num_limbs, limb[num_limbs - 1], ..limb[0]
    %stack (offset, ret, num, limb) -> (offset, limb, offset, ret, num)
    // stack: offset, limb[num_limbs - 1], offset, retdest, num_limbs, limb[num_limbs - 2], ..limb[0]
    %mstore_current_general
    // stack: offset, retdest, num_limbs, limb[num_limbs - 2], ..limb[0]
    %increment
    SWAP2
    %decrement
    SWAP2
    // stack: offset + 1, retdest, num_limbs - 1, limb[num_limbs - 2], ..limb[0]
    %jump(store_limbs)
store_limbs_return:
    // stack: offset, retdest, num_limbs=0
    POP
    SWAP1
    POP
    JUMP

%macro store_limbs
    %stack (offset, num_limbs) -> (offset, %%after, num_limbs)
    %jump(store_limbs)
%%after:
%endmacro

%macro expmod_gas_f
    // stack: x
    // Overflow check
    DUP1 %ge_const(0x800000000000000000000000000000007) %jumpi(fault_exception)
    // stack: x
    %ceil_div_const(8)
    // stack: ceil(x/8)
    %square
    // stack: ceil(x/8)^2
%endmacro

calculate_l_E_prime:
    // stack: l_E, l_B, retdest
    // Throw a fault early if the lengths are too large.
    DUP2 %gt_const(0x100000000000000000000000000000000) %jumpi(fault_exception)
    DUP1 %gt_const(0x100000000000000000000000000000000) %jumpi(fault_exception)
    DUP1 ISZERO %jumpi(case_le_zero)
    // stack: l_E, l_B, retdest
    DUP1 %le_const(32)
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
    %build_address
    %mload_packing
    // stack: i[96 + l_B..128 + l_B], l_E, l_B, retdest
    %log2_floor
    // stack: log2(i[96 + l_B..128 + l_B]), l_E, l_B, retdest
    SWAP1
    // stack: l_E, log2(i[96 + l_B..128 + l_B]), l_B, retdest
    %sub_const(32)
    // Overflow check
    DUP1 %ge_const(0x2000000000000000000000000000000000000000000000000000000000000000) %jumpi(fault_exception)
    %mul_const(8)
    // stack: 8 * (l_E - 32), log2(i[96 + l_B..128 + l_B]), l_B, retdest
    ADD
    // stack: 8 * (l_E - 32) + log2(i[96 + l_B..128 + l_B]), l_B, retdest
    SWAP1
    POP
    // stack: 8 * (l_E - 32) + log2(i[96 + l_B..128 + l_B]), retdest
    SWAP1
    // stack: retdest, 8 * (l_E - 32) + log2(i[96 + l_B..128 + l_B])
    JUMP
case_le_zero:
    %stack (l_E, l_B, retdest) -> (retdest, 0)
    JUMP
case_le_32:
    // stack: l_E, l_B, retdest
    SWAP1
    // stack: l_B, l_E, retdest
    %add_const(96)
    // stack: 96 + l_B, l_E, retdest
    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    %build_address
    %mload_packing
    // stack: E, retdest
    %log2_floor
    // stack: log2(E), retdest
    SWAP1
    // stack: retdest, log2(E)
    JUMP

global precompile_expmod:
    // stack: address, retdest, new_ctx, (old stack)
    %pop2
    // stack: new_ctx, (old stack)
    %set_new_ctx_parent_pc(after_precompile)
    // stack: new_ctx, (old stack)
    DUP1
    SET_CONTEXT
    %checkpoint // Checkpoint
    %increment_call_depth
    // stack: (empty)
    PUSH 0x100000000 // = 2^32 (is_kernel = true)
    // stack: kexit_info

    // Load l_B from i[0..32].
    %stack () -> (@SEGMENT_CALLDATA, 32)
    // stack: @SEGMENT_CALLDATA, 32, kexit_info
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 32, kexit_info
    %build_address_no_offset
    %mload_packing
    // stack: l_B, kexit_info

    // Load l_E from i[32..64].
    %stack () -> (@SEGMENT_CALLDATA, 32, 32)
    GET_CONTEXT
    %build_address
    %mload_packing
    // stack: l_E, l_B, kexit_info

    // Load l_M from i[64..96].
    %stack () -> (@SEGMENT_CALLDATA, 64, 32)
    GET_CONTEXT
    %build_address
    %mload_packing
    // stack: l_M, l_E, l_B, kexit_info
    DUP3 ISZERO DUP2 ISZERO
    MUL // AND
    // stack: l_M==0 && l_B==0, l_M, l_E, l_B, kexit_info
    %jumpi(zero_base_zero_mod)
    %stack (l: 3) -> (l, l)
    // stack: l_M, l_E, l_B, l_M, l_E, l_B, kexit_info
    %max_3
    // stack: max_len, l_M, l_E, l_B, kexit_info
    
    %ceil_div_const(16)
    // stack: len=ceil(max_len/16), l_M, l_E, l_B, kexit_info

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
    %max_const(1)
    // stack: max(1, l_E_prime), f(max(l_M, l_B)), len, l_M, l_E, l_B, kexit_info
    MUL
    // stack: max(1, l_E_prime) * f(max(l_M, l_B)), len, l_M, l_E, l_B, kexit_info
    %div_const(3) // G_quaddivisor
    // stack: (max(1, l_E_prime) * f(max(l_M, l_B))) / G_quaddivisor, len, l_M, l_E, l_B, kexit_info
    %max_const(200)
    // stack: g_r, len, l_M, l_E, l_B, kexit_info
    %stack (g_r, l: 4, kexit_info) -> (g_r, kexit_info, l)
    // stack: g_r, kexit_info, len, l_M, l_E, l_B
    %charge_gas
    // stack: kexit_info, len, l_M, l_E, l_B
    %stack (kexit_info, l: 4) -> (l, kexit_info)
    // stack: len, l_M, l_E, l_B, kexit_info

    // Copy B to memory.
    // stack: len, l_M, l_E, l_B, kexit_info
    DUP1
    // stack: len, len, l_M, l_E, l_B, kexit_info
    DUP5
    // stack: num_bytes=l_B, len, len, l_M, l_E, l_B, kexit_info
    DUP1
    %ceil_div_const(16)
    // stack: num_limbs, num_bytes, len, len, l_M, l_E, l_B, kexit_info
    DUP2
    ISZERO
    %jumpi(copy_b_len_zero)
    SWAP1
    // stack: num_bytes, num_limbs, len, len, l_M, l_E, l_B, kexit_info
    %stack () -> (@SEGMENT_CALLDATA, 96)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 96, num_bytes, num_limbs, len, len, l_M, l_E, l_B, kexit_info
    %build_address
    %mload_bytes_as_limbs
    // stack: num_limbs, len, limbs[num_limbs-1], .., limbs[0], len, l_M, l_E, l_B, kexit_info
    SWAP1
    POP
    // stack: num_limbs, limbs[num_limbs-1], .., limbs[0], len, l_M, l_E, l_B, kexit_info
    PUSH 0
    // stack: b_loc=0, num_limbs, limbs[num_limbs-1], .., limbs[0], len, l_M, l_E, l_B, kexit_info
    %store_limbs
    // stack: len, l_M, l_E, l_B, kexit_info
    %jump(copy_b_end)
copy_b_len_zero:
    // stack: num_limbs, num_bytes, len, len, l_M, l_E, l_B, kexit_info
    %pop3
copy_b_end:
    
    // Copy E to memory.
    // stack: len, l_M, l_E, l_B, kexit_info
    DUP1
    // stack: len, len, l_M, l_E, l_B, kexit_info
    DUP4
    // stack: num_bytes=l_E, len, len, l_M, l_E, l_B, kexit_info
    DUP1
    %ceil_div_const(16)
    // stack: num_limbs, num_bytes, len, len, l_M, l_E, l_B, kexit_info
    DUP2
    ISZERO
    %jumpi(copy_e_len_zero)
    SWAP1
    // stack: num_bytes, num_limbs, len, len, l_M, l_E, l_B, kexit_info
    DUP7
    %add_const(96)
    // stack: 96 + l_B, num_bytes, num_limbs, len, len, l_M, l_E, l_B, kexit_info
    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 96 + l_B, num_bytes, num_limbs, len, len, l_M, l_E, l_B, kexit_info
    %build_address
    %mload_bytes_as_limbs
    // stack: num_limbs, len, limbs[num_limbs-1], .., limbs[0], len, l_M, l_E, l_B, kexit_info
    SWAP1
    // stack: e_loc=len, num_limbs, limbs[num_limbs-1], .., limbs[0], len, l_M, l_E, l_B, kexit_info
    %store_limbs
    // stack: len, l_M, l_E, l_B, kexit_info
    %jump(copy_e_end)
copy_e_len_zero:
    // stack: num_limbs, num_bytes, len, len, l_M, l_E, l_B, kexit_info
    %pop3
copy_e_end:

    // Copy M to memory.
    // stack: len, l_M, l_E, l_B, kexit_info
    DUP1
    // stack: len, len, l_M, l_E, l_B, kexit_info
    DUP3
    // stack: num_bytes=l_M, len, len, l_M, l_E, l_B, kexit_info
    DUP1
    %ceil_div_const(16)
    // stack: num_limbs, num_bytes, len, len, l_M, l_E, l_B, kexit_info
    DUP2
    ISZERO
    %jumpi(copy_m_len_zero)
    SWAP1
    // stack: num_bytes, num_limbs, len, len, l_M, l_E, l_B, kexit_info
    DUP7
    DUP7
    ADD
    %add_const(96)
    // stack: 96 + l_B + l_E, num_bytes, num_limbs, len, len, l_M, l_E, l_B, kexit_info
    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 96 + l_B + l_E, num_bytes, num_limbs, len, len, l_M, l_E, l_B, kexit_info
    %build_address
    %mload_bytes_as_limbs
    // stack: num_limbs, len, limbs[num_limbs-1], .., limbs[0], len, l_M, l_E, l_B, kexit_info
    SWAP1
    %mul_const(2)
    // stack: m_loc=2*len, num_limbs, limbs[num_limbs-1], .., limbs[0], len, l_M, l_E, l_B, kexit_info
    %store_limbs
    // stack: len, l_M, l_E, l_B, kexit_info
    %jump(copy_m_end)
copy_m_len_zero:
    // stack: num_limbs, num_bytes, len, len, l_M, l_E, l_B, kexit_info
    %pop3
copy_m_end:

    %stack (len, l_M, ls: 2) -> (len, l_M)
    // stack: len, l_M, kexit_info

    PUSH expmod_contd
    // stack: expmod_contd, len, l_M, kexit_info
    DUP2
    // stack: len, expmod_contd, len, l_M, kexit_info

    DUP1
    %mul_const(11)
    // stack: s5=11*len, len, expmod_contd, len, l_M, kexit_info
    SWAP1
    // stack: len, s5, expmod_contd, len, l_M, kexit_info

    DUP1
    %mul_const(9)
    // stack: s4=9*len, len, s5, expmod_contd, len, l_M, kexit_info
    SWAP1
    // stack: len, s4, s5, expmod_contd, len, l_M, kexit_info

    DUP1
    %mul_const(7)
    // stack: s3=7*len, len, s4, s5, expmod_contd, len, l_M, kexit_info
    SWAP1
    // stack: len, s3, s4, s5, expmod_contd, len, l_M, kexit_info

    DUP1
    %mul_const(5)
    // stack: s2=5*len, len, s3, s4, s5, expmod_contd, len, l_M, kexit_info
    SWAP1
    // stack: len, s2, s3, s4, s5, expmod_contd, len, l_M, kexit_info

    DUP1
    %mul_const(4)
    // stack: s1=4*len, len, s2, s3, s4, s5, expmod_contd, len, l_M, kexit_info
    SWAP1
    // stack: len, s1, s2, s3, s4, s5, expmod_contd, len, l_M, kexit_info

    DUP1
    %mul_const(3)
    // stack: out=3*len, len, s1, s2, s3, s4, s5, expmod_contd, len, l_M, kexit_info
    SWAP1
    // stack: len, out, s1, s2, s3, s4, s5, expmod_contd, len, l_M, kexit_info

    DUP1
    %mul_const(2)
    // stack: m_loc=2*len, len, out, s1, s2, s3, s4, s5, expmod_contd, len, l_M, kexit_info
    SWAP1
    // stack: len, m_loc, out, s1, s2, s3, s4, s5, expmod_contd, len, l_M, kexit_info

    PUSH 0
    // stack: b_loc=0, e_loc=len, m_loc, out, s1, s2, s3, s4, s5, expmod_contd, len, l_M, kexit_info
    DUP2
    // stack: len, b_loc, e_loc, m_loc, out, s1, s2, s3, s4, s5, expmod_contd, len, l_M, kexit_info

    %jump(modexp_bignum)

expmod_contd:
    // stack: len, l_M, kexit_info

    // Copy the result value from memory to the parent's return data.

    // Store return data size: l_M (number of bytes).
    SWAP1
    // stack: l_M, len, kexit_info
    DUP1 %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE)
    // stack: l_M, len, kexit_info
    DUP1 ISZERO %jumpi(zero_modulus)
    // stack: l_M, len, kexit_info
    DUP1 %ceil_div_const(16)
    // stack: l_M_128, l_M, len, kexit_info
    SWAP1 %mod_16
    // stack: l_M%16, l_M_128, len, kexit_info
    SWAP2
    // stack: len, l_M_128, l_M%16, kexit_info
    %mul_const(3)
    // stack: out=3*len, l_M_128, l_M%16, kexit_info
    %decrement
    DUP2
    DUP2
    ADD
    // stack: cur_offset=out+l_M_128-1, end_offset=out-1, l_M_128, l_M%16, kexit_info
    DUP1 %mload_current_general
    %stack (cur_limb, cur_offset, end_offset, l_M_128, l_M_mod16, kexit_info) ->
        (@SEGMENT_RETURNDATA, cur_limb, l_M_mod16, cur_offset, end_offset, l_M_128, kexit_info)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %build_address_no_offset
    %mstore_unpacking
    // stack: address, cur_offset, end_offset, l_M_128, kexit_info
    SWAP1
    %decrement
    // stack: cur_offset, address, end_offset, l_M_128, kexit_info
    // Store in big-endian format.
expmod_store_loop:
    // stack: cur_offset, address, end_offset, l_M_128, kexit_info
    DUP3 DUP2 EQ %jumpi(expmod_store_end)
    // stack: cur_offset, address, end_offset, l_M_128, kexit_info
    DUP1 %mload_current_general
    %stack (cur_limb, cur_offset, address, end_offset, l_M_128, kexit_info) ->
         (address, cur_limb, cur_offset, end_offset, l_M_128, kexit_info)
    %stack (address, cur_limb) -> (address, cur_limb, 16)
    %mstore_unpacking
    // stack: address', cur_offset, end_offset, l_M_128, kexit_info)
    SWAP1 %decrement
    // stack: cur_offset-1, address', end_offset, l_M_128, kexit_info)
    %jump(expmod_store_loop)
expmod_store_end:
    // stack: cur_offset, address, end_offset, l_M_128, kexit_info
    %pop4
the_end:
    // stack: kexit_info
    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

zero_modulus:
    // stack: l_M, len, kexit_info
    %pop2
    %jump(the_end)

zero_base_zero_mod:
    // stack: l_M, l_E, l_B, kexit_info
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE)
    // stack: l_E, l_B, kexit_info
    %pop2
    // stack: kexit_info
    PUSH 200
    %charge_gas
    // stack: kexit_info
    %jump(the_end)
