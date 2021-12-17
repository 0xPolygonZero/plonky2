use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::field::field_types::{Field, PrimeField, RichField};
use crate::iop::target::Target;

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

    pub(crate) fn from_vec(elements: Vec<F>) -> Self {
        debug_assert!(elements.len() == 4);
        Self {
            elements: elements.try_into().unwrap(),
        }
    }

    pub(crate) fn from_partial(mut elements: Vec<F>) -> Self {
        debug_assert!(elements.len() <= 4);
        while elements.len() < 4 {
            elements.push(F::ZERO);
        }
        Self {
            elements: [elements[0], elements[1], elements[2], elements[3]],
        }
    }

    pub fn rand_from_rng<R: Rng>(rng: &mut R) -> Self {
        Self {
            elements: [
                F::rand_from_rng(rng),
                F::rand_from_rng(rng),
                F::rand_from_rng(rng),
                F::rand_from_rng(rng),
            ],
        }
    }

    pub fn rand() -> Self {
        Self {
            elements: [F::rand(), F::rand(), F::rand(), F::rand()],
        }
    }
}

impl<F: Field> Default for HashOut<F> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<F: PrimeField> From<Vec<u8>> for HashOut<F> {
    fn from(v: Vec<u8>) -> Self {
        HashOut {
            elements: v
                .chunks(8)
                .take(4)
                .map(|x| F::from_canonical_u64(u64::from_le_bytes(x.try_into().unwrap())))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl<F: PrimeField> From<HashOut<F>> for Vec<u8> {
    fn from(h: HashOut<F>) -> Self {
        h.elements
            .into_iter()
            .flat_map(|x| x.to_canonical_u64().to_le_bytes())
            .collect()
    }
}

impl<F: PrimeField> From<HashOut<F>> for Vec<F> {
    fn from(h: HashOut<F>) -> Self {
        h.elements.to_vec()
    }
}

impl<F: PrimeField> From<HashOut<F>> for u64 {
    fn from(h: HashOut<F>) -> Self {
        h.elements[0].to_canonical_u64()
    }
}

/// Represents a ~256 bit hash output.
#[derive(Copy, Clone, Debug)]
pub struct HashOutTarget {
    pub(crate) elements: [Target; 4],
}

impl HashOutTarget {
    pub(crate) fn from_vec(elements: Vec<Target>) -> Self {
        debug_assert!(elements.len() == 4);
        Self {
            elements: elements.try_into().unwrap(),
        }
    }

    pub(crate) fn from_partial(mut elements: Vec<Target>, zero: Target) -> Self {
        debug_assert!(elements.len() <= 4);
        while elements.len() < 4 {
            elements.push(zero);
        }
        Self {
            elements: [elements[0], elements[1], elements[2], elements[3]],
        }
    }
}

#[derive(Clone, Debug)]
pub struct MerkleCapTarget(pub Vec<HashOutTarget>);

/// Hash consisting of a byte array.
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct BytesHash<const N: usize>(pub [u8; N]);
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

impl<const N: usize> From<Vec<u8>> for BytesHash<N> {
    fn from(v: Vec<u8>) -> Self {
        Self(v.try_into().unwrap())
    }
}

impl<const N: usize> From<BytesHash<N>> for Vec<u8> {
    fn from(hash: BytesHash<N>) -> Self {
        hash.0.to_vec()
    }
}

impl<const N: usize> From<BytesHash<N>> for u64 {
    fn from(hash: BytesHash<N>) -> Self {
        u64::from_le_bytes(hash.0[..8].try_into().unwrap())
    }
}

impl<F: RichField, const N: usize> From<BytesHash<N>> for Vec<F> {
    fn from(hash: BytesHash<N>) -> Self {
        hash.0
            .chunks(7)
            .map(|bytes| {
                let mut arr = [0; 8];
                arr[..bytes.len()].copy_from_slice(bytes);
                F::from_canonical_u64(u64::from_le_bytes(arr))
            })
            .collect()
    }
}
