use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::fri::proof::FriProof;
use crate::hash::hashing::hash_n_to_1;
use crate::iop::challenger::Challenger;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::proof::{ProofChallenges, ProofWithPublicInputs};

impl<F: RichField + Extendable<D>, const D: usize> ProofWithPublicInputs<F, D> {
    pub(crate) fn fri_query_indices(
        &self,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<Vec<usize>> {
        Ok(self.get_challenges(common_data)?.fri_query_indices)
    }

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
        let fri_betas = match &self.proof.opening_proof {
            FriProof::Decompressed(p) => &p.commit_phase_merkle_caps,
            FriProof::Compressed(p) => &p.commit_phase_merkle_caps,
        }
        .iter()
        .map(|cap| {
            challenger.observe_cap(cap);
            challenger.get_extension_challenge()
        })
        .collect();

        challenger.observe_extension_elements(
            &match &self.proof.opening_proof {
                FriProof::Decompressed(p) => &p.final_poly,
                FriProof::Compressed(p) => &p.final_poly,
            }
            .coeffs,
        );

        let fri_pow_response = hash_n_to_1(
            challenger
                .get_hash()
                .elements
                .iter()
                .copied()
                .chain(Some(match &self.proof.opening_proof {
                    FriProof::Decompressed(p) => p.pow_witness,
                    FriProof::Compressed(p) => p.pow_witness,
                }))
                .collect(),
            false,
        );

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
            fri_pow_response,
            fri_query_indices,
        })
    }
}
