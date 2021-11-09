mod common;
mod goldilocks;
mod packed_prime_field;

use packed_prime_field::PackedPrimeField;

use crate::field::goldilocks_field::GoldilocksField;

pub type PackedGoldilocksAVX2 = PackedPrimeField<GoldilocksField>;

#[cfg(test)]
mod tests {
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::field::packed_avx2::common::ReducibleAVX2;
    use crate::field::packed_avx2::packed_prime_field::PackedPrimeField;
    use crate::field::packed_field::PackedField;

    fn test_vals_a<F: ReducibleAVX2>() -> [F; 4] {
        [
            F::from_noncanonical_u64(14479013849828404771),
            F::from_noncanonical_u64(9087029921428221768),
            F::from_noncanonical_u64(2441288194761790662),
            F::from_noncanonical_u64(5646033492608483824),
        ]
    }
    fn test_vals_b<F: ReducibleAVX2>() -> [F; 4] {
        [
            F::from_noncanonical_u64(17891926589593242302),
            F::from_noncanonical_u64(11009798273260028228),
            F::from_noncanonical_u64(2028722748960791447),
            F::from_noncanonical_u64(7929433601095175579),
        ]
    }

    fn test_add<F: ReducibleAVX2>()
    where
        [(); PackedPrimeField::<F>::WIDTH]:,
    {
        let a_arr = test_vals_a::<F>();
        let b_arr = test_vals_b::<F>();

        let packed_a = PackedPrimeField::<F>::from_arr(a_arr);
        let packed_b = PackedPrimeField::<F>::from_arr(b_arr);
        let packed_res = packed_a + packed_b;
        let arr_res = packed_res.to_arr();

        let expected = a_arr.iter().zip(b_arr).map(|(&a, b)| a + b);
        for (exp, res) in expected.zip(arr_res) {
            assert_eq!(res, exp);
        }
    }

    fn test_mul<F: ReducibleAVX2>()
    where
        [(); PackedPrimeField::<F>::WIDTH]:,
    {
        let a_arr = test_vals_a::<F>();
        let b_arr = test_vals_b::<F>();

        let packed_a = PackedPrimeField::<F>::from_arr(a_arr);
        let packed_b = PackedPrimeField::<F>::from_arr(b_arr);
        let packed_res = packed_a * packed_b;
        let arr_res = packed_res.to_arr();

        let expected = a_arr.iter().zip(b_arr).map(|(&a, b)| a * b);
        for (exp, res) in expected.zip(arr_res) {
            assert_eq!(res, exp);
        }
    }

    fn test_square<F: ReducibleAVX2>()
    where
        [(); PackedPrimeField::<F>::WIDTH]:,
    {
        let a_arr = test_vals_a::<F>();

        let packed_a = PackedPrimeField::<F>::from_arr(a_arr);
        let packed_res = packed_a.square();
        let arr_res = packed_res.to_arr();

        let expected = a_arr.iter().map(|&a| a.square());
        for (exp, res) in expected.zip(arr_res) {
            assert_eq!(res, exp);
        }
    }

    fn test_neg<F: ReducibleAVX2>()
    where
        [(); PackedPrimeField::<F>::WIDTH]:,
    {
        let a_arr = test_vals_a::<F>();

        let packed_a = PackedPrimeField::<F>::from_arr(a_arr);
        let packed_res = -packed_a;
        let arr_res = packed_res.to_arr();

        let expected = a_arr.iter().map(|&a| -a);
        for (exp, res) in expected.zip(arr_res) {
            assert_eq!(res, exp);
        }
    }

    fn test_sub<F: ReducibleAVX2>()
    where
        [(); PackedPrimeField::<F>::WIDTH]:,
    {
        let a_arr = test_vals_a::<F>();
        let b_arr = test_vals_b::<F>();

        let packed_a = PackedPrimeField::<F>::from_arr(a_arr);
        let packed_b = PackedPrimeField::<F>::from_arr(b_arr);
        let packed_res = packed_a - packed_b;
        let arr_res = packed_res.to_arr();

        let expected = a_arr.iter().zip(b_arr).map(|(&a, b)| a - b);
        for (exp, res) in expected.zip(arr_res) {
            assert_eq!(res, exp);
        }
    }

