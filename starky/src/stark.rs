use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::fri::structure::{
    FriBatchInfo, FriBatchInfoTarget, FriInstanceInfo, FriInstanceInfoTarget, FriOracleInfo,
    FriPolynomialInfo,
};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::vars::StarkEvaluationTargets;
use crate::vars::StarkEvaluationVars;

/// Represents a STARK system.
// TODO: Add a `constraint_degree` fn that returns the maximum constraint degree.
pub trait Stark<F: RichField + Extendable<D>, const D: usize>: Sync {
    /// The total number of columns in the trace.
    const COLUMNS: usize;
    /// The number of public inputs.
    const PUBLIC_INPUTS: usize;

    /// Evaluate constraints at a vector of points.
    ///
    /// The points are elements of a field `FE`, a degree `D2` extension of `F`. This lets us
    /// evaluate constraints over a larger domain if desired. This can also be called with `FE = F`
    /// and `D2 = 1`, in which case we are using the trivial extension, i.e. just evaluating
    /// constraints over `F`.
    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    /// Evaluate constraints at a vector of points from the base field `F`.
    fn eval_packed_base<P: PackedField<Scalar = F>>(
        &self,
        vars: StarkEvaluationVars<F, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) {
        self.eval_packed_generic(vars, yield_constr)
    }

    /// Evaluate constraints at a single point from the degree `D` extension field.
    fn eval_ext(
        &self,
        vars: StarkEvaluationVars<
            F::Extension,
            F::Extension,
            { Self::COLUMNS },
            { Self::PUBLIC_INPUTS },
        >,
        yield_constr: &mut ConstraintConsumer<F::Extension>,
    ) {
        self.eval_packed_generic(vars, yield_constr)
    }

    /// Evaluate constraints at a vector of points from the degree `D` extension field. This is like
    /// `eval_ext`, except in the context of a recursive circuit.
    fn eval_ext_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    );

    /// The maximum constraint degree.
    fn constraint_degree(&self) -> usize;

    /// The maximum constraint degree.
    fn quotient_degree_factor(&self) -> usize {
        1.max(self.constraint_degree() - 1)
    }

    /// Computes the FRI instance used to prove this Stark.
    // TODO: Permutation polynomials.
    fn fri_instance(
        &self,
        zeta: F::Extension,
        g: F,
        num_challenges: usize,
    ) -> FriInstanceInfo<F, D> {
        let no_blinding_oracle = FriOracleInfo { blinding: false };
        let trace_info = FriPolynomialInfo::from_range(0, 0..Self::COLUMNS);
        let quotient_info =
            FriPolynomialInfo::from_range(1, 0..self.quotient_degree_factor() * num_challenges);
        let zeta_batch = FriBatchInfo {
            point: zeta,
            polynomials: [trace_info.clone(), quotient_info].concat(),
        };
        let zeta_right_batch = FriBatchInfo::<F, D> {
            point: zeta.scalar_mul(g),
            polynomials: trace_info,
        };
        FriInstanceInfo {
            oracles: vec![no_blinding_oracle; 3],
            batches: vec![zeta_batch],
        }
    }

    /// Computes the FRI instance used to prove this Stark.
    // TODO: Permutation polynomials.
    fn fri_instance_target(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        zeta: ExtensionTarget<D>,
        g: F,
        num_challenges: usize,
    ) -> FriInstanceInfoTarget<D> {
        let no_blinding_oracle = FriOracleInfo { blinding: false };
        let trace_info = FriPolynomialInfo::from_range(0, 0..Self::COLUMNS);
        let quotient_info =
            FriPolynomialInfo::from_range(1, 0..self.quotient_degree_factor() * num_challenges);
        let zeta_batch = FriBatchInfoTarget {
            point: zeta,
            polynomials: [trace_info.clone(), quotient_info].concat(),
        };
        let zeta_right = builder.mul_const_extension(g, zeta);
        let zeta_right_batch = FriBatchInfoTarget {
            point: zeta_right,
            polynomials: trace_info,
        };
        FriInstanceInfoTarget {
            oracles: vec![no_blinding_oracle; 3],
            batches: vec![zeta_batch],
        }
    }
}
