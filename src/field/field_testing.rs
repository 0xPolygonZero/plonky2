use crate::field::field::Field;
use crate::util::{bits_u64, ceil_div_usize};

/// Generates a series of non-negative integers less than
/// `modulus` which cover a range of values and which will
/// generate lots of carries, especially at `word_bits` word
/// boundaries.
pub fn test_inputs(modulus: u64, word_bits: usize) -> Vec<u64> {
    assert!(word_bits == 32 || word_bits == 64);
    let modwords = ceil_div_usize(bits_u64(modulus), word_bits);
    // Start with basic set close to zero: 0 .. 10
    const BIGGEST_SMALL: u32 = 10;
    let smalls: Vec<_> = (0..BIGGEST_SMALL).map(u64::from).collect();
    // ... and close to MAX: MAX - x
    let word_max = (1u64 << word_bits) - 1;
    let bigs = smalls.iter().map(|x| &word_max - x).collect();
    let one_words = [smalls, bigs].concat();
    // For each of the one word inputs above, create a new one at word i.
    // TODO: Create all possible `modwords` combinations of those
    let multiple_words = (1..modwords)
        .flat_map(|i| {
            one_words
                .iter()
                .map(|x| x << (word_bits * i))
                .collect::<Vec<u64>>()
        })
        .collect();
    let basic_inputs: Vec<u64> = [one_words, multiple_words].concat();

    // Biggest value that will fit in `modwords` words
    // Inputs 'difference from' maximum value
    let diff_max = basic_inputs
        .iter()
        .map(|&x| u64::MAX - x)
        .filter(|&x| x < modulus)
        .collect();
    // Inputs 'difference from' modulus value
    let diff_mod = basic_inputs
        .iter()
        .filter(|&&x| x < modulus && x != 0)
        .map(|&x| modulus - x)
        .collect();
    let basics = basic_inputs
        .into_iter()
        .filter(|&x| x < modulus)
        .collect::<Vec<u64>>();
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
    modulus: u64,
    word_bits: usize,
    op: UnaryOp,
    expected_op: ExpectedOp,
) where
    F: Field,
    UnaryOp: Fn(F) -> F,
    ExpectedOp: Fn(u64) -> u64,
{
    let inputs = test_inputs(modulus, word_bits);
    let expected: Vec<_> = inputs.iter().map(|&x| expected_op(x)).collect();
    let output: Vec<_> = inputs
        .iter()
        .map(|&x| op(F::from_canonical_u64(x)).to_canonical_u64())
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
    modulus: u64,
    word_bits: usize,
    op: BinaryOp,
    expected_op: ExpectedOp,
) where
    F: Field,
    BinaryOp: Fn(F, F) -> F,
    ExpectedOp: Fn(u64, u64) -> u64,
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
            .map(|(&x, &y)| {
                op(F::from_canonical_u64(x), F::from_canonical_u64(y)).to_canonical_u64()
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
            use crate::field::field::Field;
            use std::ops::{Add, Mul, Neg, Sub};

            // Can be 32 or 64; doesn't have to be computer's actual word
            // bits. Choosing 32 gives more tests...
            const WORD_BITS: usize = 32;

            #[test]
            fn arithmetic_addition() {
                let modulus = <$field>::ORDER;
                crate::field::field_testing::run_binaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    <$field>::add,
                    |x, y| {
                        let (z, over) = x.overflowing_add(y);
                        if over {
                            z.overflowing_sub(modulus).0
                        } else if z >= modulus {
                            z - modulus
                        } else {
                            z
                        }
                    },
                )
            }

            #[test]
            fn arithmetic_subtraction() {
                let modulus = <$field>::ORDER;
                crate::field::field_testing::run_binaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    <$field>::sub,
                    |x, y| {
                        if x >= y {
                            x - y
                        } else {
                            &modulus - y + x
                        }
                    },
                )
            }

            #[test]
            fn arithmetic_negation() {
                let modulus = <$field>::ORDER;
                crate::field::field_testing::run_unaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    <$field>::neg,
                    |x| {
                        if x == 0 {
                            0
                        } else {
                            modulus - x
                        }
                    },
                )
            }

            #[test]
            fn arithmetic_multiplication() {
                let modulus = <$field>::ORDER;
                crate::field::field_testing::run_binaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    <$field>::mul,
                    |x, y| ((x as u128) * (y as u128) % (modulus as u128)) as u64,
                )
            }

            #[test]
            fn arithmetic_square() {
                let modulus = <$field>::ORDER;
                crate::field::field_testing::run_unaryop_test_cases(
                    modulus,
                    WORD_BITS,
                    |x: $field| x.square(),
                    |x| ((x as u128) * (x as u128) % (modulus as u128)) as u64,
                )
            }

            #[test]
            fn inversion() {
                let zero = <$field>::ZERO;
                let one = <$field>::ONE;
                let order = <$field>::ORDER;

                assert_eq!(zero.try_inverse(), None);

                for &x in &[1, 2, 3, order - 3, order - 2, order - 1] {
                    let x = <$field>::from_canonical_u64(x);
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
                let order = <$field>::ORDER;

                for &i in &[0, 1, 2, order - 2, order - 1] {
                    let i_f = <$field>::from_canonical_u64(i);
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
                    let power = F::from_canonical_u32(power);
                    if F::is_monomial_permutation(power) {
                        let x = F::rand();
                        assert_eq!(x.exp(power).kth_root(power), x);
                    }
                }
            }

            #[test]
            fn subtraction() {
                type F = $field;

                let (a, b) = (F::from_canonical_u64((F::ORDER + 1) / 2), F::TWO);
                let x = a * b;
                assert_eq!(x, F::ONE);
                assert_eq!(F::ZERO - x, F::NEG_ONE);
            }
        }
    };
}
