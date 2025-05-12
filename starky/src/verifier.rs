//! Implementation of the STARK verifier.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::any::type_name;
use core::iter::once;

use anyhow::{anyhow, ensure, Result};
use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::types::Field;
use plonky2::fri::verifier::verify_fri_proof;
use plonky2::fri::FriParams;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::iop::challenger::Challenger;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::plonk_common::reduce_with_powers;

use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::cross_table_lookup::CtlCheckVars;
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::lookup::LookupCheckVars;
use crate::proof::{StarkOpeningSet, StarkProof, StarkProofChallenges, StarkProofWithPublicInputs};
use crate::stark::Stark;
use crate::vanishing_poly::{eval_l_0_and_l_last, eval_vanishing_poly};

/// Verifies a [`StarkProofWithPublicInputs`] against a STARK statement.
pub fn verify_stark_proof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    stark: S,
    proof_with_pis: StarkProofWithPublicInputs<F, C, D>,
    config: &StarkConfig,
    verifier_circuit_fri_params: Option<FriParams>,
) -> Result<()> {
    ensure!(proof_with_pis.public_inputs.len() == S::PUBLIC_INPUTS);
    let mut challenger = Challenger::<F, C::Hasher>::new();

    let challenges = proof_with_pis.get_challenges(
        &stark,
        &mut challenger,
        None,
        None,
        false,
        config,
        verifier_circuit_fri_params,
    );

    verify_stark_proof_with_challenges(
        &stark,
        &proof_with_pis.proof,
        &challenges,
        None,
        &proof_with_pis.public_inputs,
        config,
    )
}

