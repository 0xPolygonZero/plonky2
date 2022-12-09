use alloc::vec::Vec;

use anyhow::ensure;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::field::goldilocks_field::GoldilocksField;
use crate::field::types::{Field, PrimeField64, Sample};
use crate::hash::poseidon::Poseidon;
use crate::iop::target::Target;
use crate::plonk::config::GenericHashOut;

/// A prime order field with the features we need to use it as a base field in our argument system.
pub trait RichField: PrimeField64 + Poseidon {}

impl RichField for GoldilocksField {}

/// Represents a ~256 bit hash output.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct HashOut<F: Field> {
    pub elements: [F; 4],
}

impl<F: Field> HashOut<F> {
    pub const ZERO: Self = Self {
        elements: [F::ZERO; 4],
    };

    // TODO: Switch to a TryFrom impl.
    pub fn from_vec(elements: Vec<F>) -> Self {
        debug_assert!(elements.len() == 4);
        Self {
            elements: elements.try_into().unwrap(),
        }
    }

    pub fn from_partial(elements_in: &[F]) -> Self {
        let mut elements = [F::ZERO; 4];
        elements[0..elements_in.len()].copy_from_slice(elements_in);
        Self { elements }
    }
}

impl<F: Field> From<[F; 4]> for HashOut<F> {
    fn from(elements: [F; 4]) -> Self {
        Self { elements }
    }
}

impl<F: Field> TryFrom<&[F]> for HashOut<F> {
    type Error = anyhow::Error;

    fn try_from(elements: &[F]) -> Result<Self, Self::Error> {
        ensure!(elements.len() == 4);
        Ok(Self {
            elements: elements.try_into().unwrap(),
        })
    }
}

impl<F> Sample for HashOut<F>
where
    F: Field,
{
    #[inline]
    fn sample<R>(rng: &mut R) -> Self
    where
        R: rand::RngCore + ?Sized,
    {
        Self {
            elements: [
                F::sample(rng),
                F::sample(rng),
                F::sample(rng),
                F::sample(rng),
            ],
        }
    }
}

impl<F: RichField> GenericHashOut<F> for HashOut<F> {
    fn to_bytes(&self) -> Vec<u8> {
        self.elements
            .into_iter()
            .flat_map(|x| x.to_canonical_u64().to_le_bytes())
            .collect()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        HashOut {
            elements: bytes
                .chunks(8)
                .take(4)
                .map(|x| F::from_canonical_u64(u64::from_le_bytes(x.try_into().unwrap())))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    fn to_vec(&self) -> Vec<F> {
        self.elements.to_vec()
    }
}

impl<F: Field> Default for HashOut<F> {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Represents a ~256 bit hash output.
#[derive(Copy, Clone, Debug)]
pub struct HashOutTarget {
    pub elements: [Target; 4],
}

impl HashOutTarget {
    // TODO: Switch to a TryFrom impl.
    pub fn from_vec(elements: Vec<Target>) -> Self {
        debug_assert!(elements.len() == 4);
        Self {
            elements: elements.try_into().unwrap(),
        }
    }

    pub fn from_partial(elements_in: &[Target], zero: Target) -> Self {
        let mut elements = [zero; 4];
        elements[0..elements_in.len()].copy_from_slice(elements_in);
        Self { elements }
    }
}

impl From<[Target; 4]> for HashOutTarget {
    fn from(elements: [Target; 4]) -> Self {
        Self { elements }
    }
}

impl TryFrom<&[Target]> for HashOutTarget {
    type Error = anyhow::Error;

    fn try_from(elements: &[Target]) -> Result<Self, Self::Error> {
        ensure!(elements.len() == 4);
        Ok(Self {
            elements: elements.try_into().unwrap(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct MerkleCapTarget(pub Vec<HashOutTarget>);

/// Hash consisting of a byte array.
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct BytesHash<const N: usize>(pub [u8; N]);

impl<const N: usize> Sample for BytesHash<N> {
    #[inline]
    fn sample<R>(rng: &mut R) -> Self
    where
        R: rand::RngCore + ?Sized,
    {
        let mut buf = [0; N];
        rng.fill_bytes(&mut buf);
        Self(buf)
    }
}

impl<F: RichField, const N: usize> GenericHashOut<F> for BytesHash<N> {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes.try_into().unwrap())
    }

    fn to_vec(&self) -> Vec<F> {
        self.0
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

impl<const N: usize> Serialize for BytesHash<N> {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        todo!()
    }
}

impl<'de, const N: usize> Deserialize<'de> for BytesHash<N> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}
