//! Implementation of the [`Stark`] trait that defines the set of constraints
//! related to a statement.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::fri::structure::{
    FriBatchInfo, FriBatchInfoTarget, FriInstanceInfo, FriInstanceInfoTarget, FriOracleInfo,
    FriPolynomialInfo,
};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::config::StarkConfig;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::lookup::Lookup;

/// Represents a STARK system.
pub trait Stark<F: RichField + Extendable<D>, const D: usize>: Sync {
    /// The total number of columns in the trace.
    const COLUMNS: usize = Self::EvaluationFrameTarget::COLUMNS;
    /// The total number of public inputs.
    const PUBLIC_INPUTS: usize = Self::EvaluationFrameTarget::PUBLIC_INPUTS;

    /// This is used to evaluate constraints natively.
    type EvaluationFrame<FE, P, const D2: usize>: StarkEvaluationFrame<P, FE>
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    /// The `Target` version of `Self::EvaluationFrame`, used to evaluate constraints recursively.
    type EvaluationFrameTarget: StarkEvaluationFrame<ExtensionTarget<D>, ExtensionTarget<D>>;

    /// Evaluates constraints at a vector of points.
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

    /// Evaluates constraints at a vector of points from the base field `F`.
    fn eval_packed_base<P: PackedField<Scalar = F>>(
        &self,
        vars: &Self::EvaluationFrame<F, P, 1>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) {
        self.eval_packed_generic(vars, yield_constr)
    }

    /// Evaluates constraints at a single point from the degree `D` extension field.
    fn eval_ext(
        &self,
        vars: &Self::EvaluationFrame<F::Extension, F::Extension, D>,
        yield_constr: &mut ConstraintConsumer<F::Extension>,
    ) {
        self.eval_packed_generic(vars, yield_constr)
    }

