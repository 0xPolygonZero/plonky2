//! Implementation of the STARK recursive verifier, i.e. where proof
//! verification if encoded in a plonky2 circuit.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::iter::once;

use anyhow::{ensure, Result};
use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::fri::witness_util::set_fri_proof_target;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::RecursiveChallenger;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::target::Target;
use plonky2::iop::witness::WitnessWrite;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2::util::reducing::ReducingFactorTarget;
use plonky2::with_context;
use plonky2_util::log2_ceil;

use crate::config::StarkConfig;
use crate::constraint_consumer::RecursiveConstraintConsumer;
use crate::cross_table_lookup::CtlCheckVarsTarget;
use crate::evaluation_frame::StarkEvaluationFrame;
use crate::lookup::LookupCheckVarsTarget;
use crate::proof::{
    StarkOpeningSetTarget, StarkProof, StarkProofChallengesTarget, StarkProofTarget,
    StarkProofWithPublicInputs, StarkProofWithPublicInputsTarget,
};
use crate::stark::Stark;
use crate::vanishing_poly::eval_vanishing_poly_circuit;

/// Encodes the verification of a [`StarkProofWithPublicInputsTarget`]
/// for some statement in a circuit.
pub fn verify_stark_proof_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: S,
    proof_with_pis: StarkProofWithPublicInputsTarget<D>,
    inner_config: &StarkConfig,
    degree_bits: usize,
) where
    C::Hasher: AlgebraicHasher<F>,
{
    assert_eq!(proof_with_pis.public_inputs.len(), S::PUBLIC_INPUTS);

    let mut challenger = RecursiveChallenger::<F, C::Hasher, D>::new(builder);
    let challenges = with_context!(
        builder,
        "compute challenges",
        proof_with_pis.get_challenges::<F, C>(builder, &mut challenger, None, false, inner_config)
    );

    verify_stark_proof_with_challenges_circuit::<F, C, S, D>(
        builder,
        &stark,
        &proof_with_pis.proof,
        &proof_with_pis.public_inputs,
        challenges,
        None,
        inner_config,
        degree_bits,
    );
}

