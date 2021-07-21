use num_bigint::BigUint;

use crate::field::field::Field;
use crate::util::{bits_u64, ceil_div_usize};

/// Generates a series of non-negative integers less than
/// `modulus` which cover a range of values and which will
/// generate lots of carries, especially at `word_bits` word
/// boundaries.
pub fn test_inputs(modulus: BigUint, word_bits: usize) -> Vec<BigUint> {
    //assert!(word_bits == 32 || word_bits == 64);
    let modwords = ceil_div_usize(modulus.bits(), word_bits);
    // Start with basic set close to zero: 0 .. 10
    const BIGGEST_SMALL: u32 = 10;
    let smalls: Vec<_> = (0..BIGGEST_SMALL).map(BigUint::from).collect();
    // ... and close to MAX: MAX - x
    let word_max = (BigUint::from(1u32) << word_bits) - 1u32;
    let bigs = smalls.iter().map(|x| &word_max - x).collect();
    let one_words = [smalls, bigs].concat();
    // For each of the one word inputs above, create a new one at word i.
    // TODO: Create all possible `modwords` combinations of those
    let multiple_words = (1..modwords)
        .flat_map(|i| {
            one_words
                .iter()
                .map(|x| x << (word_bits * i))
                .collect::<Vec<BigUint>>()
        })
        .collect();
    let basic_inputs: Vec<BigUint> = [one_words, multiple_words].concat();

    // Biggest value that will fit in `modwords` words
    // Inputs 'difference from' maximum value
    let diff_max = basic_inputs
        .iter()
        .map(|x| x.clone())
        .map(|x| word_max.clone() - x)
        .filter(|x| x < &modulus)
        .collect();
    // Inputs 'difference from' modulus value
    let diff_mod = basic_inputs
        .iter()
        .map(|x| x.clone())
        .filter(|&x| x < modulus && x != BigUint::from(0u32))
        .map(|x| x.clone())
        .map(|x| modulus - x)
        .collect();
    let basics = basic_inputs
        .into_iter()
        .map(|x| x.clone())
        .filter(|x| x < &modulus)
        .collect::<Vec<BigUint>>();
    [basics, diff_max, diff_mod].concat()

    // // There should be a nicer way to express the code above; something
    // // like this (and removing collect() calls from diff_max and diff_mod):
    // basic_inputs.into_iter()
    //     .chain(diff_max)
    //     .chain(diff_mod)
    //     .filter(|x| x < &modulus)
    //     .collect()
}

