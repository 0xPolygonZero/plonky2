use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::fri::structure::{
    FriBatchInfo, FriBatchInfoTarget, FriInstanceInfo, FriInstanceInfoTarget, FriOracleInfo,
    FriPolynomialInfo,
};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::lookup::Lookup;

const TRACE_ORACLE_INDEX: usize = 0;
const AUXILIARY_ORACLE_INDEX: usize = 1;
const QUOTIENT_ORACLE_INDEX: usize = 2;

/// Represents a STARK system.
pub trait Stark<F: RichField + Extendable<D>, const D: usize>: Sync {
    /// The total number of columns in the trace.
    const COLUMNS: usize = Self::EvaluationFrameTarget::COLUMNS;

    /// This is used to evaluate constraints natively.
    type EvaluationFrame<FE, P, const D2: usize>: StarkEvaluationFrame<P>
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    /// The `Target` version of `Self::EvaluationFrame`, used to evaluate constraints recursively.
    type EvaluationFrameTarget: StarkEvaluationFrame<ExtensionTarget<D>>;

    /// Evaluate constraints at a vector of points.
    ///
    /// The points are elements of a field `FE`, a degree `D2` extension of `F`. This lets us
    /// evaluate constraints over a larger domain if desired. This can also be called with `FE = F`
    /// and `D2 = 1`, in which case we are using the trivial extension, i.e. just evaluating
    /// constraints over `F`.
    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: &Self::EvaluationFrame<FE, P, D2>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    /// Evaluate constraints at a vector of points from the base field `F`.
    fn eval_packed_base<P: PackedField<Scalar = F>>(
        &self,
        vars: &Self::EvaluationFrame<F, P, 1>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) {
        self.eval_packed_generic(vars, yield_constr)
    }

    /// Evaluate constraints at a single point from the degree `D` extension field.
    fn eval_ext(
        &self,
        vars: &Self::EvaluationFrame<F::Extension, F::Extension, D>,
        yield_constr: &mut ConstraintConsumer<F::Extension>,
    ) {
        self.eval_packed_generic(vars, yield_constr)
    }

