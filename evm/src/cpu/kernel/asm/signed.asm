// BYTE returns byte N of value, where N=0 corresponds to bits
// [248,256) ... N=31 corresponds to bits [0,31); i.e. N is the Nth
// byte of value when it is considered as BIG-endian.
global sys_byte:
    // Stack: N, value, retdest
    %mul_const(8)
    // Stack:  8*N, value, retdest
    shl
    push 248
    shr
    // Stack: (value << 8*N) >> 248, retdest
    swap1
    jump

// SIGNEXTEND from the Nth byte of value, where the bytes of value are
// considered in LITTLE-endian order. Just a SHL followed by a SAR.
global sys_signextend:
    // Stack: N, value, retdest
    // Handle N >= 31, which is a no-op.
    push 31
    %max
    // Stack: max(31, N), value, retdest
    %increment
    %mul_const(8)
    // Stack: 8*(N + 1), value, retdest
    push 256
    sub
    // Stack: 256 - 8*(N + 1), value, retdest
    %stack(bits, value, retdest) -> (bits, value, bits, retdest)
    shl
    swap1
    // Stack: bits, value << bits, retdest
    jump(arithmetic_shift_right)

// SAR, i.e. shift arithmetic right, shifts `value` `shift` bits to
// the right, preserving sign by filling with the most significant bit.
//
// Reference: Hacker's Delight, 2013, 2nd edition, §2-7.
global sys_sar:
    // SAR(shift, value) is the same for all shift >= 255, so we
    // replace shift with min(shift, 255)

    // Stack: shift, value, retdest
    push 255
    %min
    // Stack: min(shift, 255), value, retdest

    // Now assume shift < 256.
    // Stack: shift, value, retdest
    push @TOP_BIT_MASK@
    dup2
    shr
    // Stack: 2^255 >> shift, shift, value, retdest
    swap2
    %add_const(@TOP_BIT_MASK@)
    // Stack: 2^255 + value, shift, 2^255 >> shift, retdest
    swap1
    shr
    sub
    // Stack: ((2^255 + value) >> shift) - (2^255 >> shift), retdest
    swap1
    jump

// SLT, i.e. signed less than, returns 1 if lhs < rhs as signed
// integers, 0 otherwise.
//
// Reference: Hacker's Delight, 2013, 2nd edition, §2-12.
global sys_slt:
    // Stack: lhs, rhs, retdest
    %add_const(@TOP_BIT_MASK@)
    // Stack: 2^255 + lhs, rhs, retdest
    swap1
    %add_const(@TOP_BIT_MASK@)
    // Stack: 2^255 + rhs, 2^255 + lhs, retdest
    gt
    // Stack: 2^255 + lhs < 2^255 + rhs, retdest
    swap1
    jump

// SGT, i.e. signed greater than, returns 1 if lhs > rhs as signed
// integers, 0 otherwise.
//
// Just delegate to signed less than.
global sys_sgt:
    swap1
    %jump(signed_less_than)
