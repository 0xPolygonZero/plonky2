use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;

/// Generate a list of inputs suitable for testing the signed operations
///
/// The result includes 0, ±1, ±2^(16i ± 1) for i = 0..15, and ±2^255
/// and then each of those ±1. Little attempt has been made to avoid
/// duplicates. Total length is 279.
fn test_inputs() -> Vec<U256> {
    let mut res = vec![U256::zero()];
    for i in 1..16 {
        res.push(U256::one() << (16 * i));
        res.push(U256::one() << (16 * i + 1));
        res.push(U256::one() << (16 * i - 1));
    }
    res.push(U256::one() << 255);

    let n = res.len();
    for i in 1..n {
        // push -res[i]
        res.push(res[i].overflowing_neg().0);
    }

    let n = res.len();
    for i in 0..n {
        res.push(res[i].overflowing_add(U256::one()).0);
        res.push(res[i].overflowing_sub(U256::one()).0);
    }

    res
}

// U256_TOP_BIT == 2^255.
const U256_TOP_BIT: U256 = U256([0x0, 0x0, 0x0, 0x8000000000000000]);

/// Given a U256 `value`, interpret as a signed 256-bit number and
/// return the arithmetic right shift of `value` by `shift` bit
/// positions, i.e. the right shift of `value` with sign extension.
fn u256_sar(shift: U256, value: U256) -> U256 {
    // Reference: Hacker's Delight, 2013, 2nd edition, §2-7.
    let shift = shift.min(U256::from(255));
    ((value ^ U256_TOP_BIT) >> shift)
        .overflowing_sub(U256_TOP_BIT >> shift)
        .0
}

/// Given a U256 x, interpret it as a signed 256-bit number and return
/// the pair abs(x) and sign(x), where sign(x) = 1 if x < 0, and 0
/// otherwise. NB: abs(x) is interpreted as an unsigned value, so
/// u256_abs_sgn(-2^255) = (2^255, -1).
fn u256_abs_sgn(x: U256) -> (U256, bool) {
    let is_neg = x.bit(255);

    // negate x if it's negative
    let x = if is_neg { x.overflowing_neg().0 } else { x };
    (x, is_neg)
}

fn u256_sdiv(x: U256, y: U256) -> U256 {
    let (abs_x, x_is_neg) = u256_abs_sgn(x);
    let (abs_y, y_is_neg) = u256_abs_sgn(y);
    if y.is_zero() {
        U256::zero()
    } else {
        let quot = abs_x / abs_y;
        // negate the quotient if arguments had opposite signs
        if x_is_neg != y_is_neg {
            quot.overflowing_neg().0
        } else {
            quot
        }
    }
}

fn u256_smod(x: U256, y: U256) -> U256 {
    let (abs_x, x_is_neg) = u256_abs_sgn(x);
    let (abs_y, _) = u256_abs_sgn(y);

    if y.is_zero() {
        U256::zero()
    } else {
        let rem = abs_x % abs_y;
        // negate the remainder if dividend was negative
        if x_is_neg {
            rem.overflowing_neg().0
        } else {
            rem
        }
    }
}

// signextend is just a SHL followed by SAR.
fn u256_signextend(byte: U256, value: U256) -> U256 {
    // byte = min(31, byte)
    let byte: u32 = byte.min(U256::from(31)).try_into().unwrap();
    let bit_offset = 256 - 8 * (byte + 1);
    u256_sar(U256::from(bit_offset), value << bit_offset)
}

// Reference: Hacker's Delight, 2013, 2nd edition, §2-12.
fn u256_slt(x: U256, y: U256) -> U256 {
    let top_bit: U256 = U256::one() << 255;
    U256::from(((x ^ top_bit) < (y ^ top_bit)) as u32)
}

fn u256_sgt(x: U256, y: U256) -> U256 {
    u256_slt(y, x)
}

fn run_test(fn_label: &str, expected_fn: fn(U256, U256) -> U256, opname: &str) {
    let inputs = test_inputs();
    let fn_label = KERNEL.global_labels[fn_label];
    let retdest = U256::from(0xDEADBEEFu32);

    for &x in &inputs {
        for &y in &inputs {
            let stack = vec![retdest, y, x];
            let mut interpreter = Interpreter::new_with_kernel(fn_label, stack);
            interpreter.run().unwrap();
            assert_eq!(interpreter.stack_len(), 1usize, "unexpected stack size");
            let output = interpreter
                .stack_top()
                .expect("The stack should not be empty.");
            let expected_output = expected_fn(x, y);
            assert_eq!(
                output, expected_output,
                "{opname}({x}, {y}): expected {expected_output} but got {output}"
            );
        }
    }
}

#[test]
fn test_sdiv() {
    // Double-check that the expected output calculation is correct in the special case.
    let x = U256::one() << 255; // -2^255
    let y = U256::one().overflowing_neg().0; // -1
    assert_eq!(u256_sdiv(x, y), x); // SDIV(-2^255, -1) = -2^255.

    run_test("_sys_sdiv", u256_sdiv, "SDIV");
}

#[test]
fn test_smod() {
    run_test("_sys_smod", u256_smod, "SMOD");
}

#[test]
fn test_signextend() {
    run_test("_sys_signextend", u256_signextend, "SIGNEXTEND");
}

#[test]
fn test_sar() {
    run_test("_sys_sar", u256_sar, "SAR");
}

#[test]
fn test_slt() {
    run_test("_sys_slt", u256_slt, "SLT");
}

#[test]
fn test_sgt() {
    run_test("_sys_sgt", u256_sgt, "SGT");
}
