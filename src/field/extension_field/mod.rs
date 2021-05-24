use crate::field::field::Field;

pub mod quadratic;
pub mod quartic;

/// Optimal extension field trait.
/// A degree `d` field extension is optimal if there exists a base field element `W`,
/// such that the extension is `F[X]/(X^d-W)`.
pub trait OEF<const D: usize>: FieldExtension<D> {
    // Element W of BaseField, such that `X^d - W` is irreducible over BaseField.
    const W: Self::BaseField;

    /// Frobenius automorphisms: x -> x^p, where p is the order of BaseField.
    fn frobenius(&self) -> Self {
        let arr = self.to_basefield_array();
        let k = (Self::BaseField::ORDER - 1) / (D as u64);
        let z0 = Self::W.exp(k);
        let mut z = Self::BaseField::ONE;
        let mut res = [Self::BaseField::ZERO; D];
        for i in 0..D {
            res[i] = arr[i] * z;
            z *= z0;
        }

        Self::from_basefield_array(res)
    }
}

pub trait Extendable<const D: usize>: Sized {
    type Extension: Field + FieldExtension<D, BaseField = Self> + From<Self>;
}

impl<F: Field> Extendable<1> for F {
    type Extension = Self;
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
        .map(|c| {
            let mut arr = [F::ZERO; D];
            arr.copy_from_slice(c);
            F::Extension::from_basefield_array(arr)
        })
        .collect()
}
