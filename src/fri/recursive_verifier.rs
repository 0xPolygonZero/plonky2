use itertools::izip;

use crate::circuit_builder::CircuitBuilder;
use crate::field::extension_field::target::{flatten_target, ExtensionTarget};
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::fri::FriConfig;
use crate::plonk_challenger::RecursiveChallenger;
use crate::plonk_common::PlonkPolynomials;
use crate::proof::{
    FriInitialTreeProofTarget, FriProofTarget, FriQueryRoundTarget, HashTarget, OpeningSetTarget,
};
use crate::target::Target;
use crate::util::{log2_strict, reverse_index_bits_in_place};

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
        debug_assert_eq!(last_evals.len(), 1 << arity_bits);

        let g = F::primitive_root_of_unity(arity_bits);

        // The evaluation vector needs to be reordered first.
        let mut evals = last_evals.to_vec();
        reverse_index_bits_in_place(&mut evals);
        let mut old_x_index_bits = self.split_le(old_x_index, arity_bits);
        old_x_index_bits.reverse();
        let evals = self.rotate_left_from_bits(&old_x_index_bits, &evals);

        // The answer is gotten by interpolating {(x*g^i, P(x*g^i))} and evaluating at beta.
        let points = g
            .powers()
            .map(|y| {
                let yt = self.constant(y);
                self.mul(x, yt)
            })
            .zip(evals)
            .collect::<Vec<_>>();

        self.interpolate(&points, beta)
    }

    fn fri_verify_proof_of_work(
        &mut self,
        proof: &FriProofTarget<D>,
        challenger: &mut RecursiveChallenger,
        config: &FriConfig,
    ) {
        let mut inputs = challenger.get_hash(self).elements.to_vec();
        inputs.push(proof.pow_witness);

        let hash = self.hash_n_to_m(inputs, 1, false)[0];
        self.assert_leading_zeros(hash, config.proof_of_work_bits + F::ORDER.leading_zeros());
    }

    pub fn verify_fri_proof(
        &mut self,
        purported_degree_log: usize,
        // Openings of the PLONK polynomials.
        os: &OpeningSetTarget<D>,
        // Point at which the PLONK polynomials are opened.
        zeta: ExtensionTarget<D>,
        // Scaling factor to combine polynomials.
        alpha: ExtensionTarget<D>,
        initial_merkle_roots: &[HashTarget],
        proof: &FriProofTarget<D>,
        challenger: &mut RecursiveChallenger,
        config: &FriConfig,
    ) {
        let total_arities = config.reduction_arity_bits.iter().sum::<usize>();
        debug_assert_eq!(
            purported_degree_log,
            log2_strict(proof.final_poly.len()) + total_arities - config.rate_bits,
            "Final polynomial has wrong degree."
        );

        // Size of the LDE domain.
        let n = proof.final_poly.len() << total_arities;

        // Recover the random betas used in the FRI reductions.
        let betas = proof
            .commit_phase_merkle_roots
            .iter()
            .map(|root| {
                challenger.observe_hash(root);
                challenger.get_extension_challenge(self)
            })
            .collect::<Vec<_>>();
        challenger.observe_extension_elements(&proof.final_poly.0);

        // Check PoW.
        self.fri_verify_proof_of_work(proof, challenger, config);

        // Check that parameters are coherent.
        debug_assert_eq!(
            config.num_query_rounds,
            proof.query_round_proofs.len(),
            "Number of query rounds does not match config."
        );
        debug_assert!(
            !config.reduction_arity_bits.is_empty(),
            "Number of reductions should be non-zero."
        );

        for round_proof in &proof.query_round_proofs {
            self.fri_verifier_query_round(
                os,
                zeta,
                alpha,
                initial_merkle_roots,
                &proof,
                challenger,
                n,
                &betas,
                round_proof,
                config,
            );
        }
    }

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
        // - one for polynomials opened at `x` and `x.frobenius()`

        // Polynomials opened at `x`, i.e., the constants, sigmas and quotient polynomials.
        let single_evals = [
            PlonkPolynomials::CONSTANTS,
            PlonkPolynomials::SIGMAS,
            PlonkPolynomials::QUOTIENT,
        ]
        .iter()
        .flat_map(|&p| proof.unsalted_evals(p))
        .map(|&e| self.convert_to_ext(e))
        .collect::<Vec<_>>();
        let single_openings = os
            .constants
            .iter()
            .chain(&os.plonk_sigmas)
            .chain(&os.quotient_polys);
        let mut single_numerator = self.zero_extension();
        for (e, &o) in izip!(single_evals, single_openings) {
            let a = alpha_powers.next(self);
            let diff = self.sub_extension(e, o);
            single_numerator = self.mul_add_extension(a, diff, single_numerator);
        }
        let single_denominator = self.sub_extension(subgroup_x, zeta);
        let quotient = self.div_unsafe_extension(single_numerator, single_denominator);
        sum = self.add_extension(sum, quotient);

        // Polynomials opened at `x` and `g x`, i.e., the Zs polynomials.
        let zs_evals = proof
            .unsalted_evals(PlonkPolynomials::ZS)
            .iter()
            .map(|&e| self.convert_to_ext(e))
            .collect::<Vec<_>>();
        // TODO: Would probably be more efficient using `CircuitBuilder::reduce_with_powers_recursive`
        let mut zs_composition_eval = self.zero_extension();
        let mut alpha_powers_cloned = alpha_powers.clone();
        for &e in &zs_evals {
            let a = alpha_powers_cloned.next(self);
            zs_composition_eval = self.mul_add_extension(a, e, zs_composition_eval);
        }

        let g = self.constant_extension(F::Extension::primitive_root_of_unity(degree_log));
        let zeta_right = self.mul_extension(g, zeta);
        let mut zs_ev_zeta = self.zero_extension();
        let mut alpha_powers_cloned = alpha_powers.clone();
        for &t in &os.plonk_zs {
            let a = alpha_powers_cloned.next(self);
            zs_ev_zeta = self.mul_add_extension(a, t, zs_ev_zeta);
        }
        let mut zs_ev_zeta_right = self.zero_extension();
        for &t in &os.plonk_zs_right {
            let a = alpha_powers.next(self);
            zs_ev_zeta_right = self.mul_add_extension(a, t, zs_ev_zeta);
        }
        let interpol_val = self.interpolate2(
            [(zeta, zs_ev_zeta), (zeta_right, zs_ev_zeta_right)],
            subgroup_x,
        );
        let zs_numerator = self.sub_extension(zs_composition_eval, interpol_val);
        let vanish_zeta = self.sub_extension(subgroup_x, zeta);
        let vanish_zeta_right = self.sub_extension(subgroup_x, zeta_right);
        let zs_denominator = self.mul_extension(vanish_zeta, vanish_zeta_right);
        let zs_quotient = self.div_unsafe_extension(zs_numerator, zs_denominator);
        sum = self.add_extension(sum, zs_quotient);

        // Polynomials opened at `x` and `x.frobenius()`, i.e., the wires polynomials.
        let wire_evals = proof
            .unsalted_evals(PlonkPolynomials::WIRES)
            .iter()
            .map(|&e| self.convert_to_ext(e))
            .collect::<Vec<_>>();
        let mut wire_composition_eval = self.zero_extension();
        let mut alpha_powers_cloned = alpha_powers.clone();
        for &e in &wire_evals {
            let a = alpha_powers_cloned.next(self);
            wire_composition_eval = self.mul_add_extension(a, e, wire_composition_eval);
        }
        let mut alpha_powers_cloned = alpha_powers.clone();
        let wire_eval = os.wires.iter().fold(self.zero_extension(), |acc, &w| {
            let a = alpha_powers_cloned.next(self);
            self.mul_add_extension(a, w, acc)
        });
        let mut alpha_powers_frob = alpha_powers.repeated_frobenius(D - 1, self);
        let wire_eval_frob = os
            .wires
            .iter()
            .fold(self.zero_extension(), |acc, &w| {
                let a = alpha_powers_frob.next(self);
                self.mul_add_extension(a, w, acc)
            })
            .frobenius(self);
        let zeta_frob = zeta.frobenius(self);
        let wire_interpol_val =
            self.interpolate2([(zeta, wire_eval), (zeta_frob, wire_eval_frob)], subgroup_x);
        let wire_numerator = self.sub_extension(wire_composition_eval, wire_interpol_val);
        let vanish_zeta_frob = self.sub_extension(subgroup_x, zeta_frob);
        let wire_denominator = self.mul_extension(vanish_zeta, vanish_zeta_frob);
        let wire_quotient = self.div_unsafe_extension(wire_numerator, wire_denominator);
        sum = self.add_extension(sum, wire_quotient);

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
    ) {
        let n_log = log2_strict(n);
        let mut evaluations: Vec<Vec<ExtensionTarget<D>>> = Vec::new();
        // TODO: Do we need to range check `x_index` to a target smaller than `p`?
        let mut x_index = challenger.get_challenge(self);
        x_index = self.split_low_high(x_index, n_log, 64).0;
        let mut x_index_num_bits = n_log;
        let mut domain_size = n;
        self.fri_verify_initial_proof(
            x_index,
            &round_proof.initial_trees_proof,
            initial_merkle_roots,
        );
        let mut old_x_index = self.zero();
        // `subgroup_x` is `subgroup[x_index]`, i.e., the actual field element in the domain.
        let g = self.constant(F::MULTIPLICATIVE_GROUP_GENERATOR);
        let phi = self.constant(F::primitive_root_of_unity(n_log));

        let reversed_x = self.reverse_limbs::<2>(x_index, n_log);
        let phi = self.exp(phi, reversed_x, n_log);
        let mut subgroup_x = self.mul(g, phi);

        for (i, &arity_bits) in config.reduction_arity_bits.iter().enumerate() {
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
            let (low_x_index, high_x_index) =
                self.split_low_high(x_index, arity_bits, x_index_num_bits);
            evals = self.insert(low_x_index, e_x, evals);
            evaluations.push(evals);
            self.verify_merkle_proof(
                flatten_target(&evaluations[i]),
                high_x_index,
                proof.commit_phase_merkle_roots[i],
                &round_proof.steps[i].merkle_proof,
            );

            if i > 0 {
                // Update the point x to x^arity.
                for _ in 0..config.reduction_arity_bits[i - 1] {
                    subgroup_x = self.square(subgroup_x);
                }
            }
            domain_size = next_domain_size;
            old_x_index = low_x_index;
            x_index = high_x_index;
            x_index_num_bits -= arity_bits;
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
            subgroup_x = self.square(subgroup_x);
        }

        // Final check of FRI. After all the reductions, we check that the final polynomial is equal
        // to the one sent by the prover.
        let eval = proof.final_poly.eval_scalar(self, subgroup_x);
        self.assert_equal_extension(eval, purported_eval);
    }
}
