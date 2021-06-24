use std::convert::TryInto;
use std::ops::Range;

use crate::field::extension_field::algebra::ExtensionAlgebra;
use crate::field::extension_field::target::{ExtensionAlgebraTarget, ExtensionTarget};
use crate::field::extension_field::Extendable;
use crate::field::field::Field;

#[derive(Debug, Copy, Clone)]
pub struct EvaluationVars<'a, F: Extendable<D>, const D: usize> {
    pub(crate) local_constants: &'a [F::Extension],
    pub(crate) local_wires: &'a [F::Extension],
}

#[derive(Debug, Copy, Clone)]
pub struct EvaluationVarsBase<'a, F: Field> {
    pub(crate) local_constants: &'a [F],
    pub(crate) local_wires: &'a [F],
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
    pub fn remove_prefix(&mut self, prefix: &[bool]) {
        self.local_constants = &self.local_constants[prefix.len()..];
    }
}

#[derive(Copy, Clone)]
pub struct EvaluationTargets<'a, const D: usize> {
    pub(crate) local_constants: &'a [ExtensionTarget<D>],
    pub(crate) local_wires: &'a [ExtensionTarget<D>],
}

impl<'a, const D: usize> EvaluationTargets<'a, D> {
    pub fn get_local_ext_algebra(&self, wire_range: Range<usize>) -> ExtensionAlgebraTarget<D> {
        debug_assert_eq!(wire_range.len(), D);
        let arr = self.local_wires[wire_range].try_into().unwrap();
        ExtensionAlgebraTarget(arr)
    }
}
