use core::fmt;
use std::error::Error;
use std::marker::PhantomData;

use ff::{Field as ff_Field, PrimeField};
use num::BigUint;
use crate::field::extension::quadratic::QuadraticExtension;
use crate::field::goldilocks_field::GoldilocksField;
use crate::field::types::Field;
use crate::hash::hash_types::RichField;
use crate::hash::poseidon::{PoseidonHash, PoseidonPermutation};
use crate::plonk::config::{GenericConfig, GenericHashOut, Hasher};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::plonk::wrapper::poseidon_bn128::{permution, GOLDILOCKS_ELEMENTS, RATE};
use crate::plonk::wrapper::utils::{Fr, FrRepr};

/// Configuration using Poseidon BN128 over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize)]
pub struct PoseidonBN128GoldilocksConfig;
impl GenericConfig<2> for PoseidonBN128GoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = PoseidonBN128Hash;
    type InnerHasher = PoseidonHash;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PoseidonBN128HashOut<F: Field> {
    pub value: Fr,
    _phantom: PhantomData<F>,
}

fn hash_out_to_bytes<F: Field>(hash: PoseidonBN128HashOut<F>) -> Vec<u8> {
    let binding = hash.value.to_repr();
    let limbs = binding.as_ref();
    limbs.to_vec()
}

impl<F: RichField> GenericHashOut<F> for PoseidonBN128HashOut<F> {
    fn to_bytes(&self) -> Vec<u8> {
        hash_out_to_bytes(*self)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let sized_bytes: [u8; 32] = bytes.try_into().unwrap();
        let fr_repr = FrRepr(sized_bytes);
        let fr = Fr::from_repr(fr_repr).unwrap();

        Self {
            value: fr,
            _phantom: PhantomData,
        }
    }

    fn to_vec(&self) -> Vec<F> {
        let bytes = hash_out_to_bytes(*self);
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

impl<F: RichField> Serialize for PoseidonBN128HashOut<F> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Output the hash as a bigint string.
        let binding = self.value.to_repr();
        let limbs = binding.as_ref();

        let big_int = BigUint::from_bytes_le(limbs);
        serializer.serialize_str(big_int.to_str_radix(10).as_str())
    }
}

impl<'de, F: RichField> Deserialize<'de> for PoseidonBN128HashOut<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PoseidonBN128HashOutVisitor;

        impl<'a> Visitor<'a> for PoseidonBN128HashOutVisitor {
            type Value = String;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string with integer value within BN128 scalar field")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            {
                Ok(v.to_string())
            }
        }

        let deserialized_str = deserializer
            .deserialize_str(PoseidonBN128HashOutVisitor)
            .unwrap();
        let big_int = BigUint::parse_bytes(deserialized_str.as_bytes(), 10).unwrap();

        let mut bytes = big_int.to_bytes_le();
        for _i in bytes.len()..32 {
            bytes.push(0);
        }

        let sized_bytes: [u8; 32] = bytes.try_into().unwrap();
        let fr_repr = FrRepr(sized_bytes);
        let fr = Fr::from_repr(fr_repr).unwrap();

        Ok(Self {
            value: fr,
            _phantom: PhantomData,
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PoseidonBN128Hash;
impl<F: RichField> Hasher<F> for PoseidonBN128Hash {
    const HASH_SIZE: usize = 32; // Hash output is 4 limbs of u64
    type Hash = PoseidonBN128HashOut<F>;
    type Permutation = PoseidonPermutation<F>;

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        let mut state = [Fr::ZERO; 4];

        state[0] = Fr::ZERO;
        for rate_chunk in input.chunks(RATE * 3) {
            for (j, bn128_chunk) in rate_chunk.chunks(3).enumerate() {
                let mut bytes = bn128_chunk[0].to_canonical_u64().to_le_bytes().to_vec();

                for gl_element in bn128_chunk.iter().skip(1) {
                    let chunk_bytes = gl_element.to_canonical_u64().to_le_bytes();
                    bytes.extend_from_slice(&chunk_bytes);
                }

                for _i in bytes.len()..32 {
                    bytes.push(0);
                }

                let sized_bytes: [u8; 32] = bytes.try_into().unwrap();
                let fr_repr = FrRepr(sized_bytes);
                state[j + 1] = Fr::from_repr(fr_repr).unwrap();
            }
            permution(&mut state);
        }

        PoseidonBN128HashOut {
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
        let mut state = [Fr::ZERO, Fr::ZERO, left.value, right.value];
        permution(&mut state);

        PoseidonBN128HashOut {
            value: state[0],
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_byte_methods() {
        type F = GoldilocksField;

        let fr = Fr::from_str_vartime(
            "11575173631114898451293296430061690731976535592475236587664058405912382527658",
        )
        .unwrap();
        let hash = PoseidonBN128HashOut::<F> {
            value: fr,
            _phantom: PhantomData,
        };

        let bytes = hash.to_bytes();

        let hash_from_bytes = PoseidonBN128HashOut::<F>::from_bytes(&bytes);
        assert_eq!(hash, hash_from_bytes);
    }

    #[test]
    fn test_serialization() {
        let fr = Fr::from_str_vartime(
            "11575173631114898451293296430061690731976535592475236587664058405912382527658",
        )
        .unwrap();
        let hash = PoseidonBN128HashOut::<GoldilocksField> {
            value: fr,
            _phantom: PhantomData,
        };

        let serialized = serde_json::to_string(&hash).unwrap();
        let deserialized: PoseidonBN128HashOut<GoldilocksField> =
            serde_json::from_str(&serialized).unwrap();
        assert_eq!(hash, deserialized);
    }
}
