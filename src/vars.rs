use std::convert::TryInto;
use std::ops::Range;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field::Field;

#[derive(Copy, Clone)]
pub struct EvaluationVars<'a, F: Extendable<D>, const D: usize> {
    pub(crate) local_constants: &'a [F::Extension],
    pub(crate) local_wires: &'a [F::Extension],
}

#[derive(Copy, Clone)]
pub struct EvaluationVarsBase<'a, F: Field> {
    pub(crate) local_constants: &'a [F],
    pub(crate) local_wires: &'a [F],
}

impl<'a, F: Extendable<D>, const D: usize> EvaluationVars<'a, F, D> {
    #[deprecated]
    pub fn get_local_ext(&self, wire_range: Range<usize>) -> F::Extension
    where
        F: Extendable<D>,
    {
        debug_assert_eq!(wire_range.len(), D);
        let arr = self.local_wires[wire_range].try_into().unwrap();
        F::Extension::from_basefield_array(arr)
    }
}

#[derive(Copy, Clone)]
pub struct EvaluationTargets<'a, const D: usize> {
    pub(crate) local_constants: &'a [ExtensionTarget<D>],
    pub(crate) local_wires: &'a [ExtensionTarget<D>],
}

impl<'a, const D: usize> EvaluationTargets<'a, D> {
    #[deprecated]
    pub fn get_local_ext(&self, wire_range: Range<usize>) -> ExtensionTarget<D> {
        debug_assert_eq!(wire_range.len(), D);
        let arr = self.local_wires[wire_range].try_into().unwrap();
        ExtensionTarget(arr)
    }
}