/// Recursively verifies an inner STARK proof.
pub fn verify_stark_proof_with_challenges_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    proof: &StarkProofTarget<D>,
    public_inputs: &[Target],
    challenges: StarkProofChallengesTarget<D>,
    ctl_vars: Option<&[CtlCheckVarsTarget<F, D>]>,
    inner_config: &StarkConfig,
    degree_bits: usize,
) where
    C::Hasher: AlgebraicHasher<F>,
{
    check_lookup_options(stark, proof, &challenges).unwrap();

    let zero = builder.zero();
    let one = builder.one_extension();

    let num_ctl_polys = ctl_vars
        .map(|v| v.iter().map(|ctl| ctl.helper_columns.len()).sum::<usize>())
        .unwrap_or_default();

    let StarkOpeningSetTarget {
        local_values,
        next_values,
        auxiliary_polys,
        auxiliary_polys_next,
        ctl_zs_first,
        quotient_polys,
    } = &proof.openings;

    let vars = S::EvaluationFrameTarget::from_values(
        local_values,
        next_values,
        &public_inputs
            .iter()
            .map(|&t| builder.convert_to_ext(t))
            .collect::<Vec<_>>(),
    );

    let max_num_degree_bits = F::TWO_ADICITY;
    {
        // degree_bits should be nonzero.
        let max_num_degree_bits_bits = log2_ceil(F::TWO_ADICITY);
        let degree_bits_bits = builder.split_le(proof.degree_bits, max_num_degree_bits_bits);
        let mut or_all_bits = builder._false();
        for i in 0..max_num_degree_bits_bits {
            or_all_bits = builder.or(or_all_bits, degree_bits_bits[i]);
        }
        builder.assert_one(or_all_bits.target);
    }

    let two = builder.two();
    let degree = builder.exp(two, proof.degree_bits, max_num_degree_bits);
    let degree_bits_vec = builder.split_le(degree, max_num_degree_bits);

    let zeta_pow_deg = builder.exp_extension_from_bits(challenges.stark_zeta, &degree_bits_vec);
    let z_h_zeta = builder.sub_extension(zeta_pow_deg, one);
    let degree_ext = builder.convert_to_ext(degree);

    // Calculate primitive_root_of_unity(degree_bits)
    let two_adicity = builder.constant(F::from_canonical_usize(F::Extension::TWO_ADICITY));
    let two_adicity_sub_degree_bits = builder.sub(two_adicity, proof.degree_bits);
    let two_exp_two_adicity_sub_degree_bits = builder.exp(
        two,
        two_adicity_sub_degree_bits,
        log2_ceil(F::Extension::TWO_ADICITY),
    );
    let exponent_bits = builder.split_le(
        two_exp_two_adicity_sub_degree_bits,
        F::Extension::TWO_ADICITY,
    );
    let base = builder.constant_extension(F::Extension::POWER_OF_TWO_GENERATOR);
    let g = builder.exp_extension_from_bits(base, &exponent_bits);

    let (l_0, l_last) =
        eval_l_0_and_l_last_circuit(builder, degree_ext, g, challenges.stark_zeta, z_h_zeta);
    let last = builder.inverse_extension(g);
    let z_last = builder.sub_extension(challenges.stark_zeta, last);

    let mut consumer = RecursiveConstraintConsumer::<F, D>::new(
        builder.zero_extension(),
        challenges.stark_alphas,
        z_last,
        l_0,
        l_last,
    );

    let num_lookup_columns = stark.num_lookup_helper_columns(inner_config);
    let lookup_challenges = stark.uses_lookups().then(|| {
        challenges
            .lookup_challenge_set
            .as_ref()
            .unwrap()
            .challenges
            .iter()
            .map(|ch| ch.beta)
            .collect::<Vec<_>>()
    });

    let lookup_vars = stark.uses_lookups().then(|| LookupCheckVarsTarget {
        local_values: auxiliary_polys.as_ref().unwrap()[..num_lookup_columns].to_vec(),
        next_values: auxiliary_polys_next.as_ref().unwrap()[..num_lookup_columns].to_vec(),
        challenges: lookup_challenges.unwrap(),
    });

    with_context!(
        builder,
        "evaluate vanishing polynomial",
        eval_vanishing_poly_circuit::<F, S, D>(
            builder,
            stark,
            &vars,
            lookup_vars,
            ctl_vars,
            &mut consumer
        )
    );
    let vanishing_polys_zeta = consumer.accumulators();

    // Check each polynomial identity, of the form `vanishing(x) = Z_H(x) quotient(x)`, at zeta.
    let mut scale = ReducingFactorTarget::new(zeta_pow_deg);
    if let Some(quotient_polys) = quotient_polys {
        for (i, chunk) in quotient_polys
            .chunks(stark.quotient_degree_factor())
            .enumerate()
        {
            let recombined_quotient = scale.reduce(chunk, builder);
            let computed_vanishing_poly = builder.mul_extension(z_h_zeta, recombined_quotient);
            builder.connect_extension(vanishing_polys_zeta[i], computed_vanishing_poly);
        }
    }

    let merkle_caps = once(proof.trace_cap.clone())
        .chain(proof.auxiliary_polys_cap.clone())
        .chain(proof.quotient_polys_cap.clone())
        .collect_vec();

    // Calculate primitive_root_of_unity(degree_bits)
    let two_adicity = builder.constant(F::from_canonical_usize(F::TWO_ADICITY));
    let two_adicity_sub_degree_bits = builder.sub(two_adicity, proof.degree_bits);
    let two_exp_two_adicity_sub_degree_bits =
        builder.exp(two, two_adicity_sub_degree_bits, F::TWO_ADICITY);
    let base = builder.constant(F::POWER_OF_TWO_GENERATOR);
    let g = builder.exp(base, two_exp_two_adicity_sub_degree_bits, F::TWO_ADICITY);

    let fri_instance = stark.fri_instance_target(
        builder,
        challenges.stark_zeta,
        g,
        num_ctl_polys,
        ctl_zs_first.as_ref().map_or(0, |c| c.len()),
        inner_config,
    );
    builder.verify_fri_proof::<C>(
        &fri_instance,
        &proof.openings.to_fri_openings(zero),
        &challenges.fri_challenges,
        &merkle_caps,
        &proof.opening_proof,
        &inner_config.fri_params(degree_bits),
    );
}

