use crate::field_types::Field;
use crate::packed_field::PackedField;

/// Points us to the default packing for a particular field. There may me multiple choices of
/// PackedField for a particular Field (e.g. every Field is also a PackedField), but this is the
/// recommended one. The recommended packing varies by target_arch and target_feature.
pub trait Packable: Field {
    type Packing: PackedField<Scalar = Self>;
}

impl<F: Field> Packable for F {
    default type Packing = Self;
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
impl Packable for crate::field::goldilocks_field::GoldilocksField {
    type Packing = crate::field::arch::x86_64::avx2_goldilocks_field::Avx2GoldilocksField;
}
