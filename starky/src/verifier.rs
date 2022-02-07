use anyhow::{ensure, Result};
use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::field_types::Field;
use plonky2::fri::verifier::verify_fri_proof;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::plonk_common::reduce_with_powers;
use plonky2_util::log2_strict;

use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::proof::{StarkOpeningSet, StarkProof, StarkProofChallenges, StarkProofWithPublicInputs};
use crate::stark::Stark;
use crate::vars::StarkEvaluationVars;

pub fn verify<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    stark: S,
    proof_with_pis: StarkProofWithPublicInputs<F, C, D>,
    config: &StarkConfig,
) -> Result<()>
where
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    let degree_bits = log2_strict(recover_degree(&proof_with_pis.proof, config));
    let challenges = proof_with_pis.get_challenges(config, degree_bits)?;
    verify_with_challenges(stark, proof_with_pis, challenges, degree_bits, config)
}

pub(crate) fn verify_with_challenges<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    stark: S,
    proof_with_pis: StarkProofWithPublicInputs<F, C, D>,
    challenges: StarkProofChallenges<F, D>,
    degree_bits: usize,
    config: &StarkConfig,
) -> Result<()>
where
    [(); S::COLUMNS]:,
    [(); S::PUBLIC_INPUTS]:,
{
    let StarkProofWithPublicInputs {
        proof,
        public_inputs,
    } = proof_with_pis;
    let local_values = &proof.openings.local_values;
    let next_values = &proof.openings.local_values;
    let StarkOpeningSet {
        local_values,
        next_values,
        permutation_zs,
        permutation_zs_right,
        quotient_polys,
    } = &proof.openings;
    let vars = StarkEvaluationVars {
        local_values: &local_values.to_vec().try_into().unwrap(),
        next_values: &next_values.to_vec().try_into().unwrap(),
        public_inputs: &public_inputs
            .into_iter()
            .map(F::Extension::from_basefield)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    };

    let (l_1, l_last) = eval_l_1_and_l_last(degree_bits, challenges.stark_zeta);
    let last = F::primitive_root_of_unity(degree_bits).inverse();
    let z_last = challenges.stark_zeta - last.into();
    let mut consumer = ConstraintConsumer::<F::Extension>::new(
        challenges
            .stark_alphas
            .iter()
            .map(|&alpha| F::Extension::from_basefield(alpha))
            .collect::<Vec<_>>(),
        z_last,
        l_1,
        l_last,
    );
    stark.eval_ext(vars, &mut consumer);
    let vanishing_polys_zeta = consumer.accumulators();

    // Check each polynomial identity, of the form `vanishing(x) = Z_H(x) quotient(x)`, at zeta.
    let quotient_polys_zeta = &proof.openings.quotient_polys;
    let zeta_pow_deg = challenges.stark_zeta.exp_power_of_2(degree_bits);
    let z_h_zeta = zeta_pow_deg - F::Extension::ONE;
    // `quotient_polys_zeta` holds `num_challenges * quotient_degree_factor` evaluations.
    // Each chunk of `quotient_degree_factor` holds the evaluations of `t_0(zeta),...,t_{quotient_degree_factor-1}(zeta)`
    // where the "real" quotient polynomial is `t(X) = t_0(X) + t_1(X)*X^n + t_2(X)*X^{2n} + ...`.
    // So to reconstruct `t(zeta)` we can compute `reduce_with_powers(chunk, zeta^n)` for each
    // `quotient_degree_factor`-sized chunk of the original evaluations.
    for (i, chunk) in quotient_polys_zeta
        .chunks(stark.quotient_degree_factor())
        .enumerate()
    {
        ensure!(vanishing_polys_zeta[i] == z_h_zeta * reduce_with_powers(chunk, zeta_pow_deg));
    }

    // TODO: Permutation polynomials.
    let merkle_caps = &[proof.trace_cap, proof.quotient_polys_cap];

    verify_fri_proof::<F, C, D>(
        &stark.fri_instance(
            challenges.stark_zeta,
            F::primitive_root_of_unity(degree_bits).into(),
            config.fri_config.rate_bits,
            config.num_challenges,
        ),
        &proof.openings.to_fri_openings(),
        &challenges.fri_challenges,
        merkle_caps,
        &proof.opening_proof,
        &config.fri_params(degree_bits),
    )?;

    Ok(())
}

/// Evaluate the Lagrange polynomials `L_1` and `L_n` at a point `x`.
/// `L_1(x) = (x^n - 1)/(n * (x - 1))`
/// `L_n(x) = (x^n - 1)/(n * (g * x - 1))`, with `g` the first element of the subgroup.
fn eval_l_1_and_l_last<F: Field>(log_n: usize, x: F) -> (F, F) {
    let n = F::from_canonical_usize(1 << log_n);
    let g = F::primitive_root_of_unity(log_n);
    let z_x = x.exp_power_of_2(log_n) - F::ONE;
    let invs = F::batch_multiplicative_inverse(&[n * (x - F::ONE), n * (g * x - F::ONE)]);

    (z_x * invs[0], z_x * invs[1])
}

/// Recover the length of the trace from a STARK proof and a STARK config.
fn recover_degree<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    proof: &StarkProof<F, C, D>,
    config: &StarkConfig,
) -> usize {
    let initial_merkle_proof = &proof.opening_proof.query_round_proofs[0]
        .initial_trees_proof
        .evals_proofs[0]
        .1;
    let lde_bits = config.fri_config.cap_height + initial_merkle_proof.siblings.len();
    1 << (lde_bits - config.fri_config.rate_bits)
}

#[cfg(test)]
mod tests {
    use plonky2::field::field_types::Field;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::polynomial::PolynomialValues;

    use crate::verifier::eval_l_1_and_l_last;

    #[test]
    fn test_eval_l_1_and_l_last() {
        type F = GoldilocksField;
        let log_n = 5;
        let n = 1 << log_n;

        let x = F::rand(); // challenge point
        let expected_l_first_x = PolynomialValues::selector(n, 0).ifft().eval(x);
        let expected_l_last_x = PolynomialValues::selector(n, n - 1).ifft().eval(x);

        let (l_first_x, l_last_x) = eval_l_1_and_l_last(log_n, x);
        assert_eq!(l_first_x, expected_l_first_x);
        assert_eq!(l_last_x, expected_l_last_x);
    }
}
