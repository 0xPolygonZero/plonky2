//! plonky2 verifier implementation.

use anyhow::{ensure, Result};
use itertools::Itertools;

use crate::boil::boil_prover::{AccInfo, AccProof};
use crate::boil::boil_verifier::verify_acc_proof;
use crate::boil::QN;
use crate::field::extension::Extendable;
use crate::field::types::Field;
use crate::fri::proof::FriChallenges;
use crate::hash::hash_types::RichField;
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::challenger::Challenger;
use crate::plonk::circuit_data::{CommonCircuitData, VerifierOnlyCircuitData};
use crate::plonk::config::{GenericConfig, Hasher};
use crate::plonk::plonk_common::reduce_with_powers;
use crate::plonk::proof::OpeningSet;
use crate::plonk::vanishing_poly::eval_vanishing_poly;
use crate::plonk::vars::EvaluationVars;

use super::circuit_builder::NUM_COINS_LOOKUP;
use super::proof::{IVCProof, IVCProofWithPublicInputs, ProofChallenges};

pub static mut IVCDEBUG_COUNTHASH: bool = false;

pub(crate) fn validate_ivc_proof_with_pis_shape<F, C, const D: usize>(
    proof_with_pis: &IVCProofWithPublicInputs<F, C, D>,
    common_data: &CommonCircuitData<F, D>,
    accs_info: &[&AccInfo<F, C::Hasher, D>],
) -> anyhow::Result<()>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let IVCProofWithPublicInputs {
        proof,
        public_inputs,
    } = proof_with_pis;
    validate_ivc_proof_shape(proof, common_data, accs_info)?;
    ensure!(
        public_inputs.len() == common_data.num_public_inputs,
        "Number of public inputs doesn't match circuit data."
    );
    Ok(())
}

fn validate_ivc_proof_shape<F, C, const D: usize>(
    proof: &IVCProof<F, C, D>,
    common_data: &CommonCircuitData<F, D>,
    accs_info: &[&AccInfo<F, C::Hasher, D>],
) -> anyhow::Result<()>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let config = &common_data.config;
    let IVCProof {
        wires_cap,
        plonk_zs_partial_products_cap,
        quotient_polys_cap,
        openings,
        // The shape of the opening proof will be checked in the FRI verifier (see
        // validate_fri_proof_shape), so we ignore it here.
        acc_proof: _,
    } = proof;
    let OpeningSet {
        constants,
        plonk_sigmas,
        wires,
        plonk_zs,
        plonk_zs_next,
        partial_products,
        quotient_polys,
        lookup_zs,
        lookup_zs_next,
    } = openings;
    let cap_height = common_data.fri_params.config.cap_height;
    ensure!(wires_cap.height() == cap_height);
    ensure!(plonk_zs_partial_products_cap.height() == cap_height);
    ensure!(quotient_polys_cap.height() == cap_height);
    ensure!(constants.len() == common_data.num_constants);
    ensure!(plonk_sigmas.len() == config.num_routed_wires);
    ensure!(wires.len() == config.num_wires);
    ensure!(plonk_zs.len() == config.num_challenges);
    ensure!(plonk_zs_next.len() == config.num_challenges);
    ensure!(partial_products.len() == config.num_challenges * common_data.num_partial_products);
    ensure!(quotient_polys.len() == common_data.num_quotient_polys());
    ensure!(lookup_zs.len() == common_data.num_all_lookup_polys());
    ensure!(lookup_zs_next.len() == common_data.num_all_lookup_polys());
    Ok(())
}

