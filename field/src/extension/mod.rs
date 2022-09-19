use std::convert::TryInto;

use crate::types::Field;

pub mod algebra;
pub mod quadratic;
pub mod quartic;
pub mod quintic;

/// Optimal extension field trait.
/// A degree `d` field extension is optimal if there exists a base field element `W`,
/// such that the extension is `F[X]/(X^d-W)`.
#[allow(clippy::upper_case_acronyms)]
pub trait OEF<const D: usize>: FieldExtension<D> {
    // Element W of BaseField, such that `X^d - W` is irreducible over BaseField.
    const W: Self::BaseField;

    // Element of BaseField such that DTH_ROOT^D == 1. Implementors
    // should set this to W^((p - 1)/D), where W is as above and p is
    // the order of the BaseField.
    const DTH_ROOT: Self::BaseField;
}

impl<F: Field> OEF<1> for F {
    const W: Self::BaseField = F::ONE;
    const DTH_ROOT: Self::BaseField = F::ONE;
}

pub trait Frobenius<const D: usize>: OEF<D> {
    /// FrobeniusField automorphisms: x -> x^p, where p is the order of BaseField.
    fn frobenius(&self) -> Self {
        self.repeated_frobenius(1)
    }

    /// Repeated Frobenius automorphisms: x -> x^(p^count).
    ///
    /// Follows precomputation suggestion in Section 11.3.3 of the
    /// Handbook of Elliptic and Hyperelliptic Curve Cryptography.
    fn repeated_frobenius(&self, count: usize) -> Self {
        if count == 0 {
            return *self;
        } else if count >= D {
            // x |-> x^(p^D) is the identity, so x^(p^count) ==
            // x^(p^(count % D))
            return self.repeated_frobenius(count % D);
        }
        let arr = self.to_basefield_array();

        // z0 = DTH_ROOT^count = W^(k * count) where k = floor((p^D-1)/D)
        let mut z0 = Self::DTH_ROOT;
        for _ in 1..count {
            z0 *= Self::DTH_ROOT;
        }

        let mut res = [Self::BaseField::ZERO; D];
        for (i, z) in z0.powers().take(D).enumerate() {
            res[i] = arr[i] * z;
        }

        Self::from_basefield_array(res)
    }
}

pub trait Extendable<const D: usize>: Field + Sized {
    type Extension: Field + OEF<D, BaseField = Self> + Frobenius<D> + From<Self>;

    const W: Self;

    const DTH_ROOT: Self;

    /// Chosen so that when raised to the power `(p^D - 1) >> F::Extension::TWO_ADICITY)`
    /// we obtain F::EXT_POWER_OF_TWO_GENERATOR.
    const EXT_MULTIPLICATIVE_GROUP_GENERATOR: [Self; D];

    /// Chosen so that when raised to the power `1<<(Self::TWO_ADICITY-Self::BaseField::TWO_ADICITY)`,
    /// we get `Self::BaseField::POWER_OF_TWO_GENERATOR`. This makes `primitive_root_of_unity` coherent
    /// with the base field which implies that the FFT commutes with field inclusion.
    const EXT_POWER_OF_TWO_GENERATOR: [Self; D];
}

impl<F: Field + Frobenius<1> + FieldExtension<1, BaseField = F>> Extendable<1> for F {
    type Extension = F;
    const W: Self = F::ONE;
    const DTH_ROOT: Self = F::ONE;
    const EXT_MULTIPLICATIVE_GROUP_GENERATOR: [Self; 1] = [F::MULTIPLICATIVE_GROUP_GENERATOR];
    const EXT_POWER_OF_TWO_GENERATOR: [Self; 1] = [F::POWER_OF_TWO_GENERATOR];
}

pub trait FieldExtension<const D: usize>: Field {
    type BaseField: Field;

    fn to_basefield_array(&self) -> [Self::BaseField; D];

    fn from_basefield_array(arr: [Self::BaseField; D]) -> Self;

    fn from_basefield(x: Self::BaseField) -> Self;

    fn is_in_basefield(&self) -> bool {
        self.to_basefield_array()[1..].iter().all(|x| x.is_zero())
    }

    fn scalar_mul(&self, scalar: Self::BaseField) -> Self {
        let mut res = self.to_basefield_array();
        res.iter_mut().for_each(|x| {
            *x *= scalar;
        });
        Self::from_basefield_array(res)
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
