// SDIV(a, b): signed division operation.
//
// If b = 0, then SDIV(a, b) = 0,
// else if a = -2^255 and b = -1, then SDIV(a, b) = -2^255
// else SDIV(a, b) = sgn(a/b) * floor(|a/b|).
global sys_sdiv:
    // stack: num, denom, return_info
    dup1
    push 0x8000000000000000000000000000000000000000000000000000000000000000
    gt
    // stack: num_is_nonneg := sign_bit > num, num, denom, return_info
    dup1
    %jumpi(sys_sdiv_nonneg_num)
    // stack: num_is_nonneg, num, denom, return_info
    swap1
    push 0
    sub
    swap1
    // stack: num_is_nonneg, num := -num, denom, return_info
sys_sdiv_nonneg_num:
    swap2
    dup1
    push 0x8000000000000000000000000000000000000000000000000000000000000000
    gt
    // stack: denom_is_nonneg := sign_bit > denom, denom, num, num_is_nonneg, return_info
    dup1
    %jumpi(sys_sdiv_nonneg_denom)
    // stack: denom_is_nonneg, denom, num, num_is_nonneg, return_info
    swap1
    push 0
    sub
    // stack: denom := -denom, denom_is_nonneg, num, num_is_nonneg, return_info
    swap1
sys_sdiv_nonneg_denom:
    // stack: denom_is_nonneg, denom, num, num_is_nonneg, return_info
    swap2
    div
    // stack: num / denom, denom_is_nonneg, num_is_nonneg, return_info
    swap2
    eq
    // stack: denom_is_nonneg == num_is_nonneg, num / denom, return_info
    %jumpi(sys_sdiv_same_sign)
    push 0
    sub
sys_sdiv_same_sign:
    swap1
    //FIXME: exit_kernel
    jump


// SMOD(a, b): signed "modulo remainder" operation.
//
// If b != 0, then SMOD(a, b) = sgn(a) * MOD(|a|, |b|),
// else SMOD(a, 0) = 0.
global sys_smod:
    // stack: x, mod, return_info
    push 0x8000000000000000000000000000000000000000000000000000000000000000
    // stack: sign_bit, x, mod, return_info
    dup1
    dup4
    lt
    // stack: mod < sign_bit, sign_bit, x, mod, return_info
    %jumpi(sys_smod_pos_mod)
    // mod is negative, so we negate it
    // sign_bit, x, mod, return_info
    swap2
    push 0
    sub
    swap2
    // sign_bit, x, mod := 0 - mod, return_info
sys_smod_pos_mod:
    // At this point, we know that mod is non-negative.
    dup2
    lt
    // stack: x < sign_bit, x, mod, return_info
    %jumpi(sys_smod_pos_x)
    // x is negative, so let's negate it
    // stack: x, mod, return_info
    push 0
    sub
    // stack: x := 0 - x, mod, return_info
    mod
    // negate the result
    push 0
    sub
    swap1
    //FIXME: exit_kernel
    jump
sys_smod_pos_x:
    // Both x and mod are non-negative
    // stack: x, mod, return_info
    mod
    swap1
    //FIXME: exit_kernel
    jump

// BYTE returns byte N of value, where N=0 corresponds to bits
// [248,256) ... N=31 corresponds to bits [0,31); i.e. N is the Nth
// byte of value when it is considered as BIG-endian.
global sys_byte:
    // Stack: N, value, return_info
    %mul_const(8)
    // Stack:  8*N, value, return_info
    shl
    push 248
    shr
    // Stack: (value << 8*N) >> 248, return_info
    swap1
    //FIXME: exit_kernel
    jump

// SIGNEXTEND from the Nth byte of value, where the bytes of value are
// considered in LITTLE-endian order. Just a SHL followed by a SAR.
global sys_signextend:
    // Stack: N, value, return_info
    // Handle N >= 31, which is a no-op.
    push 31
    %min
    // Stack: min(31, N), value, return_info
    %increment
    %mul_const(8)
    // Stack: 8*(N + 1), value, return_info
    push 256
    sub
    // Stack: 256 - 8*(N + 1), value, return_info
    %stack(bits, value, return_info) -> (bits, value, bits, return_info)
    shl
    swap1
    // Stack: bits, value << bits, return_info
    // fall through to sys_sar

// SAR, i.e. shift arithmetic right, shifts `value` `shift` bits to
// the right, preserving sign by filling with the most significant bit.
//
// Trick: x >>s i = (x + sign_bit >>u i) - (sign_bit >>u i),
//   where >>s is arithmetic shift and >>u is logical shift.
// Reference: Hacker's Delight, 2013, 2nd edition, §2-7.
global sys_sar:
    // SAR(shift, value) is the same for all shift >= 255, so we
    // replace shift with min(shift, 255)

    // Stack: shift, value, return_info
    push 255
    %min
    // Stack: min(shift, 255), value, return_info

    // Now assume shift < 256.
    // Stack: shift, value, return_info
    push 0x8000000000000000000000000000000000000000000000000000000000000000
    dup2
    shr
    // Stack: 2^255 >> shift, shift, value, return_info
    swap2
    %add_const(0x8000000000000000000000000000000000000000000000000000000000000000)
    // Stack: 2^255 + value, shift, 2^255 >> shift, return_info
    swap1
    shr
    sub
    // Stack: ((2^255 + value) >> shift) - (2^255 >> shift), return_info
    swap1
    //FIXME: exit_kernel
    jump

// SGT, i.e. signed greater than, returns 1 if lhs > rhs as signed
// integers, 0 otherwise.
//
// Just swap argument order and fall through to signed less than.
global sys_sgt:
    swap1

// SLT, i.e. signed less than, returns 1 if lhs < rhs as signed
// integers, 0 otherwise.
//
// Trick: x <s y iff (x ^ sign_bit) <u (y ^ sign bit),
//   where <s is signed comparison and <u is unsigned comparison.
// Reference: Hacker's Delight, 2013, 2nd edition, §2-12.
global sys_slt:
    // Stack: lhs, rhs, return_info
    %add_const(0x8000000000000000000000000000000000000000000000000000000000000000)
    // Stack: 2^255 + lhs, rhs, return_info
    swap1
    %add_const(0x8000000000000000000000000000000000000000000000000000000000000000)
    // Stack: 2^255 + rhs, 2^255 + lhs, return_info
    gt
    // Stack: 2^255 + lhs < 2^255 + rhs, return_info
    swap1
    //FIXME: exit_kernel
    jump
