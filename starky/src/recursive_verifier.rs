use plonky2::field::extension_field::Extendable;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::GenericConfig;
use crate::config::StarkConfig;
use crate::proof::StarkProofWithPublicInputsTarget;
use crate::stark::Stark;

pub fn verify_stark_proof<    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    S: Stark<F, D>,
    const D: usize,
    >(
        builder: &mut CircuitBuilder<F, D>,
        stark: S,
        proof_with_pis: StarkProofWithPublicInputsTarget<D>,
        inner_config: &StarkConfig
    )
    {
        let StarkProofWithPublicInputsTarget {
            proof,
            public_inputs,
        } = proof_with_pis;

        assert_eq!(public_inputs.len(), inner_common_data.num_public_inputs);
        let public_inputs_hash = self.hash_n_to_hash_no_pad::<C::InnerHasher>(public_inputs);

        self.verify_proof(
            proof,
            public_inputs_hash,
            inner_verifier_data,
            inner_common_data,
        );
    }

    /// Recursively verifies an inner proof.
    pub fn verify_proof<C: GenericConfig<D, F = F>>(
        &mut self,
        proof: ProofTarget<D>,
        public_inputs_hash: HashOutTarget,
        inner_verifier_data: &VerifierCircuitTarget,
        inner_common_data: &CommonCircuitData<F, C, D>,
    ) where
        C::Hasher: AlgebraicHasher<F>,
    {
        let one = self.one_extension();

        let num_challenges = inner_common_data.config.num_challenges;

        let mut challenger = RecursiveChallenger::<F, C::Hasher, D>::new(self);

        let (betas, gammas, alphas, zeta) =
            with_context!(self, "observe proof and generates challenges", {
                // Observe the instance.
                let digest = HashOutTarget::from_vec(
                    self.constants(&inner_common_data.circuit_digest.elements),
                );
                challenger.observe_hash(&digest);
                challenger.observe_hash(&public_inputs_hash);

                challenger.observe_cap(&proof.wires_cap);
                let betas = challenger.get_n_challenges(self, num_challenges);
                let gammas = challenger.get_n_challenges(self, num_challenges);

                challenger.observe_cap(&proof.plonk_zs_partial_products_cap);
                let alphas = challenger.get_n_challenges(self, num_challenges);

                challenger.observe_cap(&proof.quotient_polys_cap);
                let zeta = challenger.get_extension_challenge(self);

                (betas, gammas, alphas, zeta)
            });

        let local_constants = &proof.openings.constants;
        let local_wires = &proof.openings.wires;
        let vars = EvaluationTargets {
            local_constants,
            local_wires,
            public_inputs_hash: &public_inputs_hash,
        };
        let local_zs = &proof.openings.plonk_zs;
        let next_zs = &proof.openings.plonk_zs_right;
        let s_sigmas = &proof.openings.plonk_sigmas;
        let partial_products = &proof.openings.partial_products;

        let zeta_pow_deg = self.exp_power_of_2_extension(zeta, inner_common_data.degree_bits);
        let vanishing_polys_zeta = with_context!(
            self,
            "evaluate the vanishing polynomial at our challenge point, zeta.",
            eval_vanishing_poly_recursively(
                self,
                inner_common_data,
                zeta,
                zeta_pow_deg,
                vars,
                local_zs,
                next_zs,
                partial_products,
                s_sigmas,
                &betas,
                &gammas,
                &alphas,
            )
        );

        with_context!(self, "check vanishing and quotient polynomials.", {
            let quotient_polys_zeta = &proof.openings.quotient_polys;
            let mut scale = ReducingFactorTarget::new(zeta_pow_deg);
            let z_h_zeta = self.sub_extension(zeta_pow_deg, one);
            for (i, chunk) in quotient_polys_zeta
                .chunks(inner_common_data.quotient_degree_factor)
                .enumerate()
            {
                let recombined_quotient = scale.reduce(chunk, self);
                let computed_vanishing_poly = self.mul_extension(z_h_zeta, recombined_quotient);
                self.connect_extension(vanishing_polys_zeta[i], computed_vanishing_poly);
            }
        });

        let merkle_caps = &[
            inner_verifier_data.constants_sigmas_cap.clone(),
            proof.wires_cap,
            proof.plonk_zs_partial_products_cap,
            proof.quotient_polys_cap,
        ];

        let fri_instance = inner_common_data.get_fri_instance_target(self, zeta);
        with_context!(
            self,
            "verify FRI proof",
            self.verify_fri_proof::<C>(
                &fri_instance,
                &proof.openings,
                merkle_caps,
                &proof.opening_proof,
                &mut challenger,
                &inner_common_data.fri_params,
            )
        );
    }

    pub fn add_virtual_proof_with_pis<InnerC: GenericConfig<D, F = F>>(
        &mut self,
        common_data: &CommonCircuitData<F, InnerC, D>,
    ) -> ProofWithPublicInputsTarget<D> {
        let proof = self.add_virtual_proof(common_data);
        let public_inputs = self.add_virtual_targets(common_data.num_public_inputs);
        ProofWithPublicInputsTarget {
            proof,
            public_inputs,
        }
    }

    fn add_virtual_proof<InnerC: GenericConfig<D, F = F>>(
        &mut self,
        common_data: &CommonCircuitData<F, InnerC, D>,
    ) -> ProofTarget<D> {
        let config = &common_data.config;
        let fri_params = &common_data.fri_params;
        let cap_height = fri_params.config.cap_height;

        let num_leaves_per_oracle = &[
            common_data.num_preprocessed_polys(),
            config.num_wires,
            common_data.num_zs_partial_products_polys(),
            common_data.num_quotient_polys(),
        ];

        ProofTarget {
            wires_cap: self.add_virtual_cap(cap_height),
            plonk_zs_partial_products_cap: self.add_virtual_cap(cap_height),
            quotient_polys_cap: self.add_virtual_cap(cap_height),
            openings: self.add_opening_set(common_data),
            opening_proof: self.add_virtual_fri_proof(num_leaves_per_oracle, fri_params),
        }
    }

    fn add_opening_set<InnerC: GenericConfig<D, F = F>>(
        &mut self,
        common_data: &CommonCircuitData<F, InnerC, D>,
    ) -> OpeningSetTarget<D> {
        let config = &common_data.config;
        let num_challenges = config.num_challenges;
        let total_partial_products = num_challenges * common_data.num_partial_products;
        OpeningSetTarget {
            constants: self.add_virtual_extension_targets(common_data.num_constants),
            plonk_sigmas: self.add_virtual_extension_targets(config.num_routed_wires),
            wires: self.add_virtual_extension_targets(config.num_wires),
            plonk_zs: self.add_virtual_extension_targets(num_challenges),
            plonk_zs_right: self.add_virtual_extension_targets(num_challenges),
            partial_products: self.add_virtual_extension_targets(total_partial_products),
            quotient_polys: self.add_virtual_extension_targets(common_data.num_quotient_polys()),
        }
    }
}
