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
pub struct EvaluationVarsBaseBatch<'a, F: Field> {
    pub(crate) batch_size: usize,
    pub(crate) local_constants: &'a [F],
    pub(crate) local_wires: &'a [F],
    pub(crate) public_inputs_hash: &'a [HashOut<F>],
}

impl<'a, F: Field> EvaluationVarsBaseBatch<'a, F> {
    pub fn iter(&self) -> EvaluationVarsBaseBatchIter<'a, F> {
        EvaluationVarsBaseBatchIter::new(*self)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EvaluationVarsBaseBatchIter<'a, F: Field> {
    pub(crate) vars_batch: EvaluationVarsBaseBatch<'a, F>,
    pub(crate) i: usize,
}

impl<'a, F: Field> EvaluationVarsBaseBatchIter<'a, F> {
    pub fn new(vars_batch: EvaluationVarsBaseBatch<'a, F>) -> Self {
        Self { vars_batch, i: 0 }
    }
}

impl<'a, F: Field> Iterator for EvaluationVarsBaseBatchIter<'a, F> {
    type Item = EvaluationVarsBaseOwned<'a, F>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.vars_batch.batch_size {
            return None;
        }
        let res = EvaluationVarsBaseOwned {
            local_constants: (self.i..self.vars_batch.local_constants.len())
                .step_by(self.vars_batch.batch_size)
                .map(|j| self.vars_batch.local_constants[j])
                .collect(),
            local_wires: (self.i..self.vars_batch.local_wires.len())
                .step_by(self.vars_batch.batch_size)
                .map(|j| self.vars_batch.local_wires[j])
                .collect(),
            public_inputs_hash: &self.vars_batch.public_inputs_hash[self.i],
        };
        self.i += 1;
        Some(res)
    }
}

#[derive(Debug)]
pub struct EvaluationVarsBaseOwned<'a, F: Field> {
    pub(crate) local_constants: Vec<F>,
    pub(crate) local_wires: Vec<F>,
    pub(crate) public_inputs_hash: &'a HashOut<F>,
}

impl<'a, F: Field> EvaluationVarsBaseOwned<'a, F> {
    pub fn copyable(&'a self) -> EvaluationVarsBase<'a, F> {
        EvaluationVarsBase {
            local_constants: &self.local_constants,
            local_wires: &self.local_wires,
            public_inputs_hash: self.public_inputs_hash,
        }
    }
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

impl<'a, F: Field> EvaluationVarsBaseBatch<'a, F> {
    pub fn remove_prefix(&mut self, prefix: &[bool]) {
        self.local_constants = &self.local_constants[prefix.len() * self.batch_size..];
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
