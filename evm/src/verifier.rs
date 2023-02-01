use std::any::type_name;

use anyhow::{ensure, Result};
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::types::Field;
use plonky2::fri::verifier::verify_fri_proof;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::config::{GenericConfig, Hasher};
use plonky2::plonk::plonk_common::reduce_with_powers;

use crate::all_stark::{AllStark, Table};
use crate::config::StarkConfig;
use crate::constraint_consumer::ConstraintConsumer;
use crate::cpu::cpu_stark::CpuStark;
use crate::cross_table_lookup::{verify_cross_table_lookups, CtlCheckVars};
use crate::keccak::keccak_stark::KeccakStark;
use crate::keccak_sponge::keccak_sponge_stark::KeccakSpongeStark;
use crate::logic::LogicStark;
use crate::memory::memory_stark::MemoryStark;
use crate::permutation::PermutationCheckVars;
use crate::proof::{
    AllProof, AllProofChallenges, StarkOpeningSet, StarkProof, StarkProofChallenges,
};
use crate::stark::Stark;
use crate::vanishing_poly::eval_vanishing_poly;
use crate::vars::StarkEvaluationVars;

pub fn verify_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    all_stark: &AllStark<F, D>,
    all_proof: AllProof<F, C, D>,
    config: &StarkConfig,
) -> Result<()>
where
    [(); CpuStark::<F, D>::COLUMNS]:,
    [(); KeccakStark::<F, D>::COLUMNS]:,
    [(); KeccakSpongeStark::<F, D>::COLUMNS]:,
    [(); LogicStark::<F, D>::COLUMNS]:,
    [(); MemoryStark::<F, D>::COLUMNS]:,
    [(); C::Hasher::HASH_SIZE]:,
{
    let AllProofChallenges {
        stark_challenges,
        ctl_challenges,
    } = all_proof.get_challenges(all_stark, config);

    let nums_permutation_zs = all_stark.nums_permutation_zs(config);

    let AllStark {
        cpu_stark,
        keccak_stark,
        keccak_sponge_stark,
        logic_stark,
        memory_stark,
        cross_table_lookups,
    } = all_stark;

    let ctl_vars_per_table = CtlCheckVars::from_proofs(
        &all_proof.stark_proofs,
        cross_table_lookups,
        &ctl_challenges,
        &nums_permutation_zs,
    );

    verify_stark_proof_with_challenges(
        cpu_stark,
        &all_proof.stark_proofs[Table::Cpu as usize].proof,
        &stark_challenges[Table::Cpu as usize],
        &ctl_vars_per_table[Table::Cpu as usize],
        config,
    )?;
    verify_stark_proof_with_challenges(
        keccak_stark,
        &all_proof.stark_proofs[Table::Keccak as usize].proof,
        &stark_challenges[Table::Keccak as usize],
        &ctl_vars_per_table[Table::Keccak as usize],
        config,
    )?;
    verify_stark_proof_with_challenges(
        keccak_sponge_stark,
        &all_proof.stark_proofs[Table::KeccakSponge as usize].proof,
        &stark_challenges[Table::KeccakSponge as usize],
        &ctl_vars_per_table[Table::KeccakSponge as usize],
        config,
    )?;
    verify_stark_proof_with_challenges(
        memory_stark,
        &all_proof.stark_proofs[Table::Memory as usize].proof,
        &stark_challenges[Table::Memory as usize],
        &ctl_vars_per_table[Table::Memory as usize],
        config,
    )?;
    verify_stark_proof_with_challenges(
        logic_stark,
        &all_proof.stark_proofs[Table::Logic as usize].proof,
        &stark_challenges[Table::Logic as usize],
        &ctl_vars_per_table[Table::Logic as usize],
        config,
    )?;

    verify_cross_table_lookups::<F, C, D>(
        cross_table_lookups,
        all_proof.stark_proofs.map(|p| p.proof.openings.ctl_zs_last),
        config,
    )
}

