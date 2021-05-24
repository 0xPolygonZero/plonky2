use std::convert::TryInto;
use std::ops::Range;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field::Field;
use crate::target::Target;

#[derive(Copy, Clone)]
pub struct EvaluationVars<'a, F: Field> {
    pub(crate) local_constants: &'a [F],
    pub(crate) local_wires: &'a [F],
}

impl<'a, F: Field> EvaluationVars<'a, F> {
    pub fn get_local_ext<const D: usize>(&self, wire_range: Range<usize>) -> F::Extension
    where
        F: Extendable<D>,
    {
        debug_assert_eq!(wire_range.len(), D);
        let arr = self.local_wires[wire_range].try_into().unwrap();
        F::Extension::from_basefield_array(arr)
    }
}

#[derive(Copy, Clone)]
pub struct EvaluationTargets<'a> {
    pub(crate) local_constants: &'a [Target],
    pub(crate) local_wires: &'a [Target],
}

impl<'a> EvaluationTargets<'a> {
    pub fn get_local_ext<const D: usize>(&self, wire_range: Range<usize>) -> ExtensionTarget<D> {
        debug_assert_eq!(wire_range.len(), D);
        let arr = self.local_wires[wire_range].try_into().unwrap();
        ExtensionTarget(arr)
    }
}