fn get_ivc_challenges<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    public_inputs_hash: <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash,
    wires_cap: &MerkleCap<F, C::Hasher>,
    plonk_zs_partial_products_cap: &MerkleCap<F, C::Hasher>,
    quotient_polys_cap: &MerkleCap<F, C::Hasher>,
    openings: &OpeningSet<F, D>,
    new_acc_cap: &MerkleCap<F, C::Hasher>,
    circuit_digest: &<<C as GenericConfig<D>>::Hasher as Hasher<C::F>>::Hash,
    common_data: &CommonCircuitData<F, D>,
) -> anyhow::Result<ProofChallenges<F, D>> {
    let config = &common_data.config;
    let num_challenges = config.num_challenges;

    let mut challenger = Challenger::<F, C::Hasher>::new();
    let has_lookup = common_data.num_lookup_polys != 0;

    // Observe the instance.
    challenger.observe_hash::<C::Hasher>(*circuit_digest);
    challenger.observe_hash::<C::InnerHasher>(public_inputs_hash);

    challenger.observe_cap::<C::Hasher>(wires_cap);
    let plonk_betas = challenger.get_n_challenges(num_challenges);
    let plonk_gammas = challenger.get_n_challenges(num_challenges);

    // If there are lookups in the circuit, we should get delta challenges as well.
    // But we can use the already generated `plonk_betas` and `plonk_gammas` as the first `plonk_deltas` challenges.
    let plonk_deltas = if has_lookup {
        let num_lookup_challenges = NUM_COINS_LOOKUP * num_challenges;
        let mut deltas = Vec::with_capacity(num_lookup_challenges);
        let num_additional_challenges = num_lookup_challenges - 2 * num_challenges;
        let additional = challenger.get_n_challenges(num_additional_challenges);
        deltas.extend(&plonk_betas);
        deltas.extend(&plonk_gammas);
        deltas.extend(additional);
        deltas
    } else {
        vec![]
    };

    // `plonk_zs_partial_products_cap` also contains the commitment to lookup polynomials.
    challenger.observe_cap::<C::Hasher>(plonk_zs_partial_products_cap);
    let plonk_alphas = challenger.get_n_challenges(num_challenges);

    challenger.observe_cap::<C::Hasher>(quotient_polys_cap);
    let plonk_zeta = challenger.get_extension_challenge::<D>();

    challenger.observe_openings(&openings.to_fri_openings());

    let ob_alpha = challenger.get_extension_challenge::<D>();
    challenger.observe_cap::<C::Hasher>(new_acc_cap);
    let ood = challenger.get_extension_challenge::<D>();
    let ind_points  = challenger.get_n_challenges(QN).into_iter().collect_vec();
    let indic = ind_points
        .iter()
        .map(|rand| {
            let x_index = rand.to_canonical_u64() as usize % common_data.fri_params.lde_size();
            x_index
        })
        .collect::<Vec<usize>>();
    let ob_challenges: FriChallenges<F, D> = FriChallenges {
        fri_alpha: ob_alpha,
        fri_betas: Vec::new(),
        fri_pow_response: F::ZERO,
        fri_query_indices: indic, 
    };

    Ok(ProofChallenges {
        plonk_betas,
        plonk_gammas,
        plonk_alphas,
        plonk_deltas,
        plonk_zeta,
        fri_challenges: ob_challenges,
    })
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    IVCProofWithPublicInputs<F, C, D>
{
    /// Computes all Fiat-Shamir challenges used in the Plonk proof.
    pub fn get_challenges(
        &self,
        public_inputs_hash: <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash,
        circuit_digest: &<<C as GenericConfig<D>>::Hasher as Hasher<C::F>>::Hash,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<ProofChallenges<F, D>> {
        let IVCProof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            acc_proof:
                AccProof {
                    merkle_cap,
                    ..
                },
        } = &self.proof;

        get_ivc_challenges::<F, C, D>(
            public_inputs_hash,
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            merkle_cap,
            circuit_digest,
            common_data,
        )
    }

    pub fn get_public_inputs_hash(
        &self,
    ) -> <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash {
        C::InnerHasher::hash_no_pad(&self.public_inputs)
    }

}


pub fn ivc_verify<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    proof_with_pis: IVCProofWithPublicInputs<F, C, D>,
    verifier_data: &VerifierOnlyCircuitData<C, D>,
    common_data: &CommonCircuitData<F, D>,
    accs_info: &[&AccInfo<F, C::Hasher, D>],
) -> Result<()> {

    validate_ivc_proof_with_pis_shape(&proof_with_pis, common_data, accs_info)?;
    
    println!("! plonk:: ivc_verify()");
    if unsafe { IVCDEBUG_COUNTHASH } {
        unsafe { crate::hash::merkle_proofs::COUNTHASHES_MERKLE = true };
        unsafe { crate::hash::poseidon::COUNTHASHES_POSEIDON = true };
        unsafe { crate::fri::verifier::COUNTHASHES_FRIVER = true };
        println!("! plonk:: ivc_verify():: hashcount started");
    }

    let public_inputs_hash = proof_with_pis.get_public_inputs_hash();
    let challenges = proof_with_pis.get_challenges(
        public_inputs_hash,
        &verifier_data.circuit_digest,
        common_data,
    )?;

    ivc_verify_with_challenges::<F, C, D>(
        proof_with_pis.proof,
        public_inputs_hash,
        challenges,
        verifier_data,
        common_data,
        accs_info
    )
}

