use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field::Field;
use crate::target::Target;

pub struct ExtensionTarget<const D: usize>([Target; D]);

impl<F: Field> CircuitBuilder<F> {
    pub fn mul_extension<const D: usize>(a: ExtensionTarget<D>, b: ExtensionTarget<D>) -> ()
    where
        F: Extendable<D>,
    {
    }
}
