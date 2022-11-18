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
use plonky2_util::ceil_div_usize;

use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::permutation::PermutationPair;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

const TRACE_ORACLE_INDEX: usize = 0;
const PERMUTATION_CTL_ORACLE_INDEX: usize = 1;
const QUOTIENT_ORACLE_INDEX: usize = 2;

/// Represents a STARK system.
pub trait Stark<F: RichField + Extendable<D>, const D: usize>: Sync {
    /// The total number of columns in the trace.
    const COLUMNS: usize;

    /// Evaluate constraints at a vector of points.
    ///
    /// The points are elements of a field `FE`, a degree `D2` extension of `F`. This lets us
    /// evaluate constraints over a larger domain if desired. This can also be called with `FE = F`
    /// and `D2 = 1`, in which case we are using the trivial extension, i.e. just evaluating
    /// constraints over `F`.
    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    /// Evaluate constraints at a vector of points from the base field `F`.
    fn eval_packed_base<P: PackedField<Scalar = F>>(
        &self,
        vars: StarkEvaluationVars<F, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) {
        self.eval_packed_generic(vars, yield_constr)
    }

    /// Evaluate constraints at a single point from the degree `D` extension field.
    fn eval_ext(
        &self,
        vars: StarkEvaluationVars<F::Extension, F::Extension, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<F::Extension>,
    ) {
        self.eval_packed_generic(vars, yield_constr)
    }

    /// Evaluate constraints at a vector of points from the degree `D` extension field. This is like
    /// `eval_ext`, except in the context of a recursive circuit.
    /// Note: constraints must be added through`yeld_constr.constraint(builder, constraint)` in the
    /// same order as they are given in `eval_packed_generic`.
    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
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
        degree_bits: usize,
        num_ctl_zs: usize,
        config: &StarkConfig,
    ) -> FriInstanceInfo<F, D> {
        let trace_oracle = FriOracleInfo {
            num_polys: Self::COLUMNS,
            blinding: false,
        };
        let trace_info = FriPolynomialInfo::from_range(TRACE_ORACLE_INDEX, 0..Self::COLUMNS);

        let num_permutation_batches = self.num_permutation_batches(config);
        let num_perutation_ctl_polys = num_permutation_batches + num_ctl_zs;
        let permutation_ctl_oracle = FriOracleInfo {
            num_polys: num_perutation_ctl_polys,
            blinding: false,
        };
        let permutation_ctl_zs_info = FriPolynomialInfo::from_range(
            PERMUTATION_CTL_ORACLE_INDEX,
            0..num_perutation_ctl_polys,
        );

        let ctl_zs_info = FriPolynomialInfo::from_range(
            PERMUTATION_CTL_ORACLE_INDEX,
            num_permutation_batches..num_permutation_batches + num_ctl_zs,
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
                permutation_ctl_zs_info.clone(),
                quotient_info,
            ]
            .concat(),
        };
        let zeta_next_batch = FriBatchInfo {
            point: zeta.scalar_mul(g),
            polynomials: [trace_info, permutation_ctl_zs_info].concat(),
        };
        let ctl_last_batch = FriBatchInfo {
            point: F::Extension::primitive_root_of_unity(degree_bits).inverse(),
            polynomials: ctl_zs_info,
        };
        FriInstanceInfo {
            oracles: vec![trace_oracle, permutation_ctl_oracle, quotient_oracle],
            batches: vec![zeta_batch, zeta_next_batch, ctl_last_batch],
        }
    }

    /// Computes the FRI instance used to prove this Stark.
    fn fri_instance_target(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        zeta: ExtensionTarget<D>,
        g: F,
        degree_bits: usize,
        num_ctl_zs: usize,
        inner_config: &StarkConfig,
    ) -> FriInstanceInfoTarget<D> {
        let trace_oracle = FriOracleInfo {
            num_polys: Self::COLUMNS,
            blinding: false,
        };
        let trace_info = FriPolynomialInfo::from_range(TRACE_ORACLE_INDEX, 0..Self::COLUMNS);

        let num_permutation_batches = self.num_permutation_batches(inner_config);
        let num_perutation_ctl_polys = num_permutation_batches + num_ctl_zs;
        let permutation_ctl_oracle = FriOracleInfo {
            num_polys: num_perutation_ctl_polys,
            blinding: false,
        };
        let permutation_ctl_zs_info = FriPolynomialInfo::from_range(
            PERMUTATION_CTL_ORACLE_INDEX,
            0..num_perutation_ctl_polys,
        );

        let ctl_zs_info = FriPolynomialInfo::from_range(
            PERMUTATION_CTL_ORACLE_INDEX,
            num_permutation_batches..num_permutation_batches + num_ctl_zs,
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
                permutation_ctl_zs_info.clone(),
                quotient_info,
            ]
            .concat(),
        };
        let zeta_next = builder.mul_const_extension(g, zeta);
        let zeta_next_batch = FriBatchInfoTarget {
            point: zeta_next,
            polynomials: [trace_info, permutation_ctl_zs_info].concat(),
        };
        let ctl_last_batch = FriBatchInfoTarget {
            point: builder
                .constant_extension(F::Extension::primitive_root_of_unity(degree_bits).inverse()),
            polynomials: ctl_zs_info,
        };
        FriInstanceInfoTarget {
            oracles: vec![trace_oracle, permutation_ctl_oracle, quotient_oracle],
            batches: vec![zeta_batch, zeta_next_batch, ctl_last_batch],
        }
    }

    /// Pairs of lists of columns that should be permutations of one another. A permutation argument
    /// will be used for each such pair. Empty by default.
    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        vec![]
    }

    fn uses_permutation_args(&self) -> bool {
        !self.permutation_pairs().is_empty()
    }

    /// The number of permutation argument instances that can be combined into a single constraint.
    fn permutation_batch_size(&self) -> usize {
        // The permutation argument constraints look like
        //     Z(x) \prod(...) = Z(g x) \prod(...)
        // where each product has a number of terms equal to the batch size. So our batch size
        // should be one less than our constraint degree, which happens to be our quotient degree.
        self.quotient_degree_factor()
    }

    fn num_permutation_instances(&self, config: &StarkConfig) -> usize {
        self.permutation_pairs().len() * config.num_challenges
    }

    fn num_permutation_batches(&self, config: &StarkConfig) -> usize {
        ceil_div_usize(
            self.num_permutation_instances(config),
            self.permutation_batch_size(),
        )
    }
}
