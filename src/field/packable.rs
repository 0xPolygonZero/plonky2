use crate::field::field_types::Field;
use crate::field::packed_field::{PackedField, Singleton};

/// Points us to the default packing for a particular field. There may me multiple choices of
/// PackedField for a particular Field (e.g. Singleton works for all fields), but this is the
/// recommended one. The recommended packing varies by target_arch and target_feature.
pub trait Packable: Field {
    type PackedType: PackedField<FieldType = Self>;
}

impl<F: Field> Packable for F {
    default type PackedType = Singleton<Self>;
}
