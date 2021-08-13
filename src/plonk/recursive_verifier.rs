use crate::field::extension_field::Extendable;
use crate::hash::hash_types::HashOutTarget;
use crate::iop::challenger::RecursiveChallenger;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData, VerifierCircuitTarget};
use crate::plonk::proof::ProofWithPublicInputsTarget;
use crate::plonk::vanishing_poly::eval_vanishing_poly_recursively;
use crate::plonk::vars::EvaluationTargets;
use crate::util::reducing::ReducingFactorTarget;
use crate::with_context;

const MIN_WIRES: usize = 120; // TODO: Double check.
const MIN_ROUTED_WIRES: usize = 28; // TODO: Double check.

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Recursively verifies an inner proof.
    pub fn add_recursive_verifier(
        &mut self,
        proof_with_pis: ProofWithPublicInputsTarget<D>,
        inner_config: &CircuitConfig,
        inner_verifier_data: &VerifierCircuitTarget,
        inner_common_data: &CommonCircuitData<F, D>,
    ) {
        assert!(self.config.num_wires >= MIN_WIRES);
        assert!(self.config.num_wires >= MIN_ROUTED_WIRES);
        let ProofWithPublicInputsTarget {
            proof,
            public_inputs,
        } = proof_with_pis;
        let one = self.one_extension();

        let num_challenges = inner_config.num_challenges;

        let public_inputs_hash = &self.hash_n_to_hash(public_inputs, true);

        let mut challenger = RecursiveChallenger::new(self);

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
            public_inputs_hash,
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
                self.named_assert_equal_extension(
                    vanishing_polys_zeta[i],
                    computed_vanishing_poly,
                    format!("Vanishing polynomial == Z_H * quotient, challenge {}", i),
                );
            }
        });

        let merkle_caps = &[
            inner_verifier_data.constants_sigmas_cap.clone(),
            proof.wires_cap,
            proof.plonk_zs_partial_products_cap,
            proof.quotient_polys_cap,
        ];

        with_context!(
            self,
            "verify FRI proof",
            self.verify_fri_proof(
                &proof.openings,
                zeta,
                merkle_caps,
                &proof.opening_proof,
                &mut challenger,
                inner_common_data,
            )
        );
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use log::info;

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::fri::proof::{
        FriInitialTreeProofTarget, FriProofTarget, FriQueryRoundTarget, FriQueryStepTarget,
    };
    use crate::fri::FriConfig;
    use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
    use crate::hash::merkle_proofs::MerkleProofTarget;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::proof::{OpeningSetTarget, Proof, ProofTarget, ProofWithPublicInputs};
    use crate::plonk::verifier::verify;
    use crate::util::log2_strict;

    // Construct a `FriQueryRoundTarget` with the same dimensions as the ones in `proof`.
    fn get_fri_query_round<F: Extendable<D>, const D: usize>(
        proof: &Proof<F, D>,
        builder: &mut CircuitBuilder<F, D>,
    ) -> FriQueryRoundTarget<D> {
        let mut query_round = FriQueryRoundTarget {
            initial_trees_proof: FriInitialTreeProofTarget {
                evals_proofs: vec![],
            },
            steps: vec![],
        };
        for (v, merkle_proof) in &proof.opening_proof.query_round_proofs[0]
            .initial_trees_proof
            .evals_proofs
        {
            query_round.initial_trees_proof.evals_proofs.push((
                builder.add_virtual_targets(v.len()),
                MerkleProofTarget {
                    siblings: builder.add_virtual_hashes(merkle_proof.siblings.len()),
                },
            ));
        }
        for step in &proof.opening_proof.query_round_proofs[0].steps {
            query_round.steps.push(FriQueryStepTarget {
                evals: builder.add_virtual_extension_targets(step.evals.len()),
                merkle_proof: MerkleProofTarget {
                    siblings: builder.add_virtual_hashes(step.merkle_proof.siblings.len()),
                },
            });
        }
        query_round
    }

    // Construct a `ProofTarget` with the same dimensions as `proof`.
    fn proof_to_proof_target<F: Extendable<D>, const D: usize>(
        proof_with_pis: &ProofWithPublicInputs<F, D>,
        builder: &mut CircuitBuilder<F, D>,
    ) -> ProofWithPublicInputsTarget<D> {
        let ProofWithPublicInputs {
            proof,
            public_inputs,
        } = proof_with_pis;

        let wires_cap = builder.add_virtual_cap(log2_strict(proof.wires_cap.0.len()));
        let plonk_zs_cap =
            builder.add_virtual_cap(log2_strict(proof.plonk_zs_partial_products_cap.0.len()));
        let quotient_polys_cap =
            builder.add_virtual_cap(log2_strict(proof.quotient_polys_cap.0.len()));

        let openings = OpeningSetTarget {
            constants: builder.add_virtual_extension_targets(proof.openings.constants.len()),
            plonk_sigmas: builder.add_virtual_extension_targets(proof.openings.plonk_sigmas.len()),
            wires: builder.add_virtual_extension_targets(proof.openings.wires.len()),
            plonk_zs: builder.add_virtual_extension_targets(proof.openings.plonk_zs.len()),
            plonk_zs_right: builder
                .add_virtual_extension_targets(proof.openings.plonk_zs_right.len()),
            partial_products: builder
                .add_virtual_extension_targets(proof.openings.partial_products.len()),
            quotient_polys: builder
                .add_virtual_extension_targets(proof.openings.quotient_polys.len()),
        };
        let query_round_proofs = (0..proof.opening_proof.query_round_proofs.len())
            .map(|_| get_fri_query_round(proof, builder))
            .collect();
        let commit_phase_merkle_caps = proof
            .opening_proof
            .commit_phase_merkle_caps
            .iter()
            .map(|r| builder.add_virtual_cap(log2_strict(r.0.len())))
            .collect();
        let opening_proof = FriProofTarget {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly: PolynomialCoeffsExtTarget(
                builder.add_virtual_extension_targets(proof.opening_proof.final_poly.len()),
            ),
            pow_witness: builder.add_virtual_target(),
        };

        let proof = ProofTarget {
            wires_cap,
            plonk_zs_partial_products_cap: plonk_zs_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        };

        let public_inputs = builder.add_virtual_targets(public_inputs.len());
        ProofWithPublicInputsTarget {
            proof,
            public_inputs,
        }
    }

    // Set the targets in a `ProofTarget` to their corresponding values in a `Proof`.
    fn set_proof_target<F: Extendable<D>, const D: usize>(
        proof: &ProofWithPublicInputs<F, D>,
        pt: &ProofWithPublicInputsTarget<D>,
        pw: &mut PartialWitness<F>,
    ) {
        let ProofWithPublicInputs {
            proof,
            public_inputs,
        } = proof;
        let ProofWithPublicInputsTarget {
            proof: pt,
            public_inputs: pi_targets,
        } = pt;

        // Set public inputs.
        for (&pi_t, &pi) in pi_targets.iter().zip(public_inputs) {
            pw.set_target(pi_t, pi);
        }

        pw.set_cap_target(&pt.wires_cap, &proof.wires_cap);
        pw.set_cap_target(
            &pt.plonk_zs_partial_products_cap,
            &proof.plonk_zs_partial_products_cap,
        );
        pw.set_cap_target(&pt.quotient_polys_cap, &proof.quotient_polys_cap);

        for (&t, &x) in pt.openings.wires.iter().zip(&proof.openings.wires) {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt.openings.constants.iter().zip(&proof.openings.constants) {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt
            .openings
            .plonk_sigmas
            .iter()
            .zip(&proof.openings.plonk_sigmas)
        {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt.openings.plonk_zs.iter().zip(&proof.openings.plonk_zs) {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt
            .openings
            .plonk_zs_right
            .iter()
            .zip(&proof.openings.plonk_zs_right)
        {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt
            .openings
            .partial_products
            .iter()
            .zip(&proof.openings.partial_products)
        {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt
            .openings
            .quotient_polys
            .iter()
            .zip(&proof.openings.quotient_polys)
        {
            pw.set_extension_target(t, x);
        }

        let fri_proof = &proof.opening_proof;
        let fpt = &pt.opening_proof;

        pw.set_target(fpt.pow_witness, fri_proof.pow_witness);

        for (&t, &x) in fpt.final_poly.0.iter().zip(&fri_proof.final_poly.coeffs) {
            pw.set_extension_target(t, x);
        }

        for (t, x) in fpt
            .commit_phase_merkle_caps
            .iter()
            .zip(&fri_proof.commit_phase_merkle_caps)
        {
            pw.set_cap_target(t, x);
        }

        for (qt, q) in fpt
            .query_round_proofs
            .iter()
            .zip(&fri_proof.query_round_proofs)
        {
            for (at, a) in qt
                .initial_trees_proof
                .evals_proofs
                .iter()
                .zip(&q.initial_trees_proof.evals_proofs)
            {
                for (&t, &x) in at.0.iter().zip(&a.0) {
                    pw.set_target(t, x);
                }
                for (&t, &x) in at.1.siblings.iter().zip(&a.1.siblings) {
                    pw.set_hash_target(t, x);
                }
            }

            for (st, s) in qt.steps.iter().zip(&q.steps) {
                for (&t, &x) in st.evals.iter().zip(&s.evals) {
                    pw.set_extension_target(t, x);
                }
                for (&t, &x) in st
                    .merkle_proof
                    .siblings
                    .iter()
                    .zip(&s.merkle_proof.siblings)
                {
                    pw.set_hash_target(t, x);
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn test_recursive_verifier() -> Result<()> {
        env_logger::init();
        type F = CrandallField;
        const D: usize = 4;
        let config = CircuitConfig {
            num_wires: 126,
            num_routed_wires: 33,
            security_bits: 128,
            rate_bits: 3,
            num_challenges: 3,
            zero_knowledge: false,
            cap_height: 2,
            fri_config: FriConfig {
                proof_of_work_bits: 1,
                reduction_arity_bits: vec![2, 2, 2, 2, 2, 2],
                num_query_rounds: 40,
                cap_height: 1,
            },
        };
        let (proof_with_pis, vd, cd) = {
            let mut builder = CircuitBuilder::<F, D>::new(config.clone());
            let _two = builder.two();
            let _two = builder.hash_n_to_hash(vec![_two], true).elements[0];
            for _ in 0..10000 {
                let _two = builder.mul(_two, _two);
            }
            let data = builder.build();
            (
                data.prove(PartialWitness::new(config.num_wires))?,
                data.verifier_only,
                data.common,
            )
        };
        verify(proof_with_pis.clone(), &vd, &cd)?;

        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let mut pw = PartialWitness::new(config.num_wires);
        let pt = proof_to_proof_target(&proof_with_pis, &mut builder);
        set_proof_target(&proof_with_pis, &pt, &mut pw);

        let inner_data = VerifierCircuitTarget {
            constants_sigmas_cap: builder.add_virtual_cap(config.cap_height),
        };
        pw.set_cap_target(&inner_data.constants_sigmas_cap, &vd.constants_sigmas_cap);

        builder.add_recursive_verifier(pt, &config, &inner_data, &cd);

        builder.print_gate_counts(0);
        let data = builder.build();
        let recursive_proof = data.prove(pw)?;

        verify(recursive_proof, &data.verifier_only, &data.common)
    }

    #[test]
    #[ignore]
    fn test_recursive_recursive_verifier() -> Result<()> {
        env_logger::init();
        type F = CrandallField;
        const D: usize = 4;
        let config = CircuitConfig {
            num_wires: 126,
            num_routed_wires: 64,
            security_bits: 128,
            rate_bits: 3,
            num_challenges: 3,
            zero_knowledge: false,
            cap_height: 3,
            fri_config: FriConfig {
                proof_of_work_bits: 20,
                reduction_arity_bits: vec![3, 3, 3],
                num_query_rounds: 27,
                cap_height: 3,
            },
        };
        let (proof_with_pis, vd, cd) = {
            let (proof_with_pis, vd, cd) = {
                let mut builder = CircuitBuilder::<F, D>::new(config.clone());
                let _two = builder.two();
                let mut _two = builder.hash_n_to_hash(vec![_two], true).elements[0];
                for _ in 0..20000 {
                    _two = builder.mul(_two, _two);
                }
                let data = builder.build();
                (
                    data.prove(PartialWitness::new(config.num_wires))?,
                    data.verifier_only,
                    data.common,
                )
            };
            verify(proof_with_pis.clone(), &vd, &cd)?;

            let mut builder = CircuitBuilder::<F, D>::new(config.clone());
            let mut pw = PartialWitness::new(config.num_wires);
            let pt = proof_to_proof_target(&proof_with_pis, &mut builder);
            set_proof_target(&proof_with_pis, &pt, &mut pw);

            let inner_data = VerifierCircuitTarget {
                constants_sigmas_cap: builder.add_virtual_cap(config.cap_height),
            };
            pw.set_cap_target(&inner_data.constants_sigmas_cap, &vd.constants_sigmas_cap);

            builder.add_recursive_verifier(pt, &config, &inner_data, &cd);

            let data = builder.build();
            let recursive_proof = data.prove(pw)?;
            (recursive_proof, data.verifier_only, data.common)
        };

        verify(proof_with_pis.clone(), &vd, &cd)?;
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let mut pw = PartialWitness::new(config.num_wires);
        let pt = proof_to_proof_target(&proof_with_pis, &mut builder);
        set_proof_target(&proof_with_pis, &pt, &mut pw);

        let inner_data = VerifierCircuitTarget {
            constants_sigmas_cap: builder.add_virtual_cap(config.cap_height),
        };
        pw.set_cap_target(&inner_data.constants_sigmas_cap, &vd.constants_sigmas_cap);

        builder.add_recursive_verifier(pt, &config, &inner_data, &cd);

        builder.print_gate_counts(0);
        let data = builder.build();
        let recursive_proof = data.prove(pw)?;
        let proof_bytes = serde_cbor::to_vec(&recursive_proof).unwrap();
        info!("Proof length: {} bytes", proof_bytes.len());
        verify(recursive_proof, &data.verifier_only, &data.common)
    }
}