    fn test_interleave_is_involution<F: ReducibleAVX2>()
    where
        [(); PackedPrimeField::<F>::WIDTH]:,
    {
        let a_arr = test_vals_a::<F>();
        let b_arr = test_vals_b::<F>();

        let packed_a = PackedPrimeField::<F>::from_arr(a_arr);
        let packed_b = PackedPrimeField::<F>::from_arr(b_arr);
        {
            // Interleave, then deinterleave.
            let (x, y) = packed_a.interleave(packed_b, 0);
            let (res_a, res_b) = x.interleave(y, 0);
            assert_eq!(res_a.to_arr(), a_arr);
            assert_eq!(res_b.to_arr(), b_arr);
        }
        {
            let (x, y) = packed_a.interleave(packed_b, 1);
            let (res_a, res_b) = x.interleave(y, 1);
            assert_eq!(res_a.to_arr(), a_arr);
            assert_eq!(res_b.to_arr(), b_arr);
        }
    }

    fn test_interleave<F: ReducibleAVX2>()
    where
        [(); PackedPrimeField::<F>::WIDTH]:,
    {
        let in_a: [F; 4] = [
            F::from_noncanonical_u64(00),
            F::from_noncanonical_u64(01),
            F::from_noncanonical_u64(02),
            F::from_noncanonical_u64(03),
        ];
        let in_b: [F; 4] = [
            F::from_noncanonical_u64(10),
            F::from_noncanonical_u64(11),
            F::from_noncanonical_u64(12),
            F::from_noncanonical_u64(13),
        ];
        let int0_a: [F; 4] = [
            F::from_noncanonical_u64(00),
            F::from_noncanonical_u64(10),
            F::from_noncanonical_u64(02),
            F::from_noncanonical_u64(12),
        ];
        let int0_b: [F; 4] = [
            F::from_noncanonical_u64(01),
            F::from_noncanonical_u64(11),
            F::from_noncanonical_u64(03),
            F::from_noncanonical_u64(13),
        ];
        let int1_a: [F; 4] = [
            F::from_noncanonical_u64(00),
            F::from_noncanonical_u64(01),
            F::from_noncanonical_u64(10),
            F::from_noncanonical_u64(11),
        ];
        let int1_b: [F; 4] = [
            F::from_noncanonical_u64(02),
            F::from_noncanonical_u64(03),
            F::from_noncanonical_u64(12),
            F::from_noncanonical_u64(13),
        ];

        let packed_a = PackedPrimeField::<F>::from_arr(in_a);
        let packed_b = PackedPrimeField::<F>::from_arr(in_b);
        {
            let (x0, y0) = packed_a.interleave(packed_b, 0);
            assert_eq!(x0.to_arr(), int0_a);
            assert_eq!(y0.to_arr(), int0_b);
        }
        {
            let (x1, y1) = packed_a.interleave(packed_b, 1);
            assert_eq!(x1.to_arr(), int1_a);
            assert_eq!(y1.to_arr(), int1_b);
        }
    }

    #[test]
    fn test_add_goldilocks() {
        test_add::<GoldilocksField>();
    }
    #[test]
    fn test_mul_goldilocks() {
        test_mul::<GoldilocksField>();
    }
    #[test]
    fn test_square_goldilocks() {
        test_square::<GoldilocksField>();
    }
    #[test]
    fn test_neg_goldilocks() {
        test_neg::<GoldilocksField>();
    }
    #[test]
    fn test_sub_goldilocks() {
        test_sub::<GoldilocksField>();
    }
    #[test]
    fn test_interleave_is_involution_goldilocks() {
        test_interleave_is_involution::<GoldilocksField>();
    }
    #[test]
    fn test_interleave_goldilocks() {
        test_interleave::<GoldilocksField>();
    }
}