    /// Evaluates constraints at a vector of points from the degree `D` extension field.
    /// This is like `eval_ext`, except in the context of a recursive circuit.
    /// Note: constraints must be added through`yield_constr.constraint(builder, constraint)`
    /// in the same order as they are given in `eval_packed_generic`.
    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    );

    /// Outputs the maximum constraint degree of this [`Stark`].
    fn constraint_degree(&self) -> usize;

    /// Outputs the maximum quotient polynomial's degree factor of this [`Stark`].
    fn quotient_degree_factor(&self) -> usize {
        match self.constraint_degree().checked_sub(1) {
            Some(v) => 1.max(v),
            None => 0,
        }
    }

    /// Outputs the number of quotient polynomials this [`Stark`] would require with
    /// the provided [`StarkConfig`]
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
        let mut oracles = vec![];
        let trace_info = FriPolynomialInfo::from_range(oracles.len(), 0..Self::COLUMNS);
        oracles.push(FriOracleInfo {
            num_polys: Self::COLUMNS,
            blinding: false,
        });

        let num_lookup_columns = self.num_lookup_helper_columns(config);
        let num_auxiliary_polys = num_lookup_columns + num_ctl_helpers + num_ctl_zs.len();
        let auxiliary_polys_info = if self.uses_lookups() || self.requires_ctls() {
            let aux_polys = FriPolynomialInfo::from_range(oracles.len(), 0..num_auxiliary_polys);
            oracles.push(FriOracleInfo {
                num_polys: num_auxiliary_polys,
                blinding: false,
            });
            aux_polys
        } else {
            vec![]
        };

        let num_quotient_polys = self.num_quotient_polys(config);
        let quotient_info = if num_quotient_polys > 0 {
            let quotient_polys =
                FriPolynomialInfo::from_range(oracles.len(), 0..num_quotient_polys);
            oracles.push(FriOracleInfo {
                num_polys: num_quotient_polys,
                blinding: false,
            });
            quotient_polys
        } else {
            vec![]
        };

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

        let mut batches = vec![zeta_batch, zeta_next_batch];

        if self.requires_ctls() {
            let ctl_zs_info = FriPolynomialInfo::from_range(
                1, // auxiliary oracle index
                num_lookup_columns + num_ctl_helpers..num_auxiliary_polys,
            );
            let ctl_first_batch = FriBatchInfo {
                point: F::Extension::ONE,
                polynomials: ctl_zs_info,
            };

            batches.push(ctl_first_batch);
        }

        FriInstanceInfo { oracles, batches }
    }

    /// Computes the FRI instance used to prove this Stark.
    fn fri_instance_target(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        zeta: ExtensionTarget<D>,
        g: Target,
        num_ctl_helper_polys: usize,
        num_ctl_zs: usize,
        config: &StarkConfig,
    ) -> FriInstanceInfoTarget<D> {
        let mut oracles = vec![];
        let trace_info = FriPolynomialInfo::from_range(oracles.len(), 0..Self::COLUMNS);
        oracles.push(FriOracleInfo {
            num_polys: Self::COLUMNS,
            blinding: false,
        });

        let num_lookup_columns = self.num_lookup_helper_columns(config);
        let num_auxiliary_polys = num_lookup_columns + num_ctl_helper_polys + num_ctl_zs;
        let auxiliary_polys_info = if self.uses_lookups() || self.requires_ctls() {
            let aux_polys = FriPolynomialInfo::from_range(oracles.len(), 0..num_auxiliary_polys);
            oracles.push(FriOracleInfo {
                num_polys: num_auxiliary_polys,
                blinding: false,
            });
            aux_polys
        } else {
            vec![]
        };

        let num_quotient_polys = self.num_quotient_polys(config);
        let quotient_info = if num_quotient_polys > 0 {
            let quotient_polys =
                FriPolynomialInfo::from_range(oracles.len(), 0..num_quotient_polys);
            oracles.push(FriOracleInfo {
                num_polys: num_quotient_polys,
                blinding: false,
            });
            quotient_polys
        } else {
            vec![]
        };

        let zeta_batch = FriBatchInfoTarget {
            point: zeta,
            polynomials: [
                trace_info.clone(),
                auxiliary_polys_info.clone(),
                quotient_info,
            ]
            .concat(),
        };
        let g_ext = builder.convert_to_ext(g);
        let zeta_next = builder.mul_extension(g_ext, zeta);
        let zeta_next_batch = FriBatchInfoTarget {
            point: zeta_next,
            polynomials: [trace_info, auxiliary_polys_info].concat(),
        };

        let mut batches = vec![zeta_batch, zeta_next_batch];

        if self.requires_ctls() {
            let ctl_zs_info = FriPolynomialInfo::from_range(
                1, // auxiliary oracle index
                num_lookup_columns + num_ctl_helper_polys..num_auxiliary_polys,
            );
            let ctl_first_batch = FriBatchInfoTarget {
                point: builder.one_extension(),
                polynomials: ctl_zs_info,
            };

            batches.push(ctl_first_batch);
        }

        FriInstanceInfoTarget { oracles, batches }
    }

    /// Outputs all the [`Lookup`] this STARK table needs to perform across its columns.
    fn lookups(&self) -> Vec<Lookup<F>> {
        vec![]
    }

    /// Outputs the number of total lookup helper columns, based on this STARK's vector
    /// of [`Lookup`] and the number of challenges used by this [`StarkConfig`].
    fn num_lookup_helper_columns(&self, config: &StarkConfig) -> usize {
        self.lookups()
            .iter()
            .map(|lookup| lookup.num_helper_columns(self.constraint_degree()))
            .sum::<usize>()
            * config.num_challenges
    }

    /// Indicates whether this STARK uses lookups over some of its columns, and as such requires
    /// additional steps during proof generation to handle auxiliary polynomials.
    fn uses_lookups(&self) -> bool {
        !self.lookups().is_empty()
    }

    /// Indicates whether this STARK belongs to a multi-STARK system, and as such may require
    /// cross-table lookups to connect shared values across different traces.
    ///
    /// It defaults to `false`, i.e. for simple uni-STARK systems.
    fn requires_ctls(&self) -> bool {
        false
    }
}
