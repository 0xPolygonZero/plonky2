use crate::field::extension_field::Extendable;
use crate::field::extension_field::Frobenius;
use crate::field::field_types::Field;

#[macro_export]
macro_rules! test_field_arithmetic {
    ($field:ty) => {
        mod field_arithmetic {
            use num::bigint::BigUint;
            use rand::Rng;

            use crate::field::field_types::Field;

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
                type F = $field;

                for x in [F::ZERO, F::ONE, F::TWO, F::NEG_ONE] {
                    assert_eq!(x + -x, F::ZERO);
                }
            }

            #[test]
            fn exponentiation() {
                type F = $field;

                assert_eq!(F::ZERO.exp_u64(0), <F>::ONE);
                assert_eq!(F::ONE.exp_u64(0), <F>::ONE);
                assert_eq!(F::TWO.exp_u64(0), <F>::ONE);

                assert_eq!(F::ZERO.exp_u64(1), <F>::ZERO);
                assert_eq!(F::ONE.exp_u64(1), <F>::ONE);
                assert_eq!(F::TWO.exp_u64(1), <F>::TWO);

                assert_eq!(F::ZERO.kth_root_u64(1), <F>::ZERO);
                assert_eq!(F::ONE.kth_root_u64(1), <F>::ONE);
                assert_eq!(F::TWO.kth_root_u64(1), <F>::TWO);

                for power in 1..10 {
                    if F::is_monomial_permutation_u64(power) {
                        let x = F::rand();
                        assert_eq!(x.exp_u64(power).kth_root_u64(power), x);
                    }
                }
            }

            #[test]
            fn exponentiation_large() {
                type F = $field;

                let mut rng = rand::thread_rng();

                let base = F::rand();
                let pow = BigUint::from(rng.gen::<u64>());
                let cycles = rng.gen::<u32>();
                let mul_group_order = F::order() - 1u32;
                let big_pow = &pow + &mul_group_order * cycles;
                let big_pow_wrong = &pow + &mul_group_order * cycles + 1u32;

                assert_eq!(base.exp_biguint(&pow), base.exp_biguint(&big_pow));
                assert_ne!(base.exp_biguint(&pow), base.exp_biguint(&big_pow_wrong));
            }

            #[test]
            fn inverse_2exp() {
                // Just check consistency with try_inverse()
                type F = $field;

                let v = <F as Field>::PrimeField::TWO_ADICITY;

                for e in [0, 1, 2, 3, 4, v - 2, v - 1, v, v + 1, v + 2, 123 * v] {
                    let x = F::TWO.exp_u64(e as u64).inverse();
                    let y = F::inverse_2exp(e);
                    assert_eq!(x, y);
                }
            }
        }
    };
}

pub(crate) fn test_add_neg_sub_mul<BF: Extendable<D>, const D: usize>() {
    let x = BF::Extension::rand();
    let y = BF::Extension::rand();
    let z = BF::Extension::rand();
    assert_eq!(x + (-x), BF::Extension::ZERO);
    assert_eq!(-x, BF::Extension::ZERO - x);
    assert_eq!(x + x, x * BF::Extension::TWO);
    assert_eq!(x * (-x), -x.square());
    assert_eq!(x + y, y + x);
    assert_eq!(x * y, y * x);
    assert_eq!(x * (y * z), (x * y) * z);
    assert_eq!(x - (y + z), (x - y) - z);
    assert_eq!((x + y) - z, x + (y - z));
    assert_eq!(x * (y + z), x * y + x * z);
}

pub(crate) fn test_inv_div<BF: Extendable<D>, const D: usize>() {
    let x = BF::Extension::rand();
    let y = BF::Extension::rand();
    let z = BF::Extension::rand();
    assert_eq!(x * x.inverse(), BF::Extension::ONE);
    assert_eq!(x.inverse() * x, BF::Extension::ONE);
    assert_eq!(x.square().inverse(), x.inverse().square());
    assert_eq!((x / y) * y, x);
    assert_eq!(x / (y * z), (x / y) / z);
    assert_eq!((x * y) / z, x * (y / z));
}

pub(crate) fn test_frobenius<BF: Extendable<D>, const D: usize>() {
    let x = BF::Extension::rand();
    assert_eq!(x.exp_biguint(&BF::order()), x.frobenius());
    for count in 2..D {
        assert_eq!(
            x.repeated_frobenius(count),
            (0..count).fold(x, |acc, _| acc.frobenius())
        );
    }
}

pub(crate) fn test_field_order<BF: Extendable<D>, const D: usize>() {
    let x = BF::Extension::rand();
    assert_eq!(
        x.exp_biguint(&(BF::Extension::order() - 1u8)),
        BF::Extension::ONE
    );
}

pub(crate) fn test_power_of_two_gen<BF: Extendable<D>, const D: usize>() {
    assert_eq!(
        BF::Extension::MULTIPLICATIVE_GROUP_GENERATOR
            .exp_biguint(&(BF::Extension::order() >> BF::Extension::TWO_ADICITY)),
        BF::Extension::POWER_OF_TWO_GENERATOR,
    );
    assert_eq!(
        BF::Extension::POWER_OF_TWO_GENERATOR
            .exp_u64(1 << (BF::Extension::TWO_ADICITY - BF::TWO_ADICITY)),
        BF::POWER_OF_TWO_GENERATOR.into()
    );
}

#[macro_export]
macro_rules! test_field_extension {
    ($field:ty, $d:expr) => {
        mod field_extension {
            #[test]
            fn test_add_neg_sub_mul() {
                crate::field::field_testing::test_add_neg_sub_mul::<$field, $d>();
            }
            #[test]
            fn test_inv_div() {
                crate::field::field_testing::test_inv_div::<$field, $d>();
            }
            #[test]
            fn test_frobenius() {
                crate::field::field_testing::test_frobenius::<$field, $d>();
            }
            #[test]
            fn test_field_order() {
                crate::field::field_testing::test_field_order::<$field, $d>();
            }
            #[test]
            fn test_power_of_two_gen() {
                crate::field::field_testing::test_power_of_two_gen::<$field, $d>();
            }
        }
    };
}