pub(crate) fn ivc_verify_with_challenges<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    proof: IVCProof<F, C, D>,
    public_inputs_hash: <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash,
    challenges: ProofChallenges<F, D>,
    verifier_data: &VerifierOnlyCircuitData<C, D>,
    common_data: &CommonCircuitData<F, D>,
    accs_info: &[&AccInfo<F, C::Hasher, D>],
) -> Result<()> {
    let local_constants = &proof.openings.constants;
    let local_wires = &proof.openings.wires;
    let vars = EvaluationVars {
        local_constants,
        local_wires,
        public_inputs_hash: &public_inputs_hash,
    };
    let local_zs = &proof.openings.plonk_zs;
    let next_zs = &proof.openings.plonk_zs_next;
    let local_lookup_zs = &proof.openings.lookup_zs;
    let next_lookup_zs = &proof.openings.lookup_zs_next;
    let s_sigmas = &proof.openings.plonk_sigmas;
    let partial_products = &proof.openings.partial_products;

    // Evaluate the vanishing polynomial at our challenge point, zeta.
    let vanishing_polys_zeta = eval_vanishing_poly::<F, D>(
        common_data,
        challenges.plonk_zeta,
        vars,
        local_zs,
        next_zs,
        local_lookup_zs,
        next_lookup_zs,
        partial_products,
        s_sigmas,
        &challenges.plonk_betas,
        &challenges.plonk_gammas,
        &challenges.plonk_alphas,
        &challenges.plonk_deltas,
    );

    // Check each polynomial identity, of the form `vanishing(x) = Z_H(x) quotient(x)`, at zeta.
    let quotient_polys_zeta = &proof.openings.quotient_polys;
    let zeta_pow_deg = challenges
        .plonk_zeta
        .exp_power_of_2(common_data.degree_bits());
    let z_h_zeta = zeta_pow_deg - F::Extension::ONE;
    // `quotient_polys_zeta` holds `num_challenges * quotient_degree_factor` evaluations.
    // Each chunk of `quotient_degree_factor` holds the evaluations of `t_0(zeta),...,t_{quotient_degree_factor-1}(zeta)`
    // where the "real" quotient polynomial is `t(X) = t_0(X) + t_1(X)*X^n + t_2(X)*X^{2n} + ...`.
    // So to reconstruct `t(zeta)` we can compute `reduce_with_powers(chunk, zeta^n)` for each
    // `quotient_degree_factor`-sized chunk of the original evaluations.
    for (i, chunk) in quotient_polys_zeta
        .chunks(common_data.quotient_degree_factor)
        .enumerate()
    {
        ensure!(vanishing_polys_zeta[i] == z_h_zeta * reduce_with_powers(chunk, zeta_pow_deg));
    }

    let merkle_caps = &[
        verifier_data.constants_sigmas_cap.clone(),
        proof.wires_cap,
        // In the lookup case, `plonk_zs_partial_products_cap` should also include the lookup commitment.
        proof.plonk_zs_partial_products_cap,
        proof.quotient_polys_cap,
    ];

    verify_acc_proof::<F, C, D>(
        &proof.acc_proof,
        &challenges.fri_challenges,
        accs_info,
        &common_data.get_fri_instance(challenges.plonk_zeta),
        &proof.openings.to_fri_openings(),
        merkle_caps,
        &common_data.fri_params,
    )?;


    if unsafe { IVCDEBUG_COUNTHASH } {
        unsafe { crate::hash::merkle_proofs::COUNTHASHES_MERKLE = false };
        unsafe { crate::hash::poseidon::COUNTHASHES_POSEIDON = false };
        unsafe { crate::fri::verifier::COUNTHASHES_FRIVER = false };
        println!("! plonk:: ivc_verify():: hashcount started");
    }


    Ok(())
}
