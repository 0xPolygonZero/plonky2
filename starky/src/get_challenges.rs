#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::cmp::min;

use plonky2::field::extension::Extendable;
use plonky2::field::polynomial::PolynomialCoeffs;
use plonky2::field::types::Field;
use plonky2::fri::proof::{FriProof, FriProofTarget};
use plonky2::fri::prover::final_poly_coeff_len;
use plonky2::fri::FriParams;
use plonky2::gadgets::polynomial::PolynomialCoeffsExtTarget;
use plonky2::hash::hash_types::{MerkleCapTarget, RichField};
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2_util::log2_ceil;

use crate::config::StarkConfig;
use crate::cross_table_lookup::{CtlCheckVars, CtlCheckVarsTarget};
use crate::lookup::{
    get_grand_product_challenge_set, get_grand_product_challenge_set_target,
    GrandProductChallengeSet,
};
use crate::proof::*;
use crate::stark::Stark;
use crate::vanishing_poly::{compute_eval_vanishing_poly, compute_eval_vanishing_poly_circuit};

/// Generates challenges for a STARK proof from a challenger and given
/// all the arguments needed to update the challenger state.
///
/// Note: `trace_cap` is passed as `Option` to signify whether to observe it
/// or not by the challenger. Observing it here could be redundant in a
/// multi-STARK system where trace caps would have already been observed
/// before proving individually each STARK.
fn get_challenges<F, C, S: Stark<F, D>, const D: usize>(
    stark: &S,
    public_inputs: &[F],
    challenger: &mut Challenger<F, C::Hasher>,
    challenges: Option<&GrandProductChallengeSet<F>>,
    trace_cap: Option<&MerkleCap<F, C::Hasher>>,
    auxiliary_polys_cap: Option<&MerkleCap<F, C::Hasher>>,
    quotient_polys_cap: Option<&MerkleCap<F, C::Hasher>>,
    ctl_vars: Option<&[CtlCheckVars<F, F::Extension, F::Extension, D>]>,
    openings: &StarkOpeningSet<F, D>,
    commit_phase_merkle_caps: &[MerkleCap<F, C::Hasher>],
    final_poly: &PolynomialCoeffs<F::Extension>,
    pow_witness: F,
    config: &StarkConfig,
    degree_bits: usize,
    verifier_circuit_fri_params: Option<FriParams>,
) -> StarkProofChallenges<F, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let num_challenges = config.num_challenges;

    // Observe the config
    config.observe(challenger);

    if let Some(cap) = &trace_cap {
        challenger.observe_cap(cap);
    }

    let lookup_challenge_set = if let Some(&challenges) = challenges.as_ref() {
        Some(challenges.clone())
    } else {
        auxiliary_polys_cap
            .is_some()
            .then(|| get_grand_product_challenge_set(challenger, num_challenges))
    };

    if let Some(cap) = &auxiliary_polys_cap {
        challenger.observe_cap(cap);
    }

    let num_lookup_columns = stark.num_lookup_helper_columns(config);
    let lookup_challenges = if stark.uses_lookups() {
        Some(
            lookup_challenge_set
                .as_ref()
                .unwrap()
                .challenges
                .iter()
                .map(|ch| ch.beta)
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };

    // Before computing the quotient polynomial, we use grinding to "bind" the constraints with high probability.
    // To do so, we get random challenges to represent the trace, auxiliary and CTL polynomials.
    // We evaluate the constraints using those random values and combine them with `stark_alphas_prime`.
    // Then, the challenger observes the resulting evaluations, so that the constraints are bound to `stark_alphas`
    // (the challenges used in the quotient polynomials).
    let stark_alphas_prime = challenger.get_n_challenges(num_challenges);

    // First power unreachable by the constraints.
    let pow_degree = core::cmp::max(2, stark.constraint_degree() + 1);

    let poly_evals = get_dummy_polys::<F, C, D>(
        challenger,
        S::COLUMNS,
        openings.auxiliary_polys.as_ref().map_or(0, |aux| aux.len()),
        pow_degree,
    );

    // Get dummy ctl_vars.
    let total_num_ctl_polys: usize = ctl_vars
        .map(|ctls| ctls.iter().map(|ctl| ctl.helper_columns.len()).sum())
        .unwrap_or_default();
    let ctl_vars_poly_evals = ctl_vars.map(|ctl_z| {
        let mut start_index = 0;
        ctl_z
            .iter()
            .enumerate()
            .map(|(i, ctl_check_vars)| {
                let num_ctl_helper_cols = ctl_check_vars.helper_columns.len();
                let helper_columns =
                    poly_evals.auxiliary_polys.as_ref().unwrap()[num_lookup_columns + start_index
                        ..num_lookup_columns + start_index + num_ctl_helper_cols]
                        .to_vec();

                let ctl_vars = CtlCheckVars::<F, F::Extension, F::Extension, D> {
                    helper_columns,
                    local_z: poly_evals.auxiliary_polys.as_ref().unwrap()
                        [num_lookup_columns + total_num_ctl_polys + i],
                    next_z: poly_evals.auxiliary_polys_next.as_ref().unwrap()
                        [num_lookup_columns + total_num_ctl_polys + i],
                    challenges: ctl_check_vars.challenges,
                    columns: ctl_check_vars.columns.clone(),
                    filter: ctl_check_vars.filter.clone(),
                };

                start_index += num_ctl_helper_cols;

                ctl_vars
            })
            .collect::<Vec<_>>()
    });

    let zeta_prime = challenger.get_extension_challenge();

    // Bind constraints.
    let constraint_evals = compute_eval_vanishing_poly::<F, S, D>(
        stark,
        &poly_evals,
        ctl_vars_poly_evals.as_deref(),
        lookup_challenges.as_ref(),
        &stark.lookups(),
        public_inputs,
        stark_alphas_prime.clone(),
        zeta_prime,
        degree_bits,
        num_lookup_columns,
    );

    challenger.observe_extension_elements(&constraint_evals);

    let stark_alphas = challenger.get_n_challenges(num_challenges);

    if let Some(quotient_polys_cap) = quotient_polys_cap {
        challenger.observe_cap(quotient_polys_cap);
    }
    let stark_zeta = challenger.get_extension_challenge::<D>();

    challenger.observe_openings(&openings.to_fri_openings());

    let (final_poly_coeff_len, max_num_query_steps) =
        if let Some(verifier_circuit_fri_params) = verifier_circuit_fri_params {
            (
                Some(final_poly_coeff_len(
                    verifier_circuit_fri_params.degree_bits,
                    &verifier_circuit_fri_params.reduction_arity_bits,
                )),
                Some(verifier_circuit_fri_params.reduction_arity_bits.len()),
            )
        } else {
            (None, None)
        };

    StarkProofChallenges {
        lookup_challenge_set,
        stark_alphas,
        stark_zeta,
        fri_challenges: challenger.fri_challenges::<C, D>(
            commit_phase_merkle_caps,
            final_poly,
            pow_witness,
            degree_bits,
            &config.fri_config,
            final_poly_coeff_len,
            max_num_query_steps,
        ),
    }
}

