// SDIV(a, b): signed division operation.
//
// If b = 0, then SDIV(a, b) = 0,
// else if a = -2^255 and b = -1, then SDIV(a, b) = -2^255
// else SDIV(a, b) = sgn(a/b) * floor(|a/b|).
global _sys_sdiv:
    // stack: num, denom, return_info
    DUP1
    PUSH 0x8000000000000000000000000000000000000000000000000000000000000000
    GT
    // stack: num_is_nonneg := sign_bit > num, num, denom, return_info
    DUP1
    %jumpi(sys_sdiv_nonneg_num)
    // stack: num_is_nonneg, num, denom, return_info
    SWAP1
    PUSH 0
    SUB
    SWAP1
    // stack: num_is_nonneg, num := -num, denom, return_info
sys_sdiv_nonneg_num:
    SWAP2
    DUP1
    PUSH 0x8000000000000000000000000000000000000000000000000000000000000000
    GT
    // stack: denom_is_nonneg := sign_bit > denom, denom, num, num_is_nonneg, return_info
    DUP1
    %jumpi(sys_sdiv_nonneg_denom)
    // stack: denom_is_nonneg, denom, num, num_is_nonneg, return_info
    SWAP1
    PUSH 0
    SUB
    // stack: denom := -denom, denom_is_nonneg, num, num_is_nonneg, return_info
    SWAP1
sys_sdiv_nonneg_denom:
    // stack: denom_is_nonneg, denom, num, num_is_nonneg, return_info
    SWAP2
    DIV
    // stack: num / denom, denom_is_nonneg, num_is_nonneg, return_info
    SWAP2
    EQ
    // stack: denom_is_nonneg == num_is_nonneg, num / denom, return_info
    %jumpi(sys_sdiv_same_sign)
    PUSH 0
    SUB
sys_sdiv_same_sign:
    SWAP1
    JUMP


// SMOD(a, b): signed "modulo remainder" operation.
//
// If b != 0, then SMOD(a, b) = sgn(a) * MOD(|a|, |b|),
// else SMOD(a, 0) = 0.
global _sys_smod:
    // stack: x, mod, return_info
    PUSH 0x8000000000000000000000000000000000000000000000000000000000000000
    // stack: sign_bit, x, mod, return_info
    DUP1
    DUP4
    LT
    // stack: mod < sign_bit, sign_bit, x, mod, return_info
    %jumpi(sys_smod_pos_mod)
    // mod is negative, so we negate it
    // sign_bit, x, mod, return_info
    SWAP2
    PUSH 0
    SUB
    SWAP2
    // sign_bit, x, mod := 0 - mod, return_info
sys_smod_pos_mod:
    // At this point, we know that mod is non-negative.
    DUP2
    LT
    // stack: x < sign_bit, x, mod, return_info
    %jumpi(sys_smod_pos_x)
    // x is negative, so let's negate it
    // stack: x, mod, return_info
    PUSH 0
    SUB
    // stack: x := 0 - x, mod, return_info
    MOD
    // negate the result
    PUSH 0
    SUB
    SWAP1
    JUMP
sys_smod_pos_x:
    // Both x and mod are non-negative
    // stack: x, mod, return_info
    MOD
    SWAP1
    JUMP


// SIGNEXTEND from the Nth byte of value, where the bytes of value are
// considered in LITTLE-endian order. Just a SHL followed by a SAR.
global _sys_signextend:
    // Stack: N, value, return_info
    // Handle N >= 31, which is a no-op.
    PUSH 31
    %min
    // Stack: min(31, N), value, return_info
    %increment
    %mul_const(8)
    // Stack: 8*(N + 1), value, return_info
    PUSH 256
    SUB
    // Stack: 256 - 8*(N + 1), value, return_info
    %stack(bits, value, return_info) -> (bits, value, bits, return_info)
    SHL
    SWAP1
    // Stack: bits, value << bits, return_info
    // fall through to sys_sar