pub(crate) fn verify_stark_proof_with_challenges<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
>(
    stark: &S,
    proof: &StarkProof<F, C, D>,
    challenges: &StarkProofChallenges<F, D>,
    ctl_vars: &[CtlCheckVars<F, F::Extension, F::Extension, D>],
    config: &StarkConfig,
) -> Result<()>
where
    [(); S::COLUMNS]:,
    [(); C::Hasher::HASH_SIZE]:,
{
    log::debug!("Checking proof: {}", type_name::<S>());
    validate_proof_shape(stark, proof, config, ctl_vars.len())?;
    let StarkOpeningSet {
        local_values,
        next_values,
        permutation_ctl_zs,
        permutation_ctl_zs_next,
        ctl_zs_last,
        quotient_polys,
    } = &proof.openings;
    let vars = StarkEvaluationVars {
        local_values: &local_values.to_vec().try_into().unwrap(),
        next_values: &next_values.to_vec().try_into().unwrap(),
    };

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
    let num_permutation_zs = stark.num_permutation_batches(config);
    let permutation_data = stark.uses_permutation_args().then(|| PermutationCheckVars {
        local_zs: permutation_ctl_zs[..num_permutation_zs].to_vec(),
        next_zs: permutation_ctl_zs_next[..num_permutation_zs].to_vec(),
        permutation_challenge_sets: challenges.permutation_challenge_sets.clone().unwrap(),
    });
    eval_vanishing_poly::<F, F::Extension, F::Extension, C, S, D, D>(
        stark,
        config,
        vars,
        permutation_data,
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
        .chunks(stark.quotient_degree_factor())
        .enumerate()
    {
        ensure!(
            vanishing_polys_zeta[i] == z_h_zeta * reduce_with_powers(chunk, zeta_pow_deg),
            "Mismatch between evaluation and opening of quotient polynomial"
        );
    }

    let merkle_caps = vec![
        proof.trace_cap.clone(),
        proof.permutation_ctl_zs_cap.clone(),
        proof.quotient_polys_cap.clone(),
    ];

    verify_fri_proof::<F, C, D>(
        &stark.fri_instance(
            challenges.stark_zeta,
            F::primitive_root_of_unity(degree_bits),
            degree_bits,
            ctl_zs_last.len(),
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
    config: &StarkConfig,
    num_ctl_zs: usize,
) -> anyhow::Result<()>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    [(); S::COLUMNS]:,
    [(); C::Hasher::HASH_SIZE]:,
{
    let StarkProof {
        trace_cap,
        permutation_ctl_zs_cap,
        quotient_polys_cap,
        openings,
        // The shape of the opening proof will be checked in the FRI verifier (see
        // validate_fri_proof_shape), so we ignore it here.
        opening_proof: _,
    } = proof;

    let StarkOpeningSet {
        local_values,
        next_values,
        permutation_ctl_zs,
        permutation_ctl_zs_next,
        ctl_zs_last,
        quotient_polys,
    } = openings;

    let degree_bits = proof.recover_degree_bits(config);
    let fri_params = config.fri_params(degree_bits);
    let cap_height = fri_params.config.cap_height;
    let num_zs = num_ctl_zs + stark.num_permutation_batches(config);

    ensure!(trace_cap.height() == cap_height);
    ensure!(permutation_ctl_zs_cap.height() == cap_height);
    ensure!(quotient_polys_cap.height() == cap_height);

    ensure!(local_values.len() == S::COLUMNS);
    ensure!(next_values.len() == S::COLUMNS);
    ensure!(permutation_ctl_zs.len() == num_zs);
    ensure!(permutation_ctl_zs_next.len() == num_zs);
    ensure!(ctl_zs_last.len() == num_ctl_zs);
    ensure!(quotient_polys.len() == stark.num_quotient_polys(config));

    Ok(())
}

/// Evaluate the Lagrange polynomials `L_0` and `L_(n-1)` at a point `x`.
/// `L_0(x) = (x^n - 1)/(n * (x - 1))`
/// `L_(n-1)(x) = (x^n - 1)/(n * (g * x - 1))`, with `g` the first element of the subgroup.
fn eval_l_0_and_l_last<F: Field>(log_n: usize, x: F) -> (F, F) {
    let n = F::from_canonical_usize(1 << log_n);
    let g = F::primitive_root_of_unity(log_n);
    let z_x = x.exp_power_of_2(log_n) - F::ONE;
    let invs = F::batch_multiplicative_inverse(&[n * (x - F::ONE), n * (g * x - F::ONE)]);

    (z_x * invs[0], z_x * invs[1])
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