/// Simulate the trace, ctl, and auxiliary polynomials using dummy values. This is used to bind the constraints before committing to the quotient polynomial.
fn get_dummy_polys<F, C, const D: usize>(
    challenger: &mut Challenger<F, C::Hasher>,
    num_trace_polys: usize,
    num_aux_polys: usize,
    pow_degree: usize,
) -> StarkOpeningSet<F, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let log_pow_degree = log2_ceil(pow_degree);
    let num_extension_powers = core::cmp::max(1, 50 / log_pow_degree - 1);

    let total_num_dummy_extension_evals = num_trace_polys * 2 + num_aux_polys * 2; // for auxiliary_polys and auxiliary_polys_next

    // Get extension field challenges that will simulate the trace, ctl, and auxiliary polynomials.
    // Since sampling challenges for all polynomials might be heavy, we sample enough challenges {c_i}_i and use:
    // c_i, c_i^{pow_degree}, ..., c_i^{pow_degree * 50} as simulated values.
    let simulating_zetas = challenger
        .get_n_extension_challenges(total_num_dummy_extension_evals.div_ceil(num_extension_powers));

    // For each zeta in zetas, we compute the powers z^{(constraint_degree + 1)^i} for i = 0..num_extension_powers.
    let nb_dummy_per_zeta = min(num_extension_powers + 1, total_num_dummy_extension_evals);
    let dummy_extension_evals = simulating_zetas
        .into_iter()
        .flat_map(|zeta: F::Extension| {
            core::iter::successors(Some(zeta), move |prev| {
                Some(prev.exp_u64(pow_degree as u64))
            })
            .take(nb_dummy_per_zeta)
        })
        .collect::<Vec<_>>();

    let next_values_start = num_trace_polys;
    let auxiliary_polys_start = num_trace_polys * 2;
    let auxiliary_polys_next_start = auxiliary_polys_start + num_aux_polys;
    let is_aux_polys = num_aux_polys > 0;

    StarkOpeningSet {
        local_values: dummy_extension_evals[..next_values_start].to_vec(),
        next_values: dummy_extension_evals[next_values_start..auxiliary_polys_start].to_vec(),
        auxiliary_polys: if is_aux_polys {
            Some(dummy_extension_evals[auxiliary_polys_start..auxiliary_polys_next_start].to_vec())
        } else {
            None
        },
        auxiliary_polys_next: if is_aux_polys {
            Some(dummy_extension_evals[auxiliary_polys_next_start..].to_vec())
        } else {
            None
        },
        ctl_zs_first: None,   // Unused in the constraints.
        quotient_polys: None, // We don't need to simulate the quotient polynomials.
    }
}

