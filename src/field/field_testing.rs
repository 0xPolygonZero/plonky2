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
    let expected: Vec<_> = inputs.iter()
        .map(|&x| expected_op(x))
        .collect();
    let output: Vec<_> = inputs
        .iter()
        .map(|&x| op(F::from_canonical_u64(x)).to_canonical_u64())
        .collect();
    // Compare expected outputs with actual outputs
    for i in 0..inputs.len() {
        assert_eq!(output[i], expected[i],
                   "Expected {}, got {} for input {}",
                   expected[i], output[i], inputs[i]);
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
        let shifted_inputs: Vec<_> = inputs.iter()
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

        let output: Vec<_> = inputs.iter().zip(shifted_inputs.clone()).map(|(&x, &y)| {
            op(F::from_canonical_u64(x), F::from_canonical_u64(y)).to_canonical_u64()
        }).collect();

        // Compare expected outputs with actual outputs
        for i in 0..inputs.len() {
            assert_eq!(output[i], expected[i],
                       "On inputs {} . {}, expected {} but got {}",
                       inputs[i], shifted_inputs[i], expected[i], output[i]);
        }
    }
}

#[macro_export]
macro_rules! test_arithmetic {
    ($field:ty) => {
        mod arithmetic {
            use crate::{Field};
            use std::ops::{Add, Mul, Neg, Sub};

            // Can be 32 or 64; doesn't have to be computer's actual word
            // bits. Choosing 32 gives more tests...
            const WORD_BITS: usize = 32;

            #[test]
            fn arithmetic_addition() {
                let modulus = <$field>::ORDER;
                crate::field::field_testing::run_binaryop_test_cases(modulus, WORD_BITS, <$field>::add, |x, y| {
                    let (z, over) = x.overflowing_add(y);
                    if over {
                        z.overflowing_sub(modulus).0
                    } else if z >= modulus {
                        z - modulus
                    } else {
                        z
                    }
                })
            }

            #[test]
            fn arithmetic_subtraction() {
                let modulus = <$field>::ORDER;
                crate::field::field_testing::run_binaryop_test_cases(modulus, WORD_BITS, <$field>::sub, |x, y| {
                    if x >= y {
                        x - y
                    } else {
                        &modulus - y + x
                    }
                })
            }

            #[test]
            fn arithmetic_negation() {
                let modulus = <$field>::ORDER;
                crate::field::field_testing::run_unaryop_test_cases(modulus, WORD_BITS, <$field>::neg, |x| {
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
                crate::field::field_testing::run_binaryop_test_cases(modulus, WORD_BITS, <$field>::mul, |x, y| {
                    ((x as u128) * (y as u128) % (modulus as u128)) as u64
                })
            }

            #[test]
            fn arithmetic_square() {
                let modulus = <$field>::ORDER;
                crate::field::field_testing::run_unaryop_test_cases(
                    modulus, WORD_BITS,
                    |x: $field| x.square(),
                    |x| ((x as u128) * (x as u128) % (modulus as u128)) as u64)
            }

            // #[test]
            // #[ignore]
            // fn arithmetic_division() {
            //     // This test takes ages to finish so is #[ignore]d by default.
            //     // TODO: Re-enable and reimplement when
            //     // https://github.com/rust-num/num-bigint/issues/60 is finally resolved.
            //     let modulus = <$field>::ORDER;
            //     crate::field::field_testing::run_binaryop_test_cases(
            //         modulus,
            //         WORD_BITS,
            //         // Need to help the compiler infer the type of y here
            //         |x: $field, y: $field| {
            //             // TODO: Work out how to check that div() panics
            //             // appropriately when given a zero divisor.
            //             if !y.is_zero() {
            //                 <$field>::div(x, y)
            //             } else {
            //                 <$field>::ZERO
            //             }
            //         },
            //         |x, y| {
            //             // yinv = y^-1 (mod modulus)
            //             let exp = modulus - 2u64;
            //             let yinv = y.modpow(exp, modulus);
            //             // returns 0 if y was 0
            //             x * yinv % modulus
            //         },
            //     )
            // }
        }
    };
}
