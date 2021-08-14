use crate::field::extension_field::target::{flatten_target, ExtensionTarget};
use crate::field::extension_field::Extendable;
use crate::field::field_types::Field;
use crate::fri::proof::{FriInitialTreeProofTarget, FriProofTarget, FriQueryRoundTarget};
use crate::fri::FriConfig;
use crate::hash::hash_types::MerkleCapTarget;
use crate::iop::challenger::RecursiveChallenger;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::plonk_common::PlonkPolynomials;
use crate::plonk::proof::OpeningSetTarget;
use crate::util::reducing::ReducingFactorTarget;
use crate::util::{log2_strict, reverse_index_bits_in_place};
use crate::with_context;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Computes P'(x^arity) from {P(x*g^i)}_(i=0..arity), where g is a `arity`-th root of unity
    /// and P' is the FRI reduced polynomial.
    fn compute_evaluation(
        &mut self,
        x: Target,
        x_index_within_coset_bits: &[BoolTarget],
        arity_bits: usize,
        evals: &[ExtensionTarget<D>],
        beta: ExtensionTarget<D>,
    ) -> ExtensionTarget<D> {
        let arity = 1 << arity_bits;
        debug_assert_eq!(evals.len(), arity);

        let g = F::primitive_root_of_unity(arity_bits);
        let g_inv = g.exp((arity as u64) - 1);
        let g_inv_t = self.constant(g_inv);

        // The evaluation vector needs to be reordered first.
        let mut evals = evals.to_vec();
        reverse_index_bits_in_place(&mut evals);
        // Want `g^(arity - rev_x_index_within_coset)` as in the out-of-circuit version. Compute it
        // as `(g^-1)^rev_x_index_within_coset`.
        let start = self.exp_from_bits(g_inv_t, x_index_within_coset_bits.iter().rev());
        let coset_start = self.mul(start, x);

        // The answer is gotten by interpolating {(x*g^i, P(x*g^i))} and evaluating at beta.
        let g_powers = g
            .powers()
            .take(arity)
            .map(|y| self.constant(y))
            .collect::<Vec<_>>();
        let mut coset = Vec::new();
        for i in 0..arity / 2 {
            let res = self.mul_two(
                coset_start,
                g_powers[2 * i],
                coset_start,
                g_powers[2 * i + 1],
            );
            coset.push(res.0);
            coset.push(res.1);
        }
        let points = coset.into_iter().zip(evals).collect::<Vec<_>>();

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
        self.assert_leading_zeros(
            hash,
            config.proof_of_work_bits + (64 - F::order().bits()) as u32,
        );
    }

    pub fn verify_fri_proof(
        &mut self,
        // Openings of the PLONK polynomials.
        os: &OpeningSetTarget<D>,
        // Point at which the PLONK polynomials are opened.
        zeta: ExtensionTarget<D>,
        initial_merkle_caps: &[MerkleCapTarget],
        proof: &FriProofTarget<D>,
        challenger: &mut RecursiveChallenger,
        common_data: &CommonCircuitData<F, D>,
    ) {
        let config = &common_data.config;
        let total_arities = config.fri_config.reduction_arity_bits.iter().sum::<usize>();
        debug_assert_eq!(
            common_data.degree_bits,
            log2_strict(proof.final_poly.len()) + total_arities,
            "Final polynomial has wrong degree."
        );

        // Size of the LDE domain.
        let n = proof.final_poly.len() << (total_arities + config.rate_bits);

        challenger.observe_opening_set(&os);

        // Scaling factor to combine polynomials.
        let alpha = challenger.get_extension_challenge(self);

        let betas = with_context!(
            self,
            "recover the random betas used in the FRI reductions.",
            proof
                .commit_phase_merkle_caps
                .iter()
                .map(|cap| {
                    challenger.observe_cap(cap);
                    challenger.get_extension_challenge(self)
                })
                .collect::<Vec<_>>()
        );
        challenger.observe_extension_elements(&proof.final_poly.0);

        with_context!(
            self,
            "check PoW",
            self.fri_verify_proof_of_work(proof, challenger, &config.fri_config)
        );

        // Check that parameters are coherent.
        debug_assert_eq!(
            config.fri_config.num_query_rounds,
            proof.query_round_proofs.len(),
            "Number of query rounds does not match config."
        );
        debug_assert!(
            !config.fri_config.reduction_arity_bits.is_empty(),
            "Number of reductions should be non-zero."
        );

        let precomputed_reduced_evals = with_context!(
            self,
            "precompute reduced evaluations",
            PrecomputedReducedEvalsTarget::from_os_and_alpha(
                os,
                alpha,
                common_data.degree_bits,
                zeta,
                self
            )
        );

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
                &format!("verify one (of {}) query rounds", num_queries),
                self.fri_verifier_query_round(
                    zeta,
                    alpha,
                    precomputed_reduced_evals,
                    initial_merkle_caps,
                    proof,
                    challenger,
                    n,
                    &betas,
                    round_proof,
                    common_data,
                )
            );
        }
    }

    fn fri_verify_initial_proof(
        &mut self,
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
            with_context!(
                self,
                &format!("verify {}'th initial Merkle proof", i),
                self.verify_merkle_proof_with_cap_index(
                    evals.clone(),
                    x_index_bits,
                    cap_index,
                    cap,
                    merkle_proof
                )
            );
        }
    }

    fn fri_combine_initial(
        &mut self,
        proof: &FriInitialTreeProofTarget,
        alpha: ExtensionTarget<D>,
        subgroup_x: Target,
        vanish_zeta: ExtensionTarget<D>,
        precomputed_reduced_evals: PrecomputedReducedEvalsTarget<D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> ExtensionTarget<D> {
        assert!(D > 1, "Not implemented for D=1.");
        let config = self.config.clone();
        let degree_log = common_data.degree_bits;
        debug_assert_eq!(
            degree_log,
            common_data.config.cap_height + proof.evals_proofs[0].1.siblings.len()
                - config.rate_bits
        );
        let subgroup_x = self.convert_to_ext(subgroup_x);
        let mut alpha = ReducingFactorTarget::new(alpha);
        let mut sum = self.zero_extension();

        // We will add three terms to `sum`:
        // - one for polynomials opened at `x` only
        // - one for polynomials opened at `x` and `g x`

        // Polynomials opened at `x`, i.e., the constants-sigmas, wires, quotient and partial products polynomials.
        let single_evals = [
            PlonkPolynomials::CONSTANTS_SIGMAS,
            PlonkPolynomials::WIRES,
            PlonkPolynomials::QUOTIENT,
        ]
        .iter()
        .flat_map(|&p| proof.unsalted_evals(p, config.zero_knowledge))
        .chain(
            &proof.unsalted_evals(PlonkPolynomials::ZS_PARTIAL_PRODUCTS, config.zero_knowledge)
                [common_data.partial_products_range()],
        )
        .copied()
        .collect::<Vec<_>>();
        let single_composition_eval = alpha.reduce_base(&single_evals, self);
        let single_numerator =
            self.sub_extension(single_composition_eval, precomputed_reduced_evals.single);
        sum = self.div_add_extension(single_numerator, vanish_zeta, sum);
        alpha.reset();

        // Polynomials opened at `x` and `g x`, i.e., the Zs polynomials.
        let zs_evals = proof
            .unsalted_evals(PlonkPolynomials::ZS_PARTIAL_PRODUCTS, config.zero_knowledge)
            .iter()
            .take(common_data.zs_range().end)
            .copied()
            .collect::<Vec<_>>();
        let zs_composition_eval = alpha.reduce_base(&zs_evals, self);

        let interpol_val = self.mul_add_extension(
            vanish_zeta,
            precomputed_reduced_evals.slope,
            precomputed_reduced_evals.zs,
        );
        let (zs_numerator, vanish_zeta_right) = self.sub_two_extension(
            zs_composition_eval,
            interpol_val,
            subgroup_x,
            precomputed_reduced_evals.zeta_right,
        );
        let (mut sum, zs_denominator) =
            alpha.shift_and_mul(sum, vanish_zeta, vanish_zeta_right, self);
        sum = self.div_add_extension(zs_numerator, zs_denominator, sum);

        sum
    }

    fn fri_verifier_query_round(
        &mut self,
        zeta: ExtensionTarget<D>,
        alpha: ExtensionTarget<D>,
        precomputed_reduced_evals: PrecomputedReducedEvalsTarget<D>,
        initial_merkle_caps: &[MerkleCapTarget],
        proof: &FriProofTarget<D>,
        challenger: &mut RecursiveChallenger,
        n: usize,
        betas: &[ExtensionTarget<D>],
        round_proof: &FriQueryRoundTarget<D>,
        common_data: &CommonCircuitData<F, D>,
    ) {
        let config = &common_data.config.fri_config;
        let n_log = log2_strict(n);
        // TODO: Do we need to range check `x_index` to a target smaller than `p`?
        let x_index = challenger.get_challenge(self);
        let mut x_index_bits = self.low_bits(x_index, n_log, 64);
        let cap_index = self.le_sum(
            x_index_bits[x_index_bits.len() - common_data.config.fri_config.cap_height..]
                .into_iter(),
        );
        with_context!(
            self,
            "check FRI initial proof",
            self.fri_verify_initial_proof(
                &x_index_bits,
                &round_proof.initial_trees_proof,
                initial_merkle_caps,
                cap_index
            )
        );

        // `subgroup_x` is `subgroup[x_index]`, i.e., the actual field element in the domain.
        let (mut subgroup_x, vanish_zeta) = with_context!(self, "compute x from its index", {
            let g = self.constant(F::coset_shift());
            let phi = self.constant(F::primitive_root_of_unity(n_log));

            let phi = self.exp_from_bits(phi, x_index_bits.iter().rev());
            let g_ext = self.convert_to_ext(g);
            let phi_ext = self.convert_to_ext(phi);
            let zero = self.zero_extension();
            // `subgroup_x = g*phi, vanish_zeta = g*phi - zeta`
            let tmp = self.double_arithmetic_extension(
                F::ONE,
                F::NEG_ONE,
                g_ext,
                phi_ext,
                zero,
                g_ext,
                phi_ext,
                zeta,
            );
            (tmp.0 .0[0], tmp.1)
        });

        // old_eval is the last derived evaluation; it will be checked for consistency with its
        // committed "parent" value in the next iteration.
        let mut old_eval = with_context!(
            self,
            "combine initial oracles",
            self.fri_combine_initial(
                &round_proof.initial_trees_proof,
                alpha,
                subgroup_x,
                vanish_zeta,
                precomputed_reduced_evals,
                common_data,
            )
        );

        for (i, &arity_bits) in config.reduction_arity_bits.iter().enumerate() {
            let evals = &round_proof.steps[i].evals;

            // Split x_index into the index of the coset x is in, and the index of x within that coset.
            let coset_index_bits = x_index_bits[arity_bits..].to_vec();
            let x_index_within_coset_bits = &x_index_bits[..arity_bits];
            let x_index_within_coset = self.le_sum(x_index_within_coset_bits.iter());

            // Check consistency with our old evaluation from the previous round.
            self.random_access_padded(
                x_index_within_coset,
                old_eval,
                evals.clone(),
                1 << config.cap_height,
            );

            // Infer P(y) from {P(x)}_{x^arity=y}.
            old_eval = with_context!(
                self,
                "infer evaluation using interpolation",
                self.compute_evaluation(
                    subgroup_x,
                    &x_index_within_coset_bits,
                    arity_bits,
                    evals,
                    betas[i],
                )
            );

            with_context!(
                self,
                "verify FRI round Merkle proof.",
                self.verify_merkle_proof_with_cap_index(
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
        }

        // Final check of FRI. After all the reductions, we check that the final polynomial is equal
        // to the one sent by the prover.
        let eval = with_context!(
            self,
            "evaluate final polynomial",
            proof.final_poly.eval_scalar(self, subgroup_x)
        );
        self.assert_equal_extension(eval, old_eval);
    }
}

#[derive(Copy, Clone)]
struct PrecomputedReducedEvalsTarget<const D: usize> {
    pub single: ExtensionTarget<D>,
    pub zs: ExtensionTarget<D>,
    pub zs_right: ExtensionTarget<D>,
    /// Slope of the line from `(zeta, zs)` to `(zeta_right, zs_right)`.
    pub slope: ExtensionTarget<D>,
    pub zeta_right: ExtensionTarget<D>,
}

impl<const D: usize> PrecomputedReducedEvalsTarget<D> {
    fn from_os_and_alpha<F: Extendable<D>>(
        os: &OpeningSetTarget<D>,
        alpha: ExtensionTarget<D>,
        degree_log: usize,
        zeta: ExtensionTarget<D>,
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        let mut alpha = ReducingFactorTarget::new(alpha);
        let single = alpha.reduce(
            &os.constants
                .iter()
                .chain(&os.plonk_sigmas)
                .chain(&os.wires)
                .chain(&os.quotient_polys)
                .chain(&os.partial_products)
                .copied()
                .collect::<Vec<_>>(),
            builder,
        );
        let zs = alpha.reduce(&os.plonk_zs, builder);
        let zs_right = alpha.reduce(&os.plonk_zs_right, builder);

        let g = builder.constant_extension(F::Extension::primitive_root_of_unity(degree_log));
        let zeta_right = builder.mul_extension(g, zeta);
        let (numerator, denominator) = builder.sub_two_extension(zs_right, zs, zeta_right, zeta);

        Self {
            single,
            zs,
            zs_right,
            slope: builder.div_extension(numerator, denominator),
            zeta_right,
        }
    }
}
