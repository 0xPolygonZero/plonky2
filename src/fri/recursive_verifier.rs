use anyhow::{ensure, Result};
use itertools::izip;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::{flatten_target, ExtensionTarget};
use crate::field::extension_field::{flatten, Extendable, FieldExtension, OEF};
use crate::field::field::Field;
use crate::field::lagrange::{barycentric_weights, interpolant, interpolate};
use crate::fri::FriConfig;
use crate::hash::hash_n_to_1;
use crate::merkle_proofs::verify_merkle_proof;
use crate::plonk_challenger::{Challenger, RecursiveChallenger};
use crate::plonk_common::reduce_with_iter;
use crate::proof::{
    FriInitialTreeProof, FriInitialTreeProofTarget, FriProof, FriProofTarget, FriQueryRound,
    FriQueryRoundTarget, Hash, HashTarget, OpeningSet, OpeningSetTarget,
};
use crate::target::Target;
use crate::util::{log2_strict, reverse_bits, reverse_index_bits_in_place};

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Computes P'(x^arity) from {P(x*g^i)}_(i=0..arity), where g is a `arity`-th root of unity
    /// and P' is the FRI reduced polynomial.
    fn compute_evaluation(
        &mut self,
        x: Target,
        old_x_index: Target,
        arity_bits: usize,
        last_evals: &[ExtensionTarget<D>],
        beta: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        todo!()
        // debug_assert_eq!(last_evals.len(), 1 << arity_bits);
        //
        // let g = F::primitive_root_of_unity(arity_bits);
        //
        // // The evaluation vector needs to be reordered first.
        // let mut evals = last_evals.to_vec();
        // reverse_index_bits_in_place(&mut evals);
        // evals.rotate_left(reverse_bits(old_x_index, arity_bits));
        //
        // // The answer is gotten by interpolating {(x*g^i, P(x*g^i))} and evaluating at beta.
        // let points = g
        //     .powers()
        //     .zip(evals)
        //     .map(|(y, e)| ((x * y).into(), e))
        //     .collect::<Vec<_>>();
        // let barycentric_weights = barycentric_weights(&points);
        // interpolate(&points, beta, &barycentric_weights)
    }

    fn fri_verify_proof_of_work(
        &mut self,
        proof: &FriProofTarget<D>,
        challenger: &mut RecursiveChallenger,
        config: &FriConfig,
    ) -> Result<()> {
        let mut inputs = challenger.get_hash(self).elements.to_vec();
        inputs.push(proof.pow_witness);

        let hash = self.hash_n_to_m(inputs, 1, false)[0];
        self.assert_trailing_zeros::<64>(hash, config.proof_of_work_bits);

        Ok(())
    }

    // pub fn verify_fri_proof<const D: usize>(
    //     purported_degree_log: usize,
    //     // Openings of the PLONK polynomials.
    //     os: &OpeningSet<F, D>,
    //     // Point at which the PLONK polynomials are opened.
    //     zeta: F::Extension,
    //     // Scaling factor to combine polynomials.
    //     alpha: F::Extension,
    //     initial_merkle_roots: &[Hash<F>],
    //     proof: &FriProof<F, D>,
    //     challenger: &mut Challenger<F>,
    //     config: &FriConfig,
    // ) -> Result<()> {
    //     let total_arities = config.reduction_arity_bits.iter().sum::<usize>();
    //     ensure!(
    //         purported_degree_log
    //             == log2_strict(proof.final_poly.len()) + total_arities - config.rate_bits,
    //         "Final polynomial has wrong degree."
    //     );
    //
    //     // Size of the LDE domain.
    //     let n = proof.final_poly.len() << total_arities;
    //
    //     // Recover the random betas used in the FRI reductions.
    //     let betas = proof
    //         .commit_phase_merkle_roots
    //         .iter()
    //         .map(|root| {
    //             challenger.observe_hash(root);
    //             challenger.get_extension_challenge()
    //         })
    //         .collect::<Vec<_>>();
    //     challenger.observe_extension_elements(&proof.final_poly.coeffs);
    //
    //     // Check PoW.
    //     fri_verify_proof_of_work(proof, challenger, config)?;
    //
    //     // Check that parameters are coherent.
    //     ensure!(
    //         config.num_query_rounds == proof.query_round_proofs.len(),
    //         "Number of query rounds does not match config."
    //     );
    //     ensure!(
    //         !config.reduction_arity_bits.is_empty(),
    //         "Number of reductions should be non-zero."
    //     );
    //
    //     for round_proof in &proof.query_round_proofs {
    //         fri_verifier_query_round(
    //             os,
    //             zeta,
    //             alpha,
    //             initial_merkle_roots,
    //             &proof,
    //             challenger,
    //             n,
    //             &betas,
    //             round_proof,
    //             config,
    //         )?;
    //     }
    //
    //     Ok(())
    // }
    //

    fn fri_verify_initial_proof(
        &mut self,
        x_index: Target,
        proof: &FriInitialTreeProofTarget,
        initial_merkle_roots: &[HashTarget],
    ) {
        for ((evals, merkle_proof), &root) in proof.evals_proofs.iter().zip(initial_merkle_roots) {
            self.verify_merkle_proof(evals.clone(), x_index, root, merkle_proof);
        }
    }

    fn fri_combine_initial(
        &mut self,
        proof: &FriInitialTreeProofTarget,
        alpha: ExtensionTarget<D>,
        os: &OpeningSetTarget<D>,
        zeta: ExtensionTarget<D>,
        subgroup_x: Target,
    ) -> ExtensionTarget<D> {
        assert!(D > 1, "Not implemented for D=1.");
        let config = &self.config.fri_config.clone();
        let degree_log = proof.evals_proofs[0].1.siblings.len() - config.rate_bits;
        let subgroup_x = self.convert_to_ext(subgroup_x);
        let mut alpha_powers = self.powers(alpha);
        let mut sum = self.zero_extension();

        // We will add three terms to `sum`:
        // - one for polynomials opened at `x` only
        // - one for polynomials opened at `x` and `g x`
        // - one for polynomials opened at `x` and its conjugate

        let evals = [0, 1, 4]
            .iter()
            .flat_map(|&i| proof.unsalted_evals(i, config))
            .map(|&e| self.convert_to_ext(e))
            .collect::<Vec<_>>();
        let openings = os
            .constants
            .iter()
            .chain(&os.plonk_sigmas)
            .chain(&os.quotient_polys);
        let mut numerator = self.zero_extension();
        for (e, &o) in izip!(evals, openings) {
            let a = alpha_powers.next(self);
            let diff = self.sub_extension(e, o);
            numerator = self.mul_add_extension(a, diff, numerator);
        }
        let denominator = self.sub_extension(subgroup_x, zeta);
        let quotient = self.div_unsafe_extension(numerator, denominator);
        let sum = self.add_extension(sum, quotient);

        let evs = proof
            .unsalted_evals(3, config)
            .iter()
            .map(|&e| self.convert_to_ext(e))
            .collect::<Vec<_>>();
        // TODO: Would probably be more efficient using `CircuitBuilder::reduce_with_powers_recursive`
        let mut ev = self.zero_extension();
        for &e in &evs {
            let a = alpha_powers.next(self);
            ev = self.mul_add_extension(a, e, ev);
        }

        let g = self.constant_extension(F::Extension::primitive_root_of_unity(degree_log));
        let zeta_right = self.mul_extension(g, zeta);
        let mut ev_zeta = self.zero_extension();
        for &t in &os.plonk_zs {
            let a = alpha_powers.next(self);
            ev_zeta = self.mul_add_extension(a, t, ev_zeta);
        }
        let mut ev_zeta_right = self.zero_extension();
        for &t in &os.plonk_zs_right {
            let a = alpha_powers.next(self);
            ev_zeta_right = self.mul_add_extension(a, t, ev_zeta);
        }
        let zs_interpol = self.interpolate2([(zeta, ev_zeta), (zeta_right, ev_zeta_right)]);
        let interpol_val = zs_interpol.eval(self, subgroup_x);
        let numerator = self.sub_extension(ev, interpol_val);
        let vanish = self.sub_extension(subgroup_x, zeta);
        let vanish_right = self.sub_extension(subgroup_x, zeta_right);
        let denominator = self.mul_extension(vanish, vanish_right);
        let quotient = self.div_unsafe_extension(numerator, denominator);
        let sum = self.add_extension(sum, quotient);

        let evs = proof
            .unsalted_evals(2, config)
            .iter()
            .map(|&e| self.convert_to_ext(e))
            .collect::<Vec<_>>();
        let mut ev = self.zero_extension();
        for &e in &evs {
            let a = alpha_powers.next(self);
            ev = self.mul_add_extension(a, e, ev);
        }
        let zeta_frob = zeta.frobenius(self);
        let wire_evals_frob = os
            .wires
            .iter()
            .map(|e| e.frobenius(self))
            .collect::<Vec<_>>();
        let mut ev_zeta = self.zero_extension();
        for &t in &os.wires {
            let a = alpha_powers.next(self);
            ev_zeta = self.mul_add_extension(a, t, ev_zeta);
        }
        let mut ev_zeta_frob = self.zero_extension();
        for &t in &wire_evals_frob {
            let a = alpha_powers.next(self);
            ev_zeta_right = self.mul_add_extension(a, t, ev_zeta);
        }
        let wires_interpol = self.interpolate2([(zeta, ev_zeta), (zeta_frob, ev_zeta_frob)]);
        let interpol_val = wires_interpol.eval(self, subgroup_x);
        let numerator = self.sub_extension(ev, interpol_val);
        let vanish_frob = self.sub_extension(subgroup_x, zeta_frob);
        let denominator = self.mul_extension(vanish, vanish_frob);
        let quotient = self.div_unsafe_extension(numerator, denominator);
        let sum = self.add_extension(sum, quotient);

        sum
    }

    fn fri_verifier_query_round(
        &mut self,
        os: &OpeningSetTarget<D>,
        zeta: ExtensionTarget<D>,
        alpha: ExtensionTarget<D>,
        initial_merkle_roots: &[HashTarget],
        proof: &FriProofTarget<D>,
        challenger: &mut RecursiveChallenger,
        n: usize,
        betas: &[ExtensionTarget<D>],
        round_proof: &FriQueryRoundTarget<D>,
        config: &FriConfig,
    ) -> Result<()> {
        let mut evaluations: Vec<Vec<ExtensionTarget<D>>> = Vec::new();
        // TODO: Do we need to range check `x_index` to a target smaller than `p`?
        let mut x_index = challenger.get_challenge(self);
        let mut domain_size = n;
        self.fri_verify_initial_proof(
            x_index,
            &round_proof.initial_trees_proof,
            initial_merkle_roots,
        );
        let mut old_x_index = self.zero();
        // `subgroup_x` is `subgroup[x_index]`, i.e., the actual field element in the domain.
        let log_n = log2_strict(n);
        // TODO: The verifier will need to check these constants at some point (out of circuit).
        let g = self.constant(F::MULTIPLICATIVE_GROUP_GENERATOR);
        let phi = self.constant(F::primitive_root_of_unity(log_n));

        let reversed_x = self.reverse_bits::<2>(x_index, log_n);
        let phi = self.exp(phi, reversed_x);
        let mut subgroup_x = self.mul(g, phi);

        for (i, &arity_bits) in config.reduction_arity_bits.iter().enumerate() {
            let arity = 1 << arity_bits;
            let next_domain_size = domain_size >> arity_bits;
            let e_x = if i == 0 {
                self.fri_combine_initial(
                    &round_proof.initial_trees_proof,
                    alpha,
                    os,
                    zeta,
                    subgroup_x,
                )
            } else {
                let last_evals = &evaluations[i - 1];
                // Infer P(y) from {P(x)}_{x^arity=y}.
                self.compute_evaluation(
                    subgroup_x,
                    old_x_index,
                    config.reduction_arity_bits[i - 1],
                    last_evals,
                    betas[i - 1],
                )
            };
            let mut evals = round_proof.steps[i].evals.clone();
            // Insert P(y) into the evaluation vector, since it wasn't included by the prover.
            // evals.insert(x_index & (arity - 1), e_x);
            // evaluations.push(evals);
            // self.verify_merkle_proof(
            //     flatten_target(&evaluations[i]),
            //     x_index >> arity_bits,
            //     proof.commit_phase_merkle_roots[i],
            //     &round_proof.steps[i].merkle_proof,
            // )?;

            if i > 0 {
                // Update the point x to x^arity.
                for _ in 0..config.reduction_arity_bits[i - 1] {
                    subgroup_x = self.mul(subgroup_x, subgroup_x);
                }
            }
            domain_size = next_domain_size;
            old_x_index = x_index;
            // x_index >>= arity_bits;
        }

        let last_evals = evaluations.last().unwrap();
        let final_arity_bits = *config.reduction_arity_bits.last().unwrap();
        let purported_eval = self.compute_evaluation(
            subgroup_x,
            old_x_index,
            final_arity_bits,
            last_evals,
            *betas.last().unwrap(),
        );
        for _ in 0..final_arity_bits {
            subgroup_x = self.mul(subgroup_x, subgroup_x);
        }

        // Final check of FRI. After all the reductions, we check that the final polynomial is equal
        // to the one sent by the prover.
        let eval = proof.final_poly.eval_scalar(self, subgroup_x);
        self.assert_equal_extension(eval, purported_eval);

        Ok(())
    }
}
