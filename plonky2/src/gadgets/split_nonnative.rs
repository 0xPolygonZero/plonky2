use std::marker::PhantomData;

use itertools::Itertools;
use plonky2_field::extension_field::Extendable;
use plonky2_field::field_types::Field;

use crate::gadgets::arithmetic_u32::U32Target;
use crate::gadgets::biguint::BigUintTarget;
use crate::gadgets::nonnative::NonNativeTarget;
use crate::hash::hash_types::RichField;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn split_u32_to_4_bit_limbs(&mut self, val: U32Target) -> Vec<Target> {
        let two_bit_limbs = self.split_le_base::<4>(val.0, 16);
        let four = self.constant(F::from_canonical_usize(4));
        let combined_limbs = two_bit_limbs
            .iter()
            .tuples()
            .map(|(&a, &b)| self.mul_add(b, four, a))
            .collect();

        combined_limbs
    }

    pub fn split_nonnative_to_4_bit_limbs<FF: Field>(
        &mut self,
        val: &NonNativeTarget<FF>,
    ) -> Vec<Target> {
        val.value
            .limbs
            .iter()
            .flat_map(|&l| self.split_u32_to_4_bit_limbs(l))
            .collect()
    }

    pub fn split_biguint_to_2_bit_limbs(&mut self, val: &BigUintTarget) -> Vec<Target> {
        val.limbs
            .iter()
            .flat_map(|&l| self.split_le_base::<4>(l.0, 16))
            .collect()
    }

    // Note: assumes its inputs are 4-bit limbs, and does not range-check.
    pub fn recombine_nonnative_4_bit_limbs<FF: Field>(
        &mut self,
        limbs: Vec<Target>,
    ) -> NonNativeTarget<FF> {
        let base = self.constant_u32(1 << 4);
        let u32_limbs = limbs
            .chunks(8)
            .map(|chunk| {
                let mut combined_chunk = self.zero_u32();
                for i in (0..8).rev() {
                    let (low, _high) = self.mul_add_u32(combined_chunk, base, U32Target(chunk[i]));
                    combined_chunk = low;
                }
                combined_chunk
            })
            .collect();

        NonNativeTarget {
            value: BigUintTarget { limbs: u32_limbs },
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::field_types::Field;
    use plonky2_field::secp256k1_scalar::Secp256K1Scalar;

    use crate::gadgets::nonnative::NonNativeTarget;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_split_nonnative() -> Result<()> {
        type FF = Secp256K1Scalar;
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_ecc_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = FF::rand();
        let x_target = builder.constant_nonnative(x);
        let split = builder.split_nonnative_to_4_bit_limbs(&x_target);
        let combined: NonNativeTarget<Secp256K1Scalar> =
            builder.recombine_nonnative_4_bit_limbs(split);
        builder.connect_nonnative(&x_target, &combined);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        verify(proof, &data.verifier_only, &data.common)
    }
}
