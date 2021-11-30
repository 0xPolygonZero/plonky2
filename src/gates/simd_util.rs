use std::ops::Index;

use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::field::packable::Packable;
use crate::field::packed_field::PackedField;
use crate::field::packed_field::Singleton;
use crate::gates::gate::Gate;
use crate::plonk::vars::EvaluationVarsBaseBatch;

#[derive(Debug, Copy, Clone)]
pub struct StridedView<'a, P: PackedField> {
    pub(crate) stride: usize,
    pub(crate) offset: usize,
    pub(crate) data: &'a [P::FieldType],
}

impl<P: PackedField> Index<usize> for StridedView<'_, P> {
    type Output = P;

    fn index(&self, index: usize) -> &Self::Output {
        let start_index = index * self.stride + self.offset;
        let slice = &self.data[start_index..start_index + P::WIDTH];
        slice[P::WIDTH - 1]; // Panic on out of bounds access.
        unsafe { &*slice.as_ptr().cast() }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EvaluationVarsBaseSimd<'a, P: PackedField> {
    pub(crate) local_constants: StridedView<'a, P>,
    pub(crate) local_wires: StridedView<'a, P>,
}

impl<P: PackedField> EvaluationVarsBaseSimd<'_, P> {
    pub fn new<'a>(
        index: usize,
        vars_batch: EvaluationVarsBaseBatchSimd<'a, P>,
    ) -> EvaluationVarsBaseSimd<'a, P> {
        EvaluationVarsBaseSimd {
            local_constants: StridedView {
                stride: vars_batch.batch_size,
                offset: index,
                data: vars_batch.local_constants,
            },
            local_wires: StridedView {
                stride: vars_batch.batch_size,
                offset: index,
                data: vars_batch.local_wires,
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EvaluationVarsBaseBatchSimd<'a, P: PackedField> {
    pub(crate) batch_size: usize,
    pub(crate) local_constants: &'a [P::FieldType],
    pub(crate) local_wires: &'a [P::FieldType],
}

impl<P: PackedField> EvaluationVarsBaseBatchSimd<'_, P> {
    pub fn new<'a>(
        vars_batch: EvaluationVarsBaseBatch<'a, P::FieldType>,
    ) -> EvaluationVarsBaseBatchSimd<'a, P> {
        EvaluationVarsBaseBatchSimd {
            batch_size: vars_batch.batch_size,
            local_constants: vars_batch.local_constants,
            local_wires: vars_batch.local_wires,
        }
    }

    pub fn iter_with_leftovers(
        &self,
    ) -> (
        EvaluationVarsBaseBatchSimdIterator<P>,
        EvaluationVarsBaseBatchSimdIterator<Singleton<P::FieldType>>,
    ) {
        (
            EvaluationVarsBaseBatchSimdIterator::new(
                *self,
                0,
                self.batch_size / P::WIDTH * P::WIDTH,
                P::WIDTH,
            ),
            EvaluationVarsBaseBatchSimdIterator::new(
                EvaluationVarsBaseBatchSimd {
                    batch_size: self.batch_size,
                    local_constants: self.local_constants,
                    local_wires: self.local_wires,
                },
                self.batch_size / P::WIDTH * P::WIDTH,
                self.batch_size,
                1,
            ),
        )
    }
}

pub struct EvaluationVarsBaseBatchSimdIterator<'a, P: PackedField> {
    pub(crate) vars_batch: EvaluationVarsBaseBatchSimd<'a, P>,
    i: usize,
    end: usize,
    stride: usize,
}

impl<'a, P: PackedField> EvaluationVarsBaseBatchSimdIterator<'a, P> {
    pub fn new(
        vars_batch: EvaluationVarsBaseBatchSimd<'a, P>,
        start: usize,
        end: usize,
        stride: usize,
    ) -> EvaluationVarsBaseBatchSimdIterator<'a, P> {
        EvaluationVarsBaseBatchSimdIterator {
            vars_batch,
            i: start,
            end,
            stride,
        }
    }
}

impl<'a, P: PackedField> Iterator for EvaluationVarsBaseBatchSimdIterator<'a, P> {
    type Item = EvaluationVarsBaseSimd<'a, P>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.end {
            return None;
        }
        let res = EvaluationVarsBaseSimd::new(self.i, self.vars_batch);
        self.i += self.stride;
        Some(res)
    }
}

pub trait SimdGateBase<F: RichField + Extendable<D>, const D: usize>:
    'static + Gate<F, D> + Send + Sync
{
    fn eval_unfiltered_base_simd<P: PackedField<FieldType = F>, Y: FnMut(P)>(
        &self,
        vars_base: EvaluationVarsBaseSimd<P>,
        yield_constr: Y,
    );

    /// Evaluates entire batch of points. Returns a matrix of constraints. Constraint j for point i
    /// is at index j * batch_size + i.
    fn eval_unfiltered_base_batch_simd(&self, vars_batch: EvaluationVarsBaseBatch<F>) -> Vec<F> {
        let vars_batch_simd =
            EvaluationVarsBaseBatchSimd::<<F as Packable>::PackedType>::new(vars_batch);
        let (vars_simd_iter, leftovers) = vars_batch_simd.iter_with_leftovers();

        let mut res = vec![F::ZERO; vars_batch.batch_size * self.num_constraints()];
        for (i, vars_simd) in vars_simd_iter.enumerate() {
            let mut n_constr = 0usize;
            self.eval_unfiltered_base_simd(vars_simd, |constraint| {
                debug_assert!(n_constr < self.num_constraints());
                let start =
                    n_constr * vars_batch.batch_size + i * <F as Packable>::PackedType::WIDTH;
                let end = start + <F as Packable>::PackedType::WIDTH;
                unsafe {
                    *(&mut res[start..end]).as_mut_ptr().cast() = constraint;
                }
                n_constr += 1;
            });
        }
        for (i, vars_simd) in leftovers.enumerate() {
            let j = i + vars_batch.batch_size / <F as Packable>::PackedType::WIDTH
                * <F as Packable>::PackedType::WIDTH;
            // Same thing, but in scalar.
            let mut n_constr = 0usize;
            self.eval_unfiltered_base_simd(vars_simd, |constraint| {
                debug_assert!(n_constr < self.num_constraints());
                let start = n_constr * vars_batch.batch_size + j;
                let end = start + j;
                unsafe {
                    *(&mut res[start..end]).as_mut_ptr().cast() = constraint;
                }
                n_constr += 1;
            });
        }

        res
    }
}
