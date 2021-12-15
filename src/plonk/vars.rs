use std::ops::Range;

use crate::field::extension_field::algebra::ExtensionAlgebra;
use crate::field::extension_field::target::{ExtensionAlgebraTarget, ExtensionTarget};
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::Field;
use crate::hash::hash_types::{HashOut, HashOutTarget};

#[derive(Debug, Copy, Clone)]
pub struct EvaluationVars<'a, F: Extendable<D>, const D: usize> {
    pub(crate) local_constants: &'a [F::Extension],
    pub(crate) local_wires: &'a [F::Extension],
    pub(crate) public_inputs_hash: &'a HashOut<F>,
}

#[derive(Debug, Copy, Clone)]
pub struct EvaluationVarsBase<'a, F: Field> {
    pub(crate) local_constants: &'a [F],
    pub(crate) local_wires: &'a [F],
    pub(crate) public_inputs_hash: &'a HashOut<F>,
}

impl<'a, F: Extendable<D>, const D: usize> EvaluationVars<'a, F, D> {
    pub fn get_local_ext_algebra(
        &self,
        wire_range: Range<usize>,
    ) -> ExtensionAlgebra<F::Extension, D> {
        debug_assert_eq!(wire_range.len(), D);
        let arr = self.local_wires[wire_range].try_into().unwrap();
        ExtensionAlgebra::from_basefield_array(arr)
    }

    pub fn remove_prefix(&mut self, prefix: &[bool]) {
        self.local_constants = &self.local_constants[prefix.len()..];
    }
}

impl<'a, F: Field> EvaluationVarsBase<'a, F> {
    pub fn get_local_ext<const D: usize>(&self, wire_range: Range<usize>) -> F::Extension
    where
        F: Extendable<D>,
    {
        debug_assert_eq!(wire_range.len(), D);
        let arr = self.local_wires[wire_range].try_into().unwrap();
        F::Extension::from_basefield_array(arr)
    }

    pub fn remove_prefix(&mut self, prefix: &[bool]) {
        self.local_constants = &self.local_constants[prefix.len()..];
    }
}

impl<'a, const D: usize> EvaluationTargets<'a, D> {
    pub fn remove_prefix(&mut self, prefix: &[bool]) {
        self.local_constants = &self.local_constants[prefix.len()..];
    }
}

#[derive(Copy, Clone)]
pub struct EvaluationTargets<'a, const D: usize> {
    pub(crate) local_constants: &'a [ExtensionTarget<D>],
    pub(crate) local_wires: &'a [ExtensionTarget<D>],
    pub(crate) public_inputs_hash: &'a HashOutTarget,
}

impl<'a, const D: usize> EvaluationTargets<'a, D> {
    pub fn get_local_ext_algebra(&self, wire_range: Range<usize>) -> ExtensionAlgebraTarget<D> {
        debug_assert_eq!(wire_range.len(), D);
        let arr = self.local_wires[wire_range].try_into().unwrap();
        ExtensionAlgebraTarget(arr)
    }
}