fn eval_l_0_and_l_last_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    n: ExtensionTarget<D>,
    g: ExtensionTarget<D>,
    x: ExtensionTarget<D>,
    z_x: ExtensionTarget<D>,
) -> (ExtensionTarget<D>, ExtensionTarget<D>) {
    let one = builder.one_extension();
    let l_0_deno = builder.mul_sub_extension(n, x, n);
    let l_last_deno = builder.mul_sub_extension(g, x, one);
    let l_last_deno = builder.mul_extension(n, l_last_deno);

    (
        builder.div_extension(z_x, l_0_deno),
        builder.div_extension(z_x, l_last_deno),
    )
}

/// Adds a new `StarkProofWithPublicInputsTarget` to this circuit.
pub fn add_virtual_stark_proof_with_pis<
    F: RichField + Extendable<D>,
    S: Stark<F, D>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    config: &StarkConfig,
    degree_bits: usize,
    num_ctl_helper_zs: usize,
    num_ctl_zs: usize,
) -> StarkProofWithPublicInputsTarget<D> {
    let proof = add_virtual_stark_proof::<F, S, D>(
        builder,
        stark,
        config,
        degree_bits,
        num_ctl_helper_zs,
        num_ctl_zs,
    );
    let public_inputs = builder.add_virtual_targets(S::PUBLIC_INPUTS);
    StarkProofWithPublicInputsTarget {
        proof,
        public_inputs,
    }
}

/// Adds a new `StarkProofTarget` to this circuit.
pub fn add_virtual_stark_proof<F: RichField + Extendable<D>, S: Stark<F, D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    config: &StarkConfig,
    degree_bits: usize,
    num_ctl_helper_zs: usize,
    num_ctl_zs: usize,
) -> StarkProofTarget<D> {
    let fri_params = config.fri_params(degree_bits);
    let cap_height = fri_params.config.cap_height;

    let num_leaves_per_oracle = once(S::COLUMNS)
        .chain(
            (stark.uses_lookups() || stark.requires_ctls())
                .then(|| stark.num_lookup_helper_columns(config) + num_ctl_helper_zs),
        )
        .chain(
            (stark.quotient_degree_factor() > 0)
                .then(|| stark.quotient_degree_factor() * config.num_challenges),
        )
        .collect_vec();

    let auxiliary_polys_cap = (stark.uses_lookups() || stark.requires_ctls())
        .then(|| builder.add_virtual_cap(cap_height));

    let quotient_polys_cap =
        (stark.constraint_degree() > 0).then(|| builder.add_virtual_cap(cap_height));

    StarkProofTarget {
        trace_cap: builder.add_virtual_cap(cap_height),
        auxiliary_polys_cap,
        quotient_polys_cap,
        openings: add_virtual_stark_opening_set::<F, S, D>(
            builder,
            stark,
            num_ctl_helper_zs,
            num_ctl_zs,
            config,
        ),
        opening_proof: builder.add_virtual_fri_proof(&num_leaves_per_oracle, &fri_params),
        degree_bits: builder.add_virtual_target(),
    }
}

fn add_virtual_stark_opening_set<F: RichField + Extendable<D>, S: Stark<F, D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    stark: &S,
    num_ctl_helper_zs: usize,
    num_ctl_zs: usize,
    config: &StarkConfig,
) -> StarkOpeningSetTarget<D> {
    StarkOpeningSetTarget {
        local_values: builder.add_virtual_extension_targets(S::COLUMNS),
        next_values: builder.add_virtual_extension_targets(S::COLUMNS),
        auxiliary_polys: (stark.uses_lookups() || stark.requires_ctls()).then(|| {
            builder.add_virtual_extension_targets(
                stark.num_lookup_helper_columns(config) + num_ctl_helper_zs,
            )
        }),
        auxiliary_polys_next: (stark.uses_lookups() || stark.requires_ctls()).then(|| {
            builder.add_virtual_extension_targets(
                stark.num_lookup_helper_columns(config) + num_ctl_helper_zs,
            )
        }),
        ctl_zs_first: stark
            .requires_ctls()
            .then(|| builder.add_virtual_targets(num_ctl_zs)),
        quotient_polys: (stark.constraint_degree() > 0).then(|| {
            builder.add_virtual_extension_targets(
                stark.quotient_degree_factor() * config.num_challenges,
            )
        }),
    }
}

