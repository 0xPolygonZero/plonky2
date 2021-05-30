use anyhow::Result;

use crate::circuit_data::{CommonCircuitData, VerifierOnlyCircuitData};
use crate::field::extension_field::Extendable;
use crate::plonk_challenger::Challenger;
use crate::proof::Proof;

pub(crate) fn verify<F: Extendable<D>, const D: usize>(
    proof: Proof<F, D>,
    verifier_data: &VerifierOnlyCircuitData<F>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    let config = &common_data.config;
    let fri_config = &config.fri_config;
    let num_challenges = config.num_challenges;

    let mut challenger = Challenger::new();
    // Observe the instance.
    // TODO: Need to include public inputs as well.
    challenger.observe_hash(&common_data.circuit_digest);

    challenger.observe_hash(&proof.wires_root);
    let betas = challenger.get_n_challenges(num_challenges);
    let gammas = challenger.get_n_challenges(num_challenges);

    challenger.observe_hash(&proof.plonk_zs_root);
    let alphas = challenger.get_n_challenges(num_challenges);

    challenger.observe_hash(&proof.quotient_polys_root);
    let zeta = challenger.get_extension_challenge();

    // TODO: Compute PI(zeta), Z_H(zeta), etc. and check the identity at zeta.

    let evaluations = todo!();

    let merkle_roots = &[
        verifier_data.constants_root,
        verifier_data.sigmas_root,
        proof.wires_root,
        proof.plonk_zs_root,
        proof.quotient_polys_root,
    ];

    proof.opening_proof.verify(
        &[zeta],
        evaluations,
        merkle_roots,
        &mut challenger,
        fri_config,
    )?;

    Ok(())
}
