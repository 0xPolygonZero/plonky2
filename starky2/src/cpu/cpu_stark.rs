use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

#[derive(Copy, Clone)]
pub struct CpuStark<F, const D: usize> {
    pub f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for CpuStark<F, D> {
    const COLUMNS: usize = 10;
    const PUBLIC_INPUTS: usize = 0;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        _vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        _yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
    }

    fn eval_ext_circuit(
        &self,
        _builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        _vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        _yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        vec![PermutationPair::singletons(8, 9)]
    }
}
