use crate::field::extension_field::Extendable;
use crate::field::field_types::{PrimeField, RichField};
use crate::fri::verifier::fri_verify_proof_of_work;
use crate::iop::challenger::Challenger;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::proof::{ProofChallenges, ProofWithPublicInputs};

impl<F: RichField + Extendable<D>, const D: usize> ProofWithPublicInputs<F, D> {
    pub(crate) fn get_challenges(
        &self,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<ProofChallenges<F, D>> {
        let config = &common_data.config;
        let num_challenges = config.num_challenges;
        let num_fri_queries = config.fri_config.num_query_rounds;
        let lde_size = common_data.lde_size();

        let mut challenger = Challenger::new();

        // Observe the instance.
        challenger.observe_hash(&common_data.circuit_digest);
        challenger.observe_hash(&self.get_public_inputs_hash());

        challenger.observe_cap(&self.proof.wires_cap);
        let plonk_betas = challenger.get_n_challenges(num_challenges);
        let plonk_gammas = challenger.get_n_challenges(num_challenges);

        challenger.observe_cap(&self.proof.plonk_zs_partial_products_cap);
        let plonk_alphas = challenger.get_n_challenges(num_challenges);

        challenger.observe_cap(&self.proof.quotient_polys_cap);
        let plonk_zeta = challenger.get_extension_challenge();

        challenger.observe_opening_set(&self.proof.openings);

        // Scaling factor to combine polynomials.
        let fri_alpha = challenger.get_extension_challenge();

        // Recover the random betas used in the FRI reductions.
        let fri_betas = self
            .proof
            .opening_proof
            .commit_phase_merkle_caps
            .iter()
            .map(|cap| {
                challenger.observe_cap(cap);
                challenger.get_extension_challenge()
            })
            .collect();

        challenger.observe_extension_elements(&self.proof.opening_proof.final_poly.coeffs);

        // Check PoW.
        fri_verify_proof_of_work(
            &self.proof.opening_proof,
            &mut challenger,
            &config.fri_config,
        )?;

        let fri_query_indices = (0..num_fri_queries)
            .map(|_| challenger.get_challenge().to_canonical_u64() as usize % lde_size)
            .collect();

        Ok(ProofChallenges {
            plonk_betas,
            plonk_gammas,
            plonk_alphas,
            plonk_zeta,
            fri_alpha,
            fri_betas,
            fri_query_indices,
        })
    }
}