impl<F, C, const D: usize> StarkProof<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    /// For a single STARK system, the `ignore_trace_cap` boolean should
    /// always be set to `false`.
    ///
    /// Multi-STARK systems may already observe individual trace caps
    /// ahead of proving each table, and hence may ignore observing
    /// again the cap when generating individual challenges.
    pub fn get_challenges<S: Stark<F, D>>(
        &self,
        stark: &S,
        public_inputs: &[F],
        challenger: &mut Challenger<F, C::Hasher>,
        challenges: Option<&GrandProductChallengeSet<F>>,
        ctl_vars: Option<&[CtlCheckVars<F, F::Extension, F::Extension, D>]>,
        ignore_trace_cap: bool,
        config: &StarkConfig,
        verifier_circuit_fri_params: Option<FriParams>,
    ) -> StarkProofChallenges<F, D> {
        let degree_bits = self.recover_degree_bits(config);

        let StarkProof {
            trace_cap,
            auxiliary_polys_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProof {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
        } = &self;

        let trace_cap = if ignore_trace_cap {
            None
        } else {
            Some(trace_cap)
        };

        get_challenges::<F, C, S, D>(
            stark,
            public_inputs,
            challenger,
            challenges,
            trace_cap,
            auxiliary_polys_cap.as_ref(),
            quotient_polys_cap.as_ref(),
            ctl_vars,
            openings,
            commit_phase_merkle_caps,
            final_poly,
            *pow_witness,
            config,
            degree_bits,
            verifier_circuit_fri_params,
        )
    }
}

