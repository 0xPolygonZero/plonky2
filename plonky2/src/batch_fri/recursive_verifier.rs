#[cfg(not(feature = "std"))]
use alloc::{format, vec::Vec};

use itertools::Itertools;

use crate::field::extension::Extendable;
use crate::fri::proof::{
    FriChallengesTarget, FriInitialTreeProofTarget, FriProofTarget, FriQueryRoundTarget,
};
use crate::fri::recursive_verifier::PrecomputedReducedOpeningsTarget;
use crate::fri::structure::{FriBatchInfoTarget, FriInstanceInfoTarget, FriOpeningsTarget};
use crate::fri::FriParams;
use crate::hash::hash_types::{MerkleCapTarget, RichField};
use crate::iop::ext_target::{flatten_target, ExtensionTarget};
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::{AlgebraicHasher, GenericConfig};
use crate::util::reducing::ReducingFactorTarget;
use crate::with_context;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn verify_batch_fri_proof<C: GenericConfig<D, F = F>>(
        &mut self,
        degree_bits: &[usize],
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

        let mut precomputed_reduced_evals = Vec::with_capacity(openings.len());
        for opn in openings {
            let pre = with_context!(
                self,
                "precompute reduced evaluations",
                PrecomputedReducedOpeningsTarget::from_os_and_alpha(
                    opn,
                    challenges.fri_alpha,
                    self
                )
            );
            precomputed_reduced_evals.push(pre);
        }
        let degree_bits = degree_bits
            .iter()
            .map(|d| d + params.config.rate_bits)
            .collect_vec();

        for (i, round_proof) in proof.query_round_proofs.iter().enumerate() {
            // To minimize noise in our logs, we will only record a context for a single FRI query.
            // The very first query will have some extra gates due to constants being registered, so
            // the second query is a better representative.
            let level = if i == 1 {
                log::Level::Debug
            } else {
                log::Level::Trace
            };

            let num_queries = proof.query_round_proofs.len();
            with_context!(
                self,
                level,
                &format!("verify one (of {num_queries}) query rounds"),
                self.batch_fri_verifier_query_round::<C>(
                    &degree_bits,
                    instance,
                    challenges,
                    &precomputed_reduced_evals,
                    initial_merkle_caps,
                    proof,
                    challenges.fri_query_indices[i],
                    round_proof,
                    params,
                )
            );
        }
    }

    fn batch_fri_verify_initial_proof<H: AlgebraicHasher<F>>(
        &mut self,
        degree_bits: &[usize],
        instances: &[FriInstanceInfoTarget<D>],
        x_index_bits: &[BoolTarget],
        proof: &FriInitialTreeProofTarget,
        initial_merkle_caps: &[MerkleCapTarget],
        cap_index: Target,
    ) {
        for (i, ((evals, merkle_proof), cap)) in proof
            .evals_proofs
            .iter()
            .zip(initial_merkle_caps)
            .enumerate()
        {
            let leaves = instances
                .iter()
                .scan(0, |leaf_index, inst| {
                    let num_polys = inst.oracles[i].num_polys;
                    let leaves = (*leaf_index..*leaf_index + num_polys)
                        .map(|idx| evals[idx])
                        .collect::<Vec<_>>();
                    *leaf_index += num_polys;
                    Some(leaves)
                })
                .collect::<Vec<_>>();

            with_context!(
                self,
                &format!("verify {i}'th initial Merkle proof"),
                self.verify_batch_merkle_proof_to_cap_with_cap_index::<H>(
                    &leaves,
                    degree_bits,
                    x_index_bits,
                    cap_index,
                    cap,
                    merkle_proof
                )
            );
        }
    }

    fn batch_fri_combine_initial(
        &mut self,
        instance: &[FriInstanceInfoTarget<D>],
        index: usize,
        proof: &FriInitialTreeProofTarget,
        alpha: ExtensionTarget<D>,
        subgroup_x: Target,
        precomputed_reduced_evals: &PrecomputedReducedOpeningsTarget<D>,
        params: &FriParams,
    ) -> ExtensionTarget<D> {
        assert!(D > 1, "Not implemented for D=1.");
        let degree_log = params.degree_bits;
        debug_assert_eq!(
            degree_log,
            params.config.cap_height + proof.evals_proofs[0].1.siblings.len()
                - params.config.rate_bits
        );
        let subgroup_x = self.convert_to_ext(subgroup_x);
        let mut alpha = ReducingFactorTarget::new(alpha);
        let mut sum = self.zero_extension();

        for (batch, reduced_openings) in instance[index]
            .batches
            .iter()
            .zip(&precomputed_reduced_evals.reduced_openings_at_point)
        {
            let FriBatchInfoTarget { point, polynomials } = batch;
            let evals = polynomials
                .iter()
                .map(|p| {
                    let poly_blinding = instance[index].oracles[p.oracle_index].blinding;
                    let salted = params.hiding && poly_blinding;
                    proof.unsalted_eval(p.oracle_index, p.polynomial_index, salted)
                })
                .collect_vec();
            let reduced_evals = alpha.reduce_base(&evals, self);
            let numerator = self.sub_extension(reduced_evals, *reduced_openings);
            let denominator = self.sub_extension(subgroup_x, *point);
            sum = alpha.shift(sum, self);
            sum = self.div_add_extension(numerator, denominator, sum);
        }

        sum
    }

    fn batch_fri_verifier_query_round<C: GenericConfig<D, F = F>>(
        &mut self,
        degree_bits: &[usize],
        instance: &[FriInstanceInfoTarget<D>],
        challenges: &FriChallengesTarget<D>,
        precomputed_reduced_evals: &[PrecomputedReducedOpeningsTarget<D>],
        initial_merkle_caps: &[MerkleCapTarget],
        proof: &FriProofTarget<D>,
        x_index: Target,
        round_proof: &FriQueryRoundTarget<D>,
        params: &FriParams,
    ) where
        C::Hasher: AlgebraicHasher<F>,
    {
        let mut n = degree_bits[0];

        // Note that this `low_bits` decomposition permits non-canonical binary encodings. Here we
        // verify that this has a negligible impact on soundness error.
        Self::assert_noncanonical_indices_ok(&params.config);
        let mut x_index_bits = self.low_bits(x_index, n, F::BITS);

        let cap_index =
            self.le_sum(x_index_bits[x_index_bits.len() - params.config.cap_height..].iter());
        with_context!(
            self,
            "check FRI initial proof",
            self.batch_fri_verify_initial_proof::<C::Hasher>(
                degree_bits,
                instance,
                &x_index_bits,
                &round_proof.initial_trees_proof,
                initial_merkle_caps,
                cap_index
            )
        );

        // `subgroup_x` is `subgroup[x_index]`, i.e., the actual field element in the domain.
        let mut subgroup_x = with_context!(self, "compute x from its index", {
            let g = self.constant(F::coset_shift());
            let phi = F::primitive_root_of_unity(n);
            let phi = self.exp_from_bits_const_base(phi, x_index_bits.iter().rev());
            self.mul(g, phi)
        });

        let mut batch_index = 0;

        // old_eval is the last derived evaluation; it will be checked for consistency with its
        // committed "parent" value in the next iteration.
        let mut old_eval = with_context!(
            self,
            "combine initial oracles",
            self.batch_fri_combine_initial(
                instance,
                batch_index,
                &round_proof.initial_trees_proof,
                challenges.fri_alpha,
                subgroup_x,
                &precomputed_reduced_evals[batch_index],
                params,
            )
        );
        batch_index += 1;

        for (i, &arity_bits) in params.reduction_arity_bits.iter().enumerate() {
            let evals = &round_proof.steps[i].evals;

            // Split x_index into the index of the coset x is in, and the index of x within that coset.
            let coset_index_bits = x_index_bits[arity_bits..].to_vec();
            let x_index_within_coset_bits = &x_index_bits[..arity_bits];
            let x_index_within_coset = self.le_sum(x_index_within_coset_bits.iter());

            // Check consistency with our old evaluation from the previous round.
            let new_eval = self.random_access_extension(x_index_within_coset, evals.clone());
            self.connect_extension(new_eval, old_eval);

            // Infer P(y) from {P(x)}_{x^arity=y}.
            old_eval = with_context!(
                self,
                "infer evaluation using interpolation",
                self.compute_evaluation(
                    subgroup_x,
                    x_index_within_coset_bits,
                    arity_bits,
                    evals,
                    challenges.fri_betas[i],
                )
            );

            with_context!(
                self,
                "verify FRI round Merkle proof.",
                self.verify_merkle_proof_to_cap_with_cap_index::<C::Hasher>(
                    flatten_target(evals),
                    &coset_index_bits,
                    cap_index,
                    &proof.commit_phase_merkle_caps[i],
                    &round_proof.steps[i].merkle_proof,
                )
            );

            // Update the point x to x^arity.
            subgroup_x = self.exp_power_of_2(subgroup_x, arity_bits);

            x_index_bits = coset_index_bits;
            n -= arity_bits;

            if batch_index < degree_bits.len() && n == degree_bits[batch_index] {
                let subgroup_x_init = with_context!(self, "compute init x from its index", {
                    let g = self.constant(F::coset_shift());
                    let phi = F::primitive_root_of_unity(n);
                    let phi = self.exp_from_bits_const_base(phi, x_index_bits.iter().rev());
                    self.mul(g, phi)
                });
                let eval = self.batch_fri_combine_initial(
                    instance,
                    batch_index,
                    &round_proof.initial_trees_proof,
                    challenges.fri_alpha,
                    subgroup_x_init,
                    &precomputed_reduced_evals[batch_index],
                    params,
                );
                old_eval = self.mul_extension(old_eval, challenges.fri_betas[i]);
                old_eval = self.add_extension(old_eval, eval);
                batch_index += 1;
            }
        }

        // Final check of FRI. After all the reductions, we check that the final polynomial is equal
        // to the one sent by the prover.
        let eval = with_context!(
            self,
            &format!(
                "evaluate final polynomial of length {}",
                proof.final_poly.len()
            ),
            proof.final_poly.eval_scalar(self, subgroup_x)
        );
        self.connect_extension(eval, old_eval);
    }
}