/// Apply the unary functions `op` and `expected_op`
/// coordinate-wise to the inputs from `test_inputs(modulus,
/// word_bits)` and panic if the two resulting vectors differ.
pub fn run_unaryop_test_cases<F, UnaryOp, ExpectedOp>(
    modulus: BigUint,
    word_bits: usize,
    op: UnaryOp,
    expected_op: ExpectedOp,
) where
    F: Field,
    UnaryOp: Fn(F) -> F,
    ExpectedOp: Fn(BigUint) -> BigUint,
{
    let inputs = test_inputs(modulus, word_bits);
    let expected: Vec<_> = inputs.iter().map(|&x| expected_op(x)).collect();
    let output: Vec<_> = inputs
        .iter()
        .map(|x| x.clone())
        .map(|x| op(F::from_canonical_biguint(x)).to_canonical_biguint())
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

/// Apply the binary functions `op` and `expected_op` to each pair
/// in `zip(inputs, rotate_right(inputs, i))` where `inputs` is
/// `test_inputs(modulus, word_bits)` and `i` ranges from 0 to
/// `inputs.len()`.  Panic if the two functions ever give
/// different answers.
pub fn run_binaryop_test_cases<F, BinaryOp, ExpectedOp>(
    modulus: BigUint,
    word_bits: usize,
    op: BinaryOp,
    expected_op: ExpectedOp,
) where
    F: Field,
    BinaryOp: Fn(F, F) -> F,
    ExpectedOp: Fn(BigUint, BigUint) -> BigUint,
{
    let inputs = test_inputs(modulus, word_bits);

    for i in 0..inputs.len() {
        // Iterator over inputs rotated right by i places. Since
        // cycle().skip(i) rotates left by i, we need to rotate by
        // n_input_elts - i.
        let shifted_inputs: Vec<_> = inputs
            .iter()
            .cycle()
            .skip(inputs.len() - i)
            .take(inputs.len())
            .collect();

        // Calculate pointwise operations
        let expected: Vec<_> = inputs
            .iter()
            .zip(shifted_inputs.clone())
            .map(|(x, y)| expected_op(x.clone(), y.clone()))
            .collect();

        let output: Vec<_> = inputs
            .iter()
            .zip(shifted_inputs.clone())
            .map(|(x, y)| (x.clone(), y.clone()))
            .map(|(x, y)| {
                op(F::from_canonical_biguint(x), F::from_canonical_biguint(y)).to_canonical_biguint()
            })
            .collect();

        // Compare expected outputs with actual outputs
        for i in 0..inputs.len() {
            assert_eq!(
                output[i], expected[i],
                "On inputs {} . {}, expected {} but got {}",
                inputs[i], shifted_inputs[i], expected[i], output[i]
            );
        }
    }
}

#[macro_export]
macro_rules! test_arithmetic {
    ($field:ty) => {
        mod arithmetic {
            use std::ops::{Add, Mul, Neg, Sub};

            use num_bigint::BigUint;

            use crate::field::field::Field;

            // Can be 32 or 64; doesn't have to be computer's actual word
            // bits. Choosing 32 gives more tests...
            const WORD_BITS: usize = 32;

            #[test]
            fn arithmetic_addition() {
                let modulus = <$field>::order();
                crate::field::field_testing::run_binaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    <$field>::add,
                    BigUint::add,
                )
            }

            #[test]
            fn arithmetic_subtraction() {
                let modulus = <$field>::order();
                crate::field::field_testing::run_binaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    <$field>::sub,
                    BigUint::sub,
                )
            }

            #[test]
            fn arithmetic_negation() {
                let modulus = <$field>::order();
                crate::field::field_testing::run_unaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    <$field>::neg,
                    BigUint::neg,
                )
            }

            #[test]
            fn arithmetic_multiplication() {
                let modulus = <$field>::order();
                crate::field::field_testing::run_binaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    <$field>::mul,
                    BigUint::mul,
                )
            }

            #[test]
            fn arithmetic_square() {
                let modulus = <$field>::order();
                crate::field::field_testing::run_unaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    |x: $field| x.square(),
                    |x| x.clone() * x,
                )
            }

            #[test]
            fn inversion() {
                let zero = <$field>::ZERO;
                let one = <$field>::ONE;
                let order = <$field>::order();

                assert_eq!(zero.try_inverse(), None);

                for x in [
                    BigUint::from(1u32),
                    BigUint::from(2u32),
                    BigUint::from(3u32),
                    order.clone() - 3u32,
                    order.clone() - 2u32,
                    order.clone() - 1u32,
                ] {
                    let x = <$field>::from_canonical_biguint(x);
                    let inv = x.inverse();
                    assert_eq!(x * inv, one);
                }
            }

            #[test]
            fn batch_inversion() {
                let xs = (1..=3)
                    .map(|i| <$field>::from_canonical_u64(i))
                    .collect::<Vec<_>>();
                let invs = <$field>::batch_multiplicative_inverse(&xs);
                for (x, inv) in xs.into_iter().zip(invs) {
                    assert_eq!(x * inv, <$field>::ONE);
                }
            }

            #[test]
            fn primitive_root_order() {
                for n_power in 0..8 {
                    let root = <$field>::primitive_root_of_unity(n_power);
                    let order = <$field>::generator_order(root);
                    assert_eq!(order, 1 << n_power, "2^{}'th primitive root", n_power);
                }
            }

            #[test]
            fn negation() {
                let zero = <$field>::ZERO;
                let order = <$field>::order();

                for i in [
                    BigUint::from(0u32),
                    BigUint::from(1u32),
                    BigUint::from(2u32),
                    order.clone() - 2u32,
                    order.clone() - 1u32,
                ] {
                    let i_f = <$field>::from_canonical_biguint(i);
                    assert_eq!(i_f + -i_f, zero);
                }
            }

            #[test]
            fn bits() {
                assert_eq!(<$field>::ZERO.bits(), 0);
                assert_eq!(<$field>::ONE.bits(), 1);
                assert_eq!(<$field>::TWO.bits(), 2);
                assert_eq!(<$field>::from_canonical_u64(3).bits(), 2);
                assert_eq!(<$field>::from_canonical_u64(4).bits(), 3);
                assert_eq!(<$field>::from_canonical_u64(5).bits(), 3);
            }

            #[test]
            fn exponentiation() {
                type F = $field;

                assert_eq!(F::ZERO.exp_u32(0), <F>::ONE);
                assert_eq!(F::ONE.exp_u32(0), <F>::ONE);
                assert_eq!(F::TWO.exp_u32(0), <F>::ONE);

                assert_eq!(F::ZERO.exp_u32(1), <F>::ZERO);
                assert_eq!(F::ONE.exp_u32(1), <F>::ONE);
                assert_eq!(F::TWO.exp_u32(1), <F>::TWO);

                assert_eq!(F::ZERO.kth_root_u32(1), <F>::ZERO);
                assert_eq!(F::ONE.kth_root_u32(1), <F>::ONE);
                assert_eq!(F::TWO.kth_root_u32(1), <F>::TWO);

                for power in 1..10 {
                    if F::is_monomial_permutation(power) {
                        let x = F::rand();
                        assert_eq!(x.exp(power).kth_root(power), x);
                    }
                }
            }

            #[test]
            fn subtraction() {
                type F = $field;

                let (a, b) = (
                    F::from_canonical_biguint((F::order() + 1u32) / 2u32),
                    F::TWO,
                );
                let x = a * b;
                assert_eq!(x, F::ONE);
                assert_eq!(F::ZERO - x, F::NEG_ONE);
            }

            #[test]
            fn inverse_2exp() {
                // Just check consistency with try_inverse()
                type F = $field;

                let v = <F as Field>::PrimeField::TWO_ADICITY;

                for e in [0, 1, 2, 3, 4, v - 2, v - 1, v, v + 1, v + 2, 123 * v] {
                    let x = F::TWO.exp(e as u64).inverse();
                    let y = F::inverse_2exp(e);
                    assert_eq!(x, y);
                }
            }
        }
    };
}