// SAR, i.e. shift arithmetic right, shifts `value` `shift` bits to
// the right, preserving sign by filling with the most significant bit.
//
// Trick: x >>s i = (x + sign_bit >>u i) - (sign_bit >>u i),
//   where >>s is arithmetic shift and >>u is logical shift.
// Reference: Hacker's Delight, 2013, 2nd edition, §2-7.
global _sys_sar:
    // SAR(shift, value) is the same for all shift >= 255, so we
    // replace shift with min(shift, 255)

    // Stack: shift, value, return_info
    PUSH 255
    %min
    // Stack: min(shift, 255), value, return_info

    // Now assume shift < 256.
    // Stack: shift, value, return_info
    PUSH 0x8000000000000000000000000000000000000000000000000000000000000000
    DUP2
    SHR
    // Stack: 2^255 >> shift, shift, value, return_info
    SWAP2
    %add_const(0x8000000000000000000000000000000000000000000000000000000000000000)
    // Stack: 2^255 + value, shift, 2^255 >> shift, return_info
    SWAP1
    SHR
    SUB
    // Stack: ((2^255 + value) >> shift) - (2^255 >> shift), return_info
    SWAP1
    JUMP


// SGT, i.e. signed greater than, returns 1 if lhs > rhs as signed
// integers, 0 otherwise.
//
// Just swap argument order and fall through to signed less than.
global _sys_sgt:
    SWAP1


// SLT, i.e. signed less than, returns 1 if lhs < rhs as signed
// integers, 0 otherwise.
//
// Trick: x <s y iff (x ^ sign_bit) <u (y ^ sign bit),
//   where <s is signed comparison and <u is unsigned comparison.
// Reference: Hacker's Delight, 2013, 2nd edition, §2-12.
global _sys_slt:
    // Stack: lhs, rhs, return_info
    %add_const(0x8000000000000000000000000000000000000000000000000000000000000000)
    // Stack: 2^255 + lhs, rhs, return_info
    SWAP1
    %add_const(0x8000000000000000000000000000000000000000000000000000000000000000)
    // Stack: 2^255 + rhs, 2^255 + lhs, return_info
    GT
    // Stack: 2^255 + lhs < 2^255 + rhs, return_info
    SWAP1
    JUMP


/// These are the global entry-points for the signed system
/// calls. They just delegate to a subroutine with the same name
/// preceded by an underscore.
///
/// NB: The only reason to structure things this way is so that the
/// test suite can call the _sys_opcode versions, since the test_suite
/// uses our interpreter which doesn't handle `EXIT_KERNEL` in a way
/// that allows for easy testing. The cost is two extra JUMPs per call.

global sys_sdiv:
    %charge_gas_const(@GAS_LOW)
    %stack(kernel_return, x, y) -> (_sys_sdiv, x, y, _syscall_return, kernel_return)
    JUMP

global sys_smod:
    %charge_gas_const(@GAS_LOW)
    %stack(kernel_return, x, y) -> (_sys_smod, x, y, _syscall_return, kernel_return)
    JUMP

global sys_signextend:
    %charge_gas_const(@GAS_LOW)
    %stack(kernel_return, x, y) -> (_sys_signextend, x, y, _syscall_return, kernel_return)
    JUMP

global sys_sar:
    %charge_gas_const(@GAS_VERYLOW)
    %stack(kernel_return, x, y) -> (_sys_sar, x, y, _syscall_return, kernel_return)
    JUMP

global sys_slt:
    %charge_gas_const(@GAS_VERYLOW)
    %stack(kernel_return, x, y) -> (_sys_slt, x, y, _syscall_return, kernel_return)
    JUMP

global sys_sgt:
    %charge_gas_const(@GAS_VERYLOW)
    %stack(kernel_return, x, y) -> (_sys_sgt, x, y, _syscall_return, kernel_return)
    JUMP

_syscall_return:
    SWAP1
    EXIT_KERNEL
