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
