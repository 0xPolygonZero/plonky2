use crate::field::extension_field::quadratic::QuadraticFieldExtension;
use crate::field::extension_field::quartic::QuarticFieldExtension;
use crate::field::field::Field;

pub mod quadratic;
pub mod quartic;

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

impl<FE: QuadraticFieldExtension> FieldExtension<2> for FE {
    type BaseField = FE::BaseField;

    fn to_basefield_array(&self) -> [Self::BaseField; 2] {
        self.to_canonical_representation()
    }

    fn from_basefield_array(arr: [Self::BaseField; 2]) -> Self {
        Self::from_canonical_representation(arr)
    }

    fn from_basefield(x: Self::BaseField) -> Self {
        x.into()
    }
}

impl<FE: QuarticFieldExtension> FieldExtension<4> for FE {
    type BaseField = FE::BaseField;

    fn to_basefield_array(&self) -> [Self::BaseField; 4] {
        self.to_canonical_representation()
    }

    fn from_basefield_array(arr: [Self::BaseField; 4]) -> Self {
        Self::from_canonical_representation(arr)
    }

    fn from_basefield(x: Self::BaseField) -> Self {
        x.into()
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
