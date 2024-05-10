#[cfg(not(feature = "std"))]
use alloc::{format, vec::Vec};

use crate::field::extension::Extendable;
use crate::fri::proof::{
    FriChallengesTarget, FriProofTarget,
};
use crate::fri::structure::{ FriInstanceInfoTarget, FriOpeningsTarget};
use crate::fri::{ FriParams};
use crate::hash::hash_types::{MerkleCapTarget, RichField};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::{AlgebraicHasher, GenericConfig};
use crate::with_context;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn verify_batch_fri_proof<C: GenericConfig<D, F = F>>(
        &mut self,
        instance: &[FriInstanceInfoTarget<D>],
        openings: &[FriOpeningsTarget<D>],
        challenges: &FriChallengesTarget<D>,
        initial_merkle_caps: &[MerkleCapTarget],
        proof: &FriProofTarget<D>,
        params: &FriParams,
    ) where
        C::Hasher: AlgebraicHasher<F>,
    {
        if let Some(max_arity_bits) = params.max_arity_bits() {
            self.check_recursion_config(max_arity_bits);
        }

        debug_assert_eq!(
            params.final_poly_len(),
            proof.final_poly.len(),
            "Final polynomial has wrong degree."
        );

        // Size of the LDE domain.
        let n = params.lde_size();

        with_context!(
            self,
            "check PoW",
            self.fri_verify_proof_of_work(challenges.fri_pow_response, &params.config)
        );

        // Check that parameters are coherent.
        debug_assert_eq!(
            params.config.num_query_rounds,
            proof.query_round_proofs.len(),
            "Number of query rounds does not match config."
        );

        // let precomputed_reduced_evals = with_context!(
        //     self,
        //     "precompute reduced evaluations",
        //     PrecomputedReducedOpeningsTarget::from_os_and_alpha(
        //         openings,
        //         challenges.fri_alpha,
        //         self
        //     )
        // );
        //
        // for (i, round_proof) in proof.query_round_proofs.iter().enumerate() {
        //     // To minimize noise in our logs, we will only record a context for a single FRI query.
        //     // The very first query will have some extra gates due to constants being registered, so
        //     // the second query is a better representative.
        //     let level = if i == 1 {
        //         log::Level::Debug
        //     } else {
        //         log::Level::Trace
        //     };
        //
        //     let num_queries = proof.query_round_proofs.len();
        //     with_context!(
        //         self,
        //         level,
        //         &format!("verify one (of {num_queries}) query rounds"),
        //         self.fri_verifier_query_round::<C>(
        //             instance,
        //             challenges,
        //             &precomputed_reduced_evals,
        //             initial_merkle_caps,
        //             proof,
        //             challenges.fri_query_indices[i],
        //             n,
        //             round_proof,
        //             params,
        //         )
        //     );
        // }
    }
}