impl<F, C, const D: usize> StarkProofWithPublicInputs<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    /// For a single STARK system, the `ignore_trace_cap` boolean should
    /// always be set to `false`.
    ///
    /// Multi-STARK systems may already observe individual trace caps
    /// ahead of proving each table, and hence may ignore observing
    /// again the cap when generating individual challenges.
    pub fn get_challenges<S: Stark<F, D>>(
        &self,
        stark: &S,
        challenger: &mut Challenger<F, C::Hasher>,
        challenges: Option<&GrandProductChallengeSet<F>>,
        ctl_vars: Option<&[CtlCheckVars<F, F::Extension, F::Extension, D>]>,
        ignore_trace_cap: bool,
        config: &StarkConfig,
        verifier_circuit_fri_params: Option<FriParams>,
    ) -> StarkProofChallenges<F, D> {
        challenger.observe_elements(&self.public_inputs);
        self.proof.get_challenges::<S>(
            stark,
            &self.public_inputs,
            challenger,
            challenges,
            ctl_vars,
            ignore_trace_cap,
            config,
            verifier_circuit_fri_params,
        )
    }
}

/// Circuit version of `get_challenges`, with the same flexibility around
/// `trace_cap` being passed as an `Option`.
fn get_challenges_target<F, C, S: Stark<F, D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    public_inputs: &[Target],
    challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
    challenges: Option<&GrandProductChallengeSet<Target>>,
    trace_cap: Option<&MerkleCapTarget>,
    auxiliary_polys_cap: Option<&MerkleCapTarget>,
    quotient_polys_cap: Option<&MerkleCapTarget>,
    ctl_vars: Option<&[CtlCheckVarsTarget<F, D>]>,
    openings: &StarkOpeningSetTarget<D>,
    commit_phase_merkle_caps: &[MerkleCapTarget],
    final_poly: &PolynomialCoeffsExtTarget<D>,
    pow_witness: Target,
    degree_bits: usize,
    degree_bits_target: Target,
    config: &StarkConfig,
) -> StarkProofChallengesTarget<D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    let num_challenges = config.num_challenges;

    // Observe the config
    config.observe_target(builder, challenger);

    if let Some(trace_cap) = trace_cap {
        challenger.observe_cap(trace_cap);
    }

    let lookup_challenge_set = if let Some(&challenges) = challenges.as_ref() {
        Some(challenges.clone())
    } else {
        auxiliary_polys_cap
            .is_some()
            .then(|| get_grand_product_challenge_set_target(builder, challenger, num_challenges))
    };

    if let Some(cap) = auxiliary_polys_cap {
        challenger.observe_cap(cap);
    }

    let num_lookup_columns = stark.num_lookup_helper_columns(config);
    let lookup_challenges = stark.uses_lookups().then(|| {
        lookup_challenge_set
            .as_ref()
            .unwrap()
            .challenges
            .iter()
            .map(|ch| ch.beta)
            .collect::<Vec<_>>()
    });

    // Before computing the quotient polynomial, we use grinding to "bind" the constraints with high probability.
    // To do so, we get random challenges to represent the trace, auxiliary and CTL polynomials.
    // We evaluate the constraints using those random values and combine them with `stark_alphas_prime`.
    // Then, the challenger observes the resulting evaluations, so that the constraints are bound to `stark_alphas`
    // (the challenges used in the quotient polynomials).
    let stark_alphas_prime = challenger.get_n_challenges(builder, num_challenges);

    let pow_degree = core::cmp::max(2, stark.constraint_degree() + 1);
    let poly_evals = get_dummy_polys_circuit::<F, C, D>(
        builder,
        challenger,
        S::COLUMNS,
        openings.auxiliary_polys.as_ref().map_or(0, |aux| aux.len()),
        pow_degree,
    );

    // Get dummy ctl_vars.
    let total_num_ctl_polys: usize = ctl_vars
        .map(|ctls| ctls.iter().map(|ctl| ctl.helper_columns.len()).sum())
        .unwrap_or_default();
    let ctl_vars_poly_evals = ctl_vars.map(|ctl_z| {
        let mut start_index = 0;
        ctl_z
            .iter()
            .enumerate()
            .map(|(i, ctl_check_vars)| {
                let num_ctl_helper_cols = ctl_check_vars.helper_columns.len();
                let helper_columns =
                    poly_evals.auxiliary_polys.as_ref().unwrap()[num_lookup_columns + start_index
                        ..num_lookup_columns + start_index + num_ctl_helper_cols]
                        .to_vec();

                let ctl_vars = CtlCheckVarsTarget::<F, D> {
                    helper_columns,
                    local_z: poly_evals.auxiliary_polys.as_ref().unwrap()
                        [num_lookup_columns + total_num_ctl_polys + i],
                    next_z: poly_evals.auxiliary_polys_next.as_ref().unwrap()
                        [num_lookup_columns + total_num_ctl_polys + i],
                    challenges: ctl_check_vars.challenges,
                    columns: ctl_check_vars.columns.clone(),
                    filter: ctl_check_vars.filter.clone(),
                };

                start_index += num_ctl_helper_cols;

                ctl_vars
            })
            .collect::<Vec<_>>()
    });

    let zeta_prime = challenger.get_extension_challenge(builder);

    // Bind constraints.
    let constraint_evals = compute_eval_vanishing_poly_circuit::<F, S, D>(
        builder,
        stark,
        &poly_evals,
        ctl_vars_poly_evals.as_deref(),
        lookup_challenges.as_ref(),
        public_inputs,
        stark_alphas_prime.clone(),
        zeta_prime,
        degree_bits,
        degree_bits_target,
        num_lookup_columns,
    );

    challenger.observe_extension_elements(&constraint_evals);

    let stark_alphas = challenger.get_n_challenges(builder, num_challenges);

    if let Some(cap) = quotient_polys_cap {
        challenger.observe_cap(cap);
    }

    let stark_zeta = challenger.get_extension_challenge(builder);

    challenger.observe_openings(&openings.to_fri_openings(builder.zero()));

    StarkProofChallengesTarget {
        lookup_challenge_set,
        stark_alphas,
        stark_zeta,
        fri_challenges: challenger.fri_challenges(
            builder,
            commit_phase_merkle_caps,
            final_poly,
            pow_witness,
            &config.fri_config,
        ),
    }
}

