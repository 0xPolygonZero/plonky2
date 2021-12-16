use std::ops::Range;

use crate::field::extension_field::algebra::ExtensionAlgebra;
use crate::field::extension_field::target::{ExtensionAlgebraTarget, ExtensionTarget};
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::Field;
use crate::hash::hash_types::{HashOut, HashOutTarget};
use crate::util::strided_view::PackedStridedView;

#[derive(Debug, Copy, Clone)]
pub struct EvaluationVars<'a, F: Extendable<D>, const D: usize> {
    pub(crate) local_constants: &'a [F::Extension],
    pub(crate) local_wires: &'a [F::Extension],
    pub(crate) public_inputs_hash: &'a HashOut<F>,
}

/// A batch of evaluation vars, in the base field.
/// Wires and constants are stored in an evaluation point-major order (that is, wire 0 for all
/// evaluation points, then wire 1 for all points, and so on).
#[derive(Debug, Copy, Clone)]
pub struct EvaluationVarsBaseBatch<'a, F: Field> {
    batch_size: usize,
    pub(crate) local_constants: &'a [F],
    pub(crate) local_wires: &'a [F],
    pub(crate) public_inputs_hash: &'a HashOut<F>,
}

/// A view into `EvaluationVarsBaseBatch` for a particular evaluation point. Does not copy the data.
#[derive(Debug, Copy, Clone)]
pub struct EvaluationVarsBase<'a, F: Field> {
    pub(crate) local_constants: PackedStridedView<'a, F>,
    pub(crate) local_wires: PackedStridedView<'a, F>,
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

impl<'a, F: Field> EvaluationVarsBaseBatch<'a, F> {
    pub fn new(
        batch_size: usize,
        local_constants: &'a [F],
        local_wires: &'a [F],
        public_inputs_hash: &'a HashOut<F>,
    ) -> Self {
        assert_eq!(local_constants.len() % batch_size, 0);
        assert_eq!(local_wires.len() % batch_size, 0);
        Self {
            batch_size,
            local_constants,
            local_wires,
            public_inputs_hash,
        }
    }

    pub fn remove_prefix(&mut self, prefix: &[bool]) {
        self.local_constants = &self.local_constants[prefix.len() * self.len()..];
    }

    pub fn len(&self) -> usize {
        self.batch_size
    }

    pub fn view(&self, index: usize) -> EvaluationVarsBase<'a, F> {
        // We cannot implement `Index` as `EvaluationVarsBase` is a struct, not a reference.
        assert!(index < self.len());
        let local_constants = PackedStridedView::new(self.local_constants, self.len(), index);
        let local_wires = PackedStridedView::new(self.local_wires, self.len(), index);
        EvaluationVarsBase {
            local_constants,
            local_wires,
            public_inputs_hash: self.public_inputs_hash,
        }
    }

    pub fn iter(&self) -> EvaluationVarsBaseBatchIter<'a, F> {
        EvaluationVarsBaseBatchIter::new(*self)
    }
}

impl<'a, F: Field> EvaluationVarsBase<'a, F> {
    pub fn get_local_ext<const D: usize>(&self, wire_range: Range<usize>) -> F::Extension
    where
        F: Extendable<D>,
    {
        debug_assert_eq!(wire_range.len(), D);
        let arr = self.local_wires.view(wire_range).try_into().unwrap();
        F::Extension::from_basefield_array(arr)
    }
}

/// Iterator of views (`EvaluationVarsBase`) into a `EvaluationVarsBaseBatch`.
pub struct EvaluationVarsBaseBatchIter<'a, F: Field> {
    i: usize,
    vars_batch: EvaluationVarsBaseBatch<'a, F>,
}

impl<'a, F: Field> EvaluationVarsBaseBatchIter<'a, F> {
    pub fn new(vars_batch: EvaluationVarsBaseBatch<'a, F>) -> Self {
        EvaluationVarsBaseBatchIter { i: 0, vars_batch }
    }
}

impl<'a, F: Field> Iterator for EvaluationVarsBaseBatchIter<'a, F> {
    type Item = EvaluationVarsBase<'a, F>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.vars_batch.len() {
            let res = self.vars_batch.view(self.i);
            self.i += 1;
            Some(res)
        } else {
            None
        }
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