    /// Evaluate constraints at a vector of points from the degree `D` extension field. This is like
    /// `eval_ext`, except in the context of a recursive circuit.
    /// Note: constraints must be added through`yield_constr.constraint(builder, constraint)` in the
    /// same order as they are given in `eval_packed_generic`.
    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    );

    /// The maximum constraint degree.
    fn constraint_degree(&self) -> usize;

    /// The maximum constraint degree.
    fn quotient_degree_factor(&self) -> usize {
        1.max(self.constraint_degree() - 1)
    }

    fn num_quotient_polys(&self, config: &StarkConfig) -> usize {
        self.quotient_degree_factor() * config.num_challenges
    }

    /// Computes the FRI instance used to prove this Stark.
    fn fri_instance(
        &self,
        zeta: F::Extension,
        g: F,
        num_ctl_helpers: usize,
        num_ctl_zs: Vec<usize>,
        config: &StarkConfig,
    ) -> FriInstanceInfo<F, D> {
        let trace_oracle = FriOracleInfo {
            num_polys: Self::COLUMNS,
            blinding: false,
        };
        let trace_info = FriPolynomialInfo::from_range(TRACE_ORACLE_INDEX, 0..Self::COLUMNS);

        let num_lookup_columns = self.num_lookup_helper_columns(config);
        let num_auxiliary_polys = num_lookup_columns + num_ctl_helpers + num_ctl_zs.len();
        let auxiliary_oracle = FriOracleInfo {
            num_polys: num_auxiliary_polys,
            blinding: false,
        };
        let auxiliary_polys_info =
            FriPolynomialInfo::from_range(AUXILIARY_ORACLE_INDEX, 0..num_auxiliary_polys);

        let mut start_index = num_lookup_columns;
        let ctl_zs_info = FriPolynomialInfo::from_range(
            AUXILIARY_ORACLE_INDEX,
            num_lookup_columns + num_ctl_helpers..num_auxiliary_polys,
        );

        let num_quotient_polys = self.num_quotient_polys(config);
        let quotient_oracle = FriOracleInfo {
            num_polys: num_quotient_polys,
            blinding: false,
        };
        let quotient_info =
            FriPolynomialInfo::from_range(QUOTIENT_ORACLE_INDEX, 0..num_quotient_polys);

        let zeta_batch = FriBatchInfo {
            point: zeta,
            polynomials: [
                trace_info.clone(),
                auxiliary_polys_info.clone(),
                quotient_info,
            ]
            .concat(),
        };
        let zeta_next_batch = FriBatchInfo {
            point: zeta.scalar_mul(g),
            polynomials: [trace_info, auxiliary_polys_info].concat(),
        };
        let ctl_first_batch = FriBatchInfo {
            point: F::Extension::ONE,
            polynomials: ctl_zs_info,
        };
        FriInstanceInfo {
            oracles: vec![trace_oracle, auxiliary_oracle, quotient_oracle],
            batches: vec![zeta_batch, zeta_next_batch, ctl_first_batch],
        }
    }

    /// Computes the FRI instance used to prove this Stark.
    fn fri_instance_target(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        zeta: ExtensionTarget<D>,
        g: F,
        num_ctl_helper_polys: usize,
        num_ctl_zs: usize,
        inner_config: &StarkConfig,
    ) -> FriInstanceInfoTarget<D> {
        let trace_oracle = FriOracleInfo {
            num_polys: Self::COLUMNS,
            blinding: false,
        };
        let trace_info = FriPolynomialInfo::from_range(TRACE_ORACLE_INDEX, 0..Self::COLUMNS);

        let num_lookup_columns = self.num_lookup_helper_columns(inner_config);
        let num_auxiliary_polys = num_lookup_columns + num_ctl_helper_polys + num_ctl_zs;
        let auxiliary_oracle = FriOracleInfo {
            num_polys: num_auxiliary_polys,
            blinding: false,
        };
        let auxiliary_polys_info =
            FriPolynomialInfo::from_range(AUXILIARY_ORACLE_INDEX, 0..num_auxiliary_polys);

        let ctl_zs_info = FriPolynomialInfo::from_range(
            AUXILIARY_ORACLE_INDEX,
            num_lookup_columns + num_ctl_helper_polys
                ..num_lookup_columns + num_ctl_helper_polys + num_ctl_zs,
        );

        let num_quotient_polys = self.num_quotient_polys(inner_config);
        let quotient_oracle = FriOracleInfo {
            num_polys: num_quotient_polys,
            blinding: false,
        };
        let quotient_info =
            FriPolynomialInfo::from_range(QUOTIENT_ORACLE_INDEX, 0..num_quotient_polys);

        let zeta_batch = FriBatchInfoTarget {
            point: zeta,
            polynomials: [
                trace_info.clone(),
                auxiliary_polys_info.clone(),
                quotient_info,
            ]
            .concat(),
        };
        let zeta_next = builder.mul_const_extension(g, zeta);
        let zeta_next_batch = FriBatchInfoTarget {
            point: zeta_next,
            polynomials: [trace_info, auxiliary_polys_info].concat(),
        };
        let ctl_first_batch = FriBatchInfoTarget {
            point: builder.one_extension(),
            polynomials: ctl_zs_info,
        };
        FriInstanceInfoTarget {
            oracles: vec![trace_oracle, auxiliary_oracle, quotient_oracle],
            batches: vec![zeta_batch, zeta_next_batch, ctl_first_batch],
        }
    }

    fn lookups(&self) -> Vec<Lookup<F>> {
        vec![]
    }

    fn num_lookup_helper_columns(&self, config: &StarkConfig) -> usize {
        self.lookups()
            .iter()
            .map(|lookup| lookup.num_helper_columns(self.constraint_degree()))
            .sum::<usize>()
            * config.num_challenges
    }

    fn uses_lookups(&self) -> bool {
        !self.lookups().is_empty()
    }
}