/// Verifies a [`StarkProofWithPublicInputs`] against a STARK statement,
/// with the provided [`StarkProofChallenges`].
/// It also supports optional cross-table lookups data and challenges,
/// in case this proof is part of a multi-STARK system.
pub fn verify_stark_proof_with_challenges<F, C, S, const D: usize>(
    stark: &S,
    proof: &StarkProof<F, C, D>,
    challenges: &StarkProofChallenges<F, D>,
    ctl_vars: Option<&[CtlCheckVars<F, F::Extension, F::Extension, D>]>,
    public_inputs: &[F],
    config: &StarkConfig,
) -> Result<()>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    log::debug!("Checking proof: {}", type_name::<S>());

    let (num_ctl_z_polys, num_ctl_polys) = ctl_vars
        .map(|ctls| {
            (
                ctls.len(),
                ctls.iter().map(|ctl| ctl.helper_columns.len()).sum(),
            )
        })
        .unwrap_or_default();

    validate_proof_shape(
        stark,
        proof,
        public_inputs,
        config,
        num_ctl_polys,
        num_ctl_z_polys,
    )?;

    let StarkOpeningSet {
        local_values,
        next_values,
        auxiliary_polys,
        auxiliary_polys_next,
        ctl_zs_first: _,
        quotient_polys,
    } = &proof.openings;

    let vars = S::EvaluationFrame::from_values(
        local_values,
        next_values,
        &public_inputs
            .iter()
            .copied()
            .map(F::Extension::from_basefield)
            .collect::<Vec<_>>(),
    );

    let degree_bits = proof.recover_degree_bits(config);
    let (l_0, l_last) = eval_l_0_and_l_last(degree_bits, challenges.stark_zeta);
    let last = F::primitive_root_of_unity(degree_bits).inverse();
    let z_last = challenges.stark_zeta - last.into();

    let mut consumer = ConstraintConsumer::<F::Extension>::new(
        challenges
            .stark_alphas
            .iter()
            .map(|&alpha| F::Extension::from_basefield(alpha))
            .collect::<Vec<_>>(),
        z_last,
        l_0,
        l_last,
    );

    let num_lookup_columns = stark.num_lookup_helper_columns(config);
    let lookup_challenges = if stark.uses_lookups() {
        Some(
            challenges
                .lookup_challenge_set
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

    let lookup_vars = stark.uses_lookups().then(|| LookupCheckVars {
        local_values: auxiliary_polys.as_ref().unwrap()[..num_lookup_columns].to_vec(),
        next_values: auxiliary_polys_next.as_ref().unwrap()[..num_lookup_columns].to_vec(),
        challenges: lookup_challenges.unwrap(),
    });
    let lookups = stark.lookups();

    eval_vanishing_poly::<F, F::Extension, F::Extension, S, D, D>(
        stark,
        &vars,
        &lookups,
        lookup_vars,
        ctl_vars,
        &mut consumer,
    );
    let vanishing_polys_zeta = consumer.accumulators();

    // Check each polynomial identity, of the form `vanishing(x) = Z_H(x) quotient(x)`, at zeta.
    let zeta_pow_deg = challenges.stark_zeta.exp_power_of_2(degree_bits);
    let z_h_zeta = zeta_pow_deg - F::Extension::ONE;
    // `quotient_polys_zeta` holds `num_challenges * quotient_degree_factor` evaluations.
    // Each chunk of `quotient_degree_factor` holds the evaluations of `t_0(zeta),...,t_{quotient_degree_factor-1}(zeta)`
    // where the "real" quotient polynomial is `t(X) = t_0(X) + t_1(X)*X^n + t_2(X)*X^{2n} + ...`.
    // So to reconstruct `t(zeta)` we can compute `reduce_with_powers(chunk, zeta^n)` for each
    // `quotient_degree_factor`-sized chunk of the original evaluations.

    for (i, chunk) in quotient_polys
        .iter()
        .flat_map(|x| x.chunks(stark.quotient_degree_factor()))
        .enumerate()
    {
        ensure!(
            vanishing_polys_zeta[i] == z_h_zeta * reduce_with_powers(chunk, zeta_pow_deg),
            "Mismatch between evaluation and opening of quotient polynomial"
        );
    }

    let merkle_caps = once(proof.trace_cap.clone())
        .chain(proof.auxiliary_polys_cap.clone())
        .chain(proof.quotient_polys_cap.clone())
        .collect_vec();

    let num_ctl_zs = ctl_vars
        .map(|vars| {
            vars.iter()
                .map(|ctl| ctl.helper_columns.len())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    verify_fri_proof::<F, C, D>(
        &stark.fri_instance(
            challenges.stark_zeta,
            F::primitive_root_of_unity(degree_bits),
            num_ctl_polys,
            num_ctl_zs,
            config,
        ),
        &proof.openings.to_fri_openings(),
        &challenges.fri_challenges,
        &merkle_caps,
        &proof.opening_proof,
        &config.fri_params(degree_bits),
    )?;

    Ok(())
}

fn validate_proof_shape<F, C, S, const D: usize>(
    stark: &S,
    proof: &StarkProof<F, C, D>,
    public_inputs: &[F],
    config: &StarkConfig,
    num_ctl_helpers: usize,
    num_ctl_zs: usize,
) -> anyhow::Result<()>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    let degree_bits = proof.recover_degree_bits(config);

    let StarkProof {
        trace_cap,
        auxiliary_polys_cap,
        quotient_polys_cap,
        openings,
        // The shape of the opening proof will be checked in the FRI verifier (see
        // validate_fri_proof_shape), so we ignore it here.
        opening_proof: _,
    } = proof;

    let StarkOpeningSet {
        local_values,
        next_values,
        auxiliary_polys,
        auxiliary_polys_next,
        ctl_zs_first,
        quotient_polys,
    } = openings;

    ensure!(public_inputs.len() == S::PUBLIC_INPUTS);

    let fri_params = config.fri_params(degree_bits);
    let cap_height = fri_params.config.cap_height;

    ensure!(trace_cap.height() == cap_height);
    ensure!(
        quotient_polys_cap.is_none()
            || quotient_polys_cap.as_ref().map(|q| q.height()) == Some(cap_height)
    );

    ensure!(local_values.len() == S::COLUMNS);
    ensure!(next_values.len() == S::COLUMNS);
    ensure!(if let Some(quotient_polys) = quotient_polys {
        quotient_polys.len() == stark.num_quotient_polys(config)
    } else {
        stark.num_quotient_polys(config) == 0
    });

    check_lookup_options::<F, C, S, D>(
        stark,
        auxiliary_polys_cap,
        auxiliary_polys,
        auxiliary_polys_next,
        num_ctl_helpers,
        num_ctl_zs,
        ctl_zs_first,
        config,
    )?;

    Ok(())
}

/// Utility function to check that all lookups data wrapped in `Option`s are `Some` iff
/// the STARK uses a permutation argument.
fn check_lookup_options<F, C, S, const D: usize>(
    stark: &S,
    auxiliary_polys_cap: &Option<MerkleCap<F, <C as GenericConfig<D>>::Hasher>>,
    auxiliary_polys: &Option<Vec<<F as Extendable<D>>::Extension>>,
    auxiliary_polys_next: &Option<Vec<<F as Extendable<D>>::Extension>>,
    num_ctl_helpers: usize,
    num_ctl_zs: usize,
    ctl_zs_first: &Option<Vec<F>>,
    config: &StarkConfig,
) -> Result<()>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
{
    if stark.uses_lookups() || stark.requires_ctls() {
        let num_auxiliary = stark.num_lookup_helper_columns(config) + num_ctl_helpers + num_ctl_zs;
        let cap_height = config.fri_config.cap_height;

        let auxiliary_polys_cap = auxiliary_polys_cap
            .as_ref()
            .ok_or_else(|| anyhow!("Missing auxiliary_polys_cap"))?;
        let auxiliary_polys = auxiliary_polys
            .as_ref()
            .ok_or_else(|| anyhow!("Missing auxiliary_polys"))?;
        let auxiliary_polys_next = auxiliary_polys_next
            .as_ref()
            .ok_or_else(|| anyhow!("Missing auxiliary_polys_next"))?;

        if let Some(ctl_zs_first) = ctl_zs_first {
            ensure!(ctl_zs_first.len() == num_ctl_zs);
        }

        ensure!(auxiliary_polys_cap.height() == cap_height);
        ensure!(auxiliary_polys.len() == num_auxiliary);
        ensure!(auxiliary_polys_next.len() == num_auxiliary);
    } else {
        ensure!(auxiliary_polys_cap.is_none());
        ensure!(auxiliary_polys.is_none());
        ensure!(auxiliary_polys_next.is_none());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::field::types::Sample;

    use crate::verifier::eval_l_0_and_l_last;

    #[test]
    fn test_eval_l_0_and_l_last() {
        type F = GoldilocksField;
        let log_n = 5;
        let n = 1 << log_n;

        let x = F::rand(); // challenge point
        let expected_l_first_x = PolynomialValues::selector(n, 0).ifft().eval(x);
        let expected_l_last_x = PolynomialValues::selector(n, n - 1).ifft().eval(x);

        let (l_first_x, l_last_x) = eval_l_0_and_l_last(log_n, x);
        assert_eq!(l_first_x, expected_l_first_x);
        assert_eq!(l_last_x, expected_l_last_x);
    }
}