// Simulate the trace, ctl, and auxiliary polynomials using dummy values. This is used to bind the constraints before committing to the quotient polynomial.
fn get_dummy_polys_circuit<F, C, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
    num_trace_polys: usize,
    num_aux_polys: usize,
    pow_degree: usize,
) -> StarkOpeningSetTarget<D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<F>,
{
    let log_pow_degree = log2_ceil(pow_degree);
    let num_extension_powers = core::cmp::max(1, 50 / log_pow_degree - 1);

    let total_num_dummy_extension_evals = num_trace_polys * 2 + num_aux_polys * 2; // for auxiliary_polys and auxiliary_polys_next

    // Get extension field challenges that will simulate the trace, ctl, and auxiliary polynomials.
    // Since sampling challenges for all polynomials might be heavy, we sample enough challenges {c_i}_i and use:
    // c_i, c_i^{pow_degree}, ..., c_i^{pow_degree * 50} as simulated values.
    let simulating_zetas = challenger.get_n_extension_challenges(
        builder,
        total_num_dummy_extension_evals.div_ceil(num_extension_powers),
    );

    // For each zeta in zetas, we compute the powers z^{(constraint_degree + 1)^i} for i = 0..num_extension_powers.
    let nb_dummy_per_zeta = min(num_extension_powers + 1, total_num_dummy_extension_evals);
    let dummy_extension_evals = simulating_zetas
        .into_iter()
        .flat_map(|zeta| {
            let mut powers = Vec::with_capacity(num_extension_powers);
            powers.push(zeta);
            let mut pow_val = zeta;
            for _ in 1..nb_dummy_per_zeta {
                pow_val = builder.exp_u64_extension(pow_val, pow_degree as u64);
                powers.push(pow_val);
            }
            powers
        })
        .collect::<Vec<_>>();

    let next_values_start = num_trace_polys;
    let auxiliary_polys_start = num_trace_polys * 2;
    let auxiliary_polys_next_start = auxiliary_polys_start + num_aux_polys;
    let is_aux_polys = num_aux_polys > 0;

    StarkOpeningSetTarget {
        local_values: dummy_extension_evals[..next_values_start].to_vec(),
        next_values: dummy_extension_evals[next_values_start..auxiliary_polys_start].to_vec(),
        auxiliary_polys: if is_aux_polys {
            Some(dummy_extension_evals[auxiliary_polys_start..auxiliary_polys_next_start].to_vec())
        } else {
            None
        },
        auxiliary_polys_next: if is_aux_polys {
            Some(dummy_extension_evals[auxiliary_polys_next_start..].to_vec())
        } else {
            None
        },
        ctl_zs_first: None,
        quotient_polys: None, // We don't need to simulate the quotient polynomials.
    }
}

