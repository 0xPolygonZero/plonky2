use alloc::vec::Vec;

use crate::types::PrimeField64;

/// Generates a series of non-negative integers less than `modulus` which cover a range of
/// interesting test values.
pub fn test_inputs(modulus: u64) -> Vec<u64> {
    const CHUNK_SIZE: u64 = 10;

    (0..CHUNK_SIZE)
        .chain((1 << 31) - CHUNK_SIZE..(1 << 31) + CHUNK_SIZE)
        .chain((1 << 32) - CHUNK_SIZE..(1 << 32) + CHUNK_SIZE)
        .chain((1 << 63) - CHUNK_SIZE..(1 << 63) + CHUNK_SIZE)
        .chain(modulus - CHUNK_SIZE..modulus)
        .filter(|&x| x < modulus)
        .collect()
}

/// Apply the unary functions `op` and `expected_op`
/// coordinate-wise to the inputs from `test_inputs(modulus,
/// word_bits)` and panic if the two resulting vectors differ.
pub fn run_unaryop_test_cases<F, UnaryOp, ExpectedOp>(op: UnaryOp, expected_op: ExpectedOp)
where
    F: PrimeField64,
    UnaryOp: Fn(F) -> F,
    ExpectedOp: Fn(u64) -> u64,
{
    let inputs = test_inputs(F::ORDER);
    let expected: Vec<_> = inputs.iter().map(|&x| expected_op(x)).collect();
    let output: Vec<_> = inputs
        .iter()
        .cloned()
        .map(|x| op(F::from_canonical_u64(x)).to_canonical_u64())
        .collect();
    // Compare expected outputs with actual outputs
    for i in 0..inputs.len() {
        assert_eq!(
            output[i], expected[i],
            "Expected {}, got {} for input {}",
            expected[i], output[i], inputs[i]
        );
    }
}

/// Apply the binary functions `op` and `expected_op` to each pair of inputs.
pub fn run_binaryop_test_cases<F, BinaryOp, ExpectedOp>(op: BinaryOp, expected_op: ExpectedOp)
where
    F: PrimeField64,
    BinaryOp: Fn(F, F) -> F,
    ExpectedOp: Fn(u64, u64) -> u64,
{
    let inputs = test_inputs(F::ORDER);

    for &lhs in &inputs {
        for &rhs in &inputs {
            let lhs_f = F::from_canonical_u64(lhs);
            let rhs_f = F::from_canonical_u64(rhs);
            let actual = op(lhs_f, rhs_f).to_canonical_u64();
            let expected = expected_op(lhs, rhs);
            assert_eq!(
                actual, expected,
                "Expected {}, got {} for inputs ({}, {})",
                expected, actual, lhs, rhs
            );
        }
    }
}

#[macro_export]
macro_rules! test_prime_field_arithmetic {
    ($field:ty) => {
        mod prime_field_arithmetic {
            use core::ops::{Add, Mul, Neg, Sub};

            use $crate::ops::Square;
            use $crate::types::{Field, Field64};

            #[test]
            fn arithmetic_addition() {
                let modulus = <$field>::ORDER;
                $crate::prime_field_testing::run_binaryop_test_cases(<$field>::add, |x, y| {
                    ((x as u128 + y as u128) % (modulus as u128)) as u64
                })
            }

            #[test]
            fn arithmetic_subtraction() {
                let modulus = <$field>::ORDER;
                $crate::prime_field_testing::run_binaryop_test_cases(<$field>::sub, |x, y| {
                    if x >= y {
                        x - y
                    } else {
                        modulus - y + x
                    }
                })
            }

            #[test]
            fn arithmetic_negation() {
                let modulus = <$field>::ORDER;
                $crate::prime_field_testing::run_unaryop_test_cases(<$field>::neg, |x| {
                    if x == 0 {
                        0
                    } else {
                        modulus - x
                    }
                })
            }

            #[test]
            fn arithmetic_multiplication() {
                let modulus = <$field>::ORDER;
                $crate::prime_field_testing::run_binaryop_test_cases(<$field>::mul, |x, y| {
                    ((x as u128) * (y as u128) % (modulus as u128)) as u64
                })
            }

            #[test]
            fn arithmetic_square() {
                let modulus = <$field>::ORDER;
                $crate::prime_field_testing::run_unaryop_test_cases(
                    |x: $field| x.square(),
                    |x| ((x as u128 * x as u128) % (modulus as u128)) as u64,
                )
            }

            #[test]
            fn inversion() {
                let zero = <$field>::ZERO;
                let one = <$field>::ONE;
                let modulus = <$field>::ORDER;

                assert_eq!(zero.try_inverse(), None);

                let inputs = $crate::prime_field_testing::test_inputs(modulus);

                for x in inputs {
                    if x != 0 {
                        let x = <$field>::from_canonical_u64(x);
                        let inv = x.inverse();
                        assert_eq!(x * inv, one);
                    }
                }
            }

            #[test]
            fn inverse_2exp() {
                type F = $field;

                let v = <F as Field>::TWO_ADICITY;

                for e in [0, 1, 2, 3, 4, v - 2, v - 1, v, v + 1, v + 2, 123 * v] {
                    let x = F::TWO.exp_u64(e as u64);
                    let y = F::inverse_2exp(e);
                    assert_eq!(x * y, F::ONE);
                }
            }

            #[test]
            fn subtraction_double_wraparound() {
                type F = $field;

                let (a, b) = (F::from_canonical_u64(F::ORDER.div_ceil(2u64)), F::TWO);
                let x = a * b;
                assert_eq!(x, F::ONE);
                assert_eq!(F::ZERO - x, F::NEG_ONE);
            }

            #[test]
            fn addition_double_wraparound() {
                type F = $field;

                let a = F::from_canonical_u64(u64::MAX - F::ORDER);
                let b = F::NEG_ONE;

                let c = (a + a) + (b + b);
                let d = (a + b) + (a + b);

                assert_eq!(c, d);
            }
        }
    };
}
