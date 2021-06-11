use crate::field::field::Field;
use std::convert::TryInto;

pub mod algebra;
pub mod quadratic;
pub mod quartic;
pub mod target;

/// Optimal extension field trait.
/// A degree `d` field extension is optimal if there exists a base field element `W`,
/// such that the extension is `F[X]/(X^d-W)`.
#[allow(clippy::upper_case_acronyms)]
pub trait OEF<const D: usize>: FieldExtension<D> {
    // Element W of BaseField, such that `X^d - W` is irreducible over BaseField.
    const W: Self::BaseField;
}

impl<F: Field> OEF<1> for F {
    const W: Self::BaseField = F::ZERO;
}

pub trait Frobenius<BF: Frobeniable, const D: usize>: OEF<D, BaseField = BF> {
    /// FrobeniusField automorphisms: x -> x^p, where p is the order of BaseField.
    fn frobenius(&self) -> Self {
        self.repeated_frobenius(1)
    }

    /// Repeated Frobenius automorphisms: x -> x^(p^k).
    fn repeated_frobenius(&self, k: usize) -> Self {
        if k == 0 {
            return *self;
        } else if k >= D {
            return self.repeated_frobenius(k % D);
        }
        let arr = self.to_basefield_array();
        let z0 = match D {
            2 => Self::W.exp(BF::FROBENIUS_CONSTANTS_2[k - 1]),
            3 => Self::W.exp(BF::FROBENIUS_CONSTANTS_3[k - 1]),
            4 => Self::W.exp(BF::FROBENIUS_CONSTANTS_4[k - 1]),
            _ => unimplemented!("Only extensions of degree 2, 3, or 4 are allowed for now."),
        };
        let mut z = Self::BaseField::ONE;
        let mut res = [Self::BaseField::ZERO; D];
        for i in 0..D {
            res[i] = arr[i] * z;
            z *= z0;
        }

        Self::from_basefield_array(res)
    }
}

impl<F: Frobeniable> Frobenius<F, 1> for F {
    fn frobenius(&self) -> Self {
        *self
    }
    fn repeated_frobenius(&self, _k: usize) -> Self {
        *self
    }
}

/// Trait to hardcode constants used in the Frobenius automorphism.
pub trait Frobeniable: Field {
    //! `FROBENIUS_CONSTANTS_D[i-1] = floor( p^i / D) mod p-1`
    const FROBENIUS_CONSTANTS_2: [u64; 1];
    const FROBENIUS_CONSTANTS_3: [u64; 2];
    const FROBENIUS_CONSTANTS_4: [u64; 3];
}

pub trait Extendable<const D: usize>: Frobeniable + Sized {
    type Extension: Field + OEF<D, BaseField = Self> + Frobenius<Self, D> + From<Self>;
}

impl<F: Frobeniable + Frobenius<F, 1>> Extendable<1> for F {
    type Extension = F;
}

pub trait FieldExtension<const D: usize>: Field {
    type BaseField: Field;

    fn to_basefield_array(&self) -> [Self::BaseField; D];

    fn from_basefield_array(arr: [Self::BaseField; D]) -> Self;

    fn from_basefield(x: Self::BaseField) -> Self;

    fn is_in_basefield(&self) -> bool {
        self.to_basefield_array()[1..].iter().all(|x| x.is_zero())
    }
}

impl<F: Field> FieldExtension<1> for F {
    type BaseField = F;

    fn to_basefield_array(&self) -> [Self::BaseField; 1] {
        [*self]
    }

    fn from_basefield_array(arr: [Self::BaseField; 1]) -> Self {
        arr[0]
    }

    fn from_basefield(x: Self::BaseField) -> Self {
        x
    }
}

/// Flatten the slice by sending every extension field element to its D-sized canonical representation.
pub fn flatten<F: Field, const D: usize>(l: &[F::Extension]) -> Vec<F>
where
    F: Extendable<D>,
{
    l.iter()
        .flat_map(|x| x.to_basefield_array().to_vec())
        .collect()
}

/// Batch every D-sized chunks into extension field elements.
pub fn unflatten<F: Field, const D: usize>(l: &[F]) -> Vec<F::Extension>
where
    F: Extendable<D>,
{
    debug_assert_eq!(l.len() % D, 0);
    l.chunks_exact(D)
        .map(|c| F::Extension::from_basefield_array(c.to_vec().try_into().unwrap()))
        .collect()
}