impl<const D: usize> StarkProofTarget<D> {
    /// Creates all Fiat-Shamir `Target` challenges used in the STARK proof.
    /// For a single STARK system, the `ignore_trace_cap` boolean should
    /// always be set to `false`.
    ///
    /// Multi-STARK systems may already observe individual trace caps
    /// ahead of proving each table, and hence may ignore observing
    /// again the cap when generating individual challenges.
    pub fn get_challenges<F, C, S: Stark<F, D>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        stark: &S,
        public_inputs: &[Target],
        challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
        challenges: Option<&GrandProductChallengeSet<Target>>,
        ctl_vars: Option<&[CtlCheckVarsTarget<F, D>]>,
        degree_bits: usize,
        ignore_trace_cap: bool,
        config: &StarkConfig,
    ) -> StarkProofChallengesTarget<D>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        C::Hasher: AlgebraicHasher<F>,
    {
        let StarkProofTarget {
            trace_cap,
            auxiliary_polys_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProofTarget {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
            ..
        } = self;

        let trace_cap = if ignore_trace_cap {
            None
        } else {
            Some(trace_cap)
        };

        get_challenges_target::<F, C, S, D>(
            builder,
            stark,
            public_inputs,
            challenger,
            challenges,
            trace_cap,
            auxiliary_polys_cap.as_ref(),
            quotient_polys_cap.as_ref(),
            ctl_vars,
            openings,
            commit_phase_merkle_caps,
            final_poly,
            *pow_witness,
            degree_bits,
            self.degree_bits,
            config,
        )
    }
}

impl<const D: usize> StarkProofWithPublicInputsTarget<D> {
    /// Creates all Fiat-Shamir `Target` challenges used in the STARK proof.
    /// For a single STARK system, the `ignore_trace_cap` boolean should
    /// always be set to `false`.
    ///
    /// Multi-STARK systems may already observe individual trace caps
    /// ahead of proving each table, and hence may ignore observing
    /// again the cap when generating individual challenges.
    pub fn get_challenges<F, C, S: Stark<F, D>>(
        &self,
        stark: &S,
        builder: &mut CircuitBuilder<F, D>,
        challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
        challenges: Option<&GrandProductChallengeSet<Target>>,
        ctl_vars: Option<&[CtlCheckVarsTarget<F, D>]>,
        degree_bits: usize,
        ignore_trace_cap: bool,
        config: &StarkConfig,
    ) -> StarkProofChallengesTarget<D>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        C::Hasher: AlgebraicHasher<F>,
    {
        challenger.observe_elements(&self.public_inputs);
        self.proof.get_challenges::<F, C, S>(
            builder,
            stark,
            &self.public_inputs,
            challenger,
            challenges,
            ctl_vars,
            degree_bits,
            ignore_trace_cap,
            config,
        )
    }
}