/// Set the targets in a `StarkProofWithPublicInputsTarget` to
/// their corresponding values in a `StarkProofWithPublicInputs`.
pub fn set_stark_proof_with_pis_target<F, C: GenericConfig<D, F = F>, W, const D: usize>(
    witness: &mut W,
    stark_proof_with_pis_target: &StarkProofWithPublicInputsTarget<D>,
    stark_proof_with_pis: &StarkProofWithPublicInputs<F, C, D>,
    degree_bits: usize,
    zero: Target,
) -> Result<()>
where
    F: RichField + Extendable<D>,
    C::Hasher: AlgebraicHasher<F>,
    W: WitnessWrite<F>,
{
    let StarkProofWithPublicInputs {
        proof,
        public_inputs,
    } = stark_proof_with_pis;
    let StarkProofWithPublicInputsTarget {
        proof: pt,
        public_inputs: pi_targets,
    } = stark_proof_with_pis_target;

    // Set public inputs.
    for (&pi_t, &pi) in pi_targets.iter().zip_eq(public_inputs) {
        witness.set_target(pi_t, pi)?;
    }

    set_stark_proof_target(witness, pt, proof, degree_bits, zero)
}

/// Set the targets in a [`StarkProofTarget`] to their corresponding values in a
/// [`StarkProof`].
pub fn set_stark_proof_target<F, C: GenericConfig<D, F = F>, W, const D: usize>(
    witness: &mut W,
    proof_target: &StarkProofTarget<D>,
    proof: &StarkProof<F, C, D>,
    degree_bits: usize,
    zero: Target,
) -> Result<()>
where
    F: RichField + Extendable<D>,
    C::Hasher: AlgebraicHasher<F>,
    W: WitnessWrite<F>,
{
    witness.set_target(
        proof_target.degree_bits,
        F::from_canonical_usize(degree_bits),
    )?;
    witness.set_cap_target(&proof_target.trace_cap, &proof.trace_cap)?;
    if let (Some(quotient_polys_cap_target), Some(quotient_polys_cap)) =
        (&proof_target.quotient_polys_cap, &proof.quotient_polys_cap)
    {
        witness.set_cap_target(quotient_polys_cap_target, quotient_polys_cap)?;
    }

    witness.set_fri_openings(
        &proof_target.openings.to_fri_openings(zero),
        &proof.openings.to_fri_openings(),
    )?;

    if let (Some(auxiliary_polys_cap_target), Some(auxiliary_polys_cap)) = (
        &proof_target.auxiliary_polys_cap,
        &proof.auxiliary_polys_cap,
    ) {
        witness.set_cap_target(auxiliary_polys_cap_target, auxiliary_polys_cap)?;
    }

    set_fri_proof_target(witness, &proof_target.opening_proof, &proof.opening_proof)
}

/// Utility function to check that all lookups data wrapped in `Option`s are `Some` iff
/// the STARK uses a permutation argument.
fn check_lookup_options<F: RichField + Extendable<D>, S: Stark<F, D>, const D: usize>(
    stark: &S,
    proof: &StarkProofTarget<D>,
    challenges: &StarkProofChallengesTarget<D>,
) -> Result<()> {
    let options_is_some = [
        proof.auxiliary_polys_cap.is_some(),
        proof.openings.auxiliary_polys.is_some(),
        proof.openings.auxiliary_polys_next.is_some(),
        challenges.lookup_challenge_set.is_some(),
    ];
    ensure!(
        options_is_some
            .iter()
            .all(|&b| b == stark.uses_lookups() || stark.requires_ctls()),
        "Lookups data doesn't match with STARK configuration."
    );
    Ok(())
}
