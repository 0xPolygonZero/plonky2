use core::fmt;
use std::error::Error;
use std::marker::PhantomData;

use num::bigint::BigUint;
use plonky2_field::types::PrimeField;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::field::bn254::Bn254Field;
use crate::field::types::Field;
use crate::hash::hash_types::RichField;
use crate::hash::poseidon::PoseidonPermutation;
use crate::hash::poseidon_bn254::{permutation, GOLDILOCKS_ELEMENTS, RATE};
use crate::plonk::config::{GenericHashOut, Hasher};

pub const NUM_HASH_OUT_ELTS: usize = 1;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PoseidonBn254HashOut<F: Field> {
    value: Bn254Field,
    _phantom: PhantomData<F>,
}

impl<F: RichField> GenericHashOut<F> for PoseidonBn254HashOut<F> {
    fn to_bytes(&self) -> Vec<u8> {
        self.value.to_canonical_biguint().to_bytes_le()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let sized_bytes: [u8; 32] = bytes.try_into().unwrap();
        PoseidonBn254HashOut {
            value: Bn254Field::from_noncanonical_biguint(BigUint::from_bytes_le(
                &sized_bytes, // bytes.first_chunk::<32>().unwrap(),
            )),
            _phantom: PhantomData,
        }
    }

    fn to_vec(&self) -> Vec<F> {
        let bytes = self.to_bytes();
        bytes
            // Chunks of 7 bytes since 8 bytes would allow collisions.
            .chunks(7)
            .map(|bytes| {
                let mut arr = [0; 8];
                arr[..bytes.len()].copy_from_slice(bytes);
                F::from_canonical_u64(u64::from_le_bytes(arr))
            })
            .collect()
    }
}

impl<F: RichField> Serialize for PoseidonBn254HashOut<F> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Output the hash as a bigint string.
        let bytes = self.to_bytes();

        let big_uint = BigUint::from_bytes_le(&bytes);
        serializer.serialize_str(big_uint.to_str_radix(10).as_str())
    }
}

impl<'de, F: RichField> Deserialize<'de> for PoseidonBn254HashOut<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PoseidonBn254HashOutVisitor;

        impl<'a> Visitor<'a> for PoseidonBn254HashOutVisitor {
            type Value = String;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with integer value within Bn254 scalar field")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(v.to_string())
            }
        }

        let deserialized_str = deserializer
            .deserialize_str(PoseidonBn254HashOutVisitor)
            .unwrap();
        let big_uint = BigUint::parse_bytes(deserialized_str.as_bytes(), 10).unwrap();

        let mut bytes = big_uint.to_bytes_le();
        for _i in bytes.len()..32 {
            bytes.push(0);
        }

        Ok(PoseidonBn254HashOut::from_bytes(&bytes))
    }
}

/// Poseidon hash function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PoseidonBn254Hash;
impl<F: RichField> Hasher<F> for PoseidonBn254Hash {
    const HASH_SIZE: usize = 32; // Hash output is 4 limbs of u64
    type Hash = PoseidonBn254HashOut<F>;
    type Permutation = PoseidonPermutation<F>;

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        let mut state = [Bn254Field::ZERO; 4];

        state[0] = Bn254Field::ZERO;
        for rate_chunk in input.chunks(RATE * 3) {
            for (j, bn254_chunk) in rate_chunk.chunks(3).enumerate() {
                let mut bytes = bn254_chunk[0].to_canonical_u64().to_le_bytes().to_vec();

                for gl_element in bn254_chunk.iter().skip(1) {
                    let chunk_bytes = gl_element.to_canonical_u64().to_le_bytes();
                    bytes.extend_from_slice(&chunk_bytes);
                }

                for _i in bytes.len()..32 {
                    bytes.push(0);
                }

                let sized_bytes: [u8; 32] = bytes.try_into().unwrap();
                state[j + 1] =
                Bn254Field::from_noncanonical_biguint(BigUint::from_bytes_le(&sized_bytes));
            }
            permutation(&mut state);
        }

        PoseidonBn254HashOut {
            value: state[0],
            _phantom: PhantomData,
        }
    }

    fn hash_pad(input: &[F]) -> Self::Hash {
        let mut padded_input = input.to_vec();
        padded_input.push(F::ONE);
        while (padded_input.len() + 1) % (RATE * GOLDILOCKS_ELEMENTS) != 0 {
            padded_input.push(F::ZERO);
        }
        padded_input.push(F::ONE);
        Self::hash_no_pad(&padded_input)
    }

    fn hash_or_noop(inputs: &[F]) -> Self::Hash {
        if inputs.len() * 8 <= GOLDILOCKS_ELEMENTS * 8 {
            let mut inputs_bytes = vec![0u8; 32];
            for i in 0..inputs.len() {
                inputs_bytes[i * 8..(i + 1) * 8]
                    .copy_from_slice(&inputs[i].to_canonical_u64().to_le_bytes());
            }
            Self::Hash::from_bytes(&inputs_bytes)
        } else {
            Self::hash_no_pad(inputs)
        }
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        let mut state = [Bn254Field::ZERO, Bn254Field::ZERO, left.value, right.value];
        permutation(&mut state);
        PoseidonBn254HashOut {
            value: state[0],
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::field::bn254::Bn254Field;
    use crate::field::goldilocks_field::GoldilocksField;

    #[test]
    fn test_byte_methods() {
        type F = GoldilocksField;

        let fr = Bn254Field::from_noncanonical_str(
            "11575173631114898451293296430061690731976535592475236587664058405912382527658",
        );
        let hash = PoseidonBn254HashOut::<F> {
            value: fr,
            _phantom: PhantomData,
        };

        let bytes = hash.to_bytes();

        let hash_from_bytes = PoseidonBn254HashOut::<F>::from_bytes(&bytes);
        assert_eq!(hash, hash_from_bytes);
    }

    #[test]
    fn test_serialization() {
        let fr = Bn254Field::from_noncanonical_str(
                "11575173631114898451293296430061690731976535592475236587664058405912382527658",
        );
        let hash = PoseidonBn254HashOut::<GoldilocksField> {
            value: fr,
            _phantom: PhantomData,
        };

        let serialized = serde_json::to_string(&hash).unwrap();
        let deserialized: PoseidonBn254HashOut<GoldilocksField> =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(hash, deserialized);
    }
}
