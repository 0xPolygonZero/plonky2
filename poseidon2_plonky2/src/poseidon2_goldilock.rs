//! Implementations for Poseidon2 over Goldilocks field of widths 8 and 12.
//!
//! These contents of the implementations *must* be generated using the
//! `poseidon_constants.sage` script in the `mir-protocol/hash-constants`
//! repository.

use plonky2::field::{goldilocks_field::GoldilocksField, extension::quadratic::QuadraticExtension};
use crate::poseidon2_hash::{Poseidon2, Poseidon2Hash};
use plonky2::plonk::config::GenericConfig;

#[rustfmt::skip]
impl Poseidon2 for GoldilocksField {
    // We only need INTERNAL_MATRIX_DIAG_M_1 here, specifying the diagonal - 1 of the internal matrix

    const INTERNAL_MATRIX_DIAG_M_1: [u64; 12]  = [
        0xcf6f77ac16722af9, 0x3fd4c0d74672aebc, 0x9b72bf1c1c3d08a8, 0xe4940f84b71e4ac2,
        0x61b27b077118bc72, 0x2efd8379b8e661e2, 0x858edcf353df0341, 0x2d9c20affb5c4516,
        0x5120143f0695defb, 0x62fc898ae34a5c5b, 0xa3d9560c99123ed2, 0x98fd739d8e7fc933,
    ];

    // #[cfg(all(target_arch="aarch64", target_feature="neon"))]
    // #[inline(always)]
    // fn sbox_layer(state: &mut [Self; 12]) {
    //     unsafe {
    //         crate::hash::arch::aarch64::poseidon_goldilocks_neon::sbox_layer(state);
    //     }
    // }

    // #[cfg(all(target_arch="aarch64", target_feature="neon"))]
    // #[inline(always)]
    // fn mds_layer(state: &[Self; 12]) -> [Self; 12] {
    //     unsafe {
    //         crate::hash::arch::aarch64::poseidon_goldilocks_neon::mds_layer(state)
    //     }
    // }
}

/// Configuration using Poseidon2 over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Poseidon2GoldilocksConfig;
impl GenericConfig<2> for Poseidon2GoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = Poseidon2Hash;
    type InnerHasher = Poseidon2Hash;
}

#[cfg(test)]
mod tests {
    use log::info;
    use plonky2::field::extension::Extendable;
    use plonky2::field::goldilocks_field::{GoldilocksField as F, GoldilocksField};
    use plonky2::hash::hash_types::RichField;
    use plonky2::hash::poseidon::PoseidonHash;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher, PoseidonGoldilocksConfig};
    use crate::poseidon2_goldilock::Poseidon2GoldilocksConfig;
    use crate::poseidon2_hash::{Poseidon2, Poseidon2Hash};
    use crate::poseidon2_hash::test_helpers::{check_consistency, check_test_vectors, prove_circuit_with_poseidon2, recursive_proof};
    use rstest::rstest;
    use serial_test::serial;
    use serde_cbor;

    #[test]
    fn test_vectors() {
        // Test inputs are:
        // 1. range 0..WIDTH

        #[rustfmt::skip]
            let test_vectors12: Vec<([u64; 12], [u64; 12])> = vec![
            ([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, ],
             [0xed3dbcc4ff1e8d33, 0xfb85eac6ac91a150, 0xd41e1e237ed3e2ef, 0x5e289bf0a4c11897,
                 0x4398b20f93e3ba6b, 0x5659a48ffaf2901d, 0xe44d81e89a88f8ae, 0x08efdb285f8c3dbc,
                 0x294ab7503297850e, 0xa11c61f4870b9904, 0xa6855c112cc08968, 0x17c6d53d2fb3e8c1, ]),
        ];

        check_test_vectors::<F>(test_vectors12);
    }

    #[test]
    fn consistency() {
        check_consistency::<F>();
    }

    #[test]
    fn test_circuit_with_poseidon2() {
        const D: usize = 2;
        type C = Poseidon2GoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();

        let (cd, proof) = prove_circuit_with_poseidon2::<F,C,D,_>(
            config,
            1024,
            Poseidon2Hash{},
            false
        ).unwrap();

        cd.verify(proof).unwrap();
    }

    const D: usize = 2;
    #[rstest]
    #[case::poseidon(PoseidonGoldilocksConfig{})]
    #[case::poseidon2(Poseidon2GoldilocksConfig{})]
    #[serial]
    fn compare_proof_generation_with_poseidon<C: GenericConfig<D, F = GoldilocksField>>(
        #[case] _c: C,
    ) {
        let _ = env_logger::try_init();

        let config = CircuitConfig::standard_recursion_config();

        let (cd, proof) = prove_circuit_with_poseidon2::<C::F,C,D,Poseidon2Hash>(
            config,
            4096,
            Poseidon2Hash{},
            true
        ).unwrap();

        let proof_bytes = serde_cbor::to_vec(&proof).unwrap();
        info!("proof size: {}", proof_bytes.len());

        cd.verify(proof).unwrap();

        assert_eq!(cd.common.degree_bits(), 14);
    }
    #[rstest]
    #[case::poseidon(PoseidonHash{})]
    #[case::poseidon2(Poseidon2Hash{})]
    #[serial]
    fn compare_circuits_with_poseidon<
        H: Hasher<GoldilocksField>+ AlgebraicHasher<GoldilocksField>
    >(
        #[case] hasher: H,
    ) {
        let _ = env_logger::try_init();
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;

        let config = CircuitConfig::standard_recursion_config();

        let (cd, proof) = prove_circuit_with_poseidon2::<GoldilocksField,C,D,H>(
            config,
            4096,
            hasher,
            true
        ).unwrap();

        let proof_bytes = serde_cbor::to_vec(&proof).unwrap();
        info!("proof size: {}", proof_bytes.len());

        cd.verify(proof).unwrap();

        assert_eq!(cd.common.degree_bits(), 14);
    }

    #[rstest]
    #[serial]
    fn test_recursive_circuit_with_poseidon2<
        F: RichField + Poseidon2 + Extendable<D>,
        C: GenericConfig<D, F=F>,
        InnerC: GenericConfig<D, F = F>,
        const D: usize,
    >(
        #[values(PoseidonGoldilocksConfig{}, Poseidon2GoldilocksConfig{})] _c: C,
        #[values(PoseidonGoldilocksConfig{}, Poseidon2GoldilocksConfig{})] _inner: InnerC,
    )
        where
            InnerC::Hasher: AlgebraicHasher<F>,
    {

        let config = CircuitConfig::standard_recursion_config();

        let (cd, proof) = prove_circuit_with_poseidon2::<F,InnerC,D,_>(
            config,
            1024,
            Poseidon2Hash{},
            false
        ).unwrap();

        println!("base proof generated");

        let (rec_cd, rec_proof) = recursive_proof::<F,C,InnerC,D>(
            proof,
            &cd,
            &cd.common.config,
        ).unwrap();

        println!("recursive proof generated");

        rec_cd.verify(rec_proof).unwrap();

        assert_eq!(rec_cd.common.degree_bits(), 12);
    }
}
