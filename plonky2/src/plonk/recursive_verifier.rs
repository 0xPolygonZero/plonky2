use plonky2_field::extension_field::Extendable;

use crate::hash::hash_types::{HashOutTarget, RichField};
use crate::iop::challenger::RecursiveChallenger;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData, VerifierCircuitTarget};
use crate::plonk::config::{AlgebraicHasher, GenericConfig};
use crate::plonk::proof::{OpeningSetTarget, ProofTarget, ProofWithPublicInputsTarget};
use crate::plonk::vanishing_poly::eval_vanishing_poly_recursively;
use crate::plonk::vars::EvaluationTargets;
use crate::util::reducing::ReducingFactorTarget;
use crate::with_context;

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Recursively verifies an inner proof.
    pub fn verify_proof_with_pis<C: GenericConfig<D, F = F>>(
        &mut self,
        proof_with_pis: ProofWithPublicInputsTarget<D>,
        inner_config: &CircuitConfig,
        inner_verifier_data: &VerifierCircuitTarget,
        inner_common_data: &CommonCircuitData<F, C, D>,
    ) where
        C::Hasher: AlgebraicHasher<F>,
    {
        let ProofWithPublicInputsTarget {
            proof,
            public_inputs,
        } = proof_with_pis;

        assert_eq!(public_inputs.len(), inner_common_data.num_public_inputs);
        let public_inputs_hash = self.hash_n_to_hash::<C::InnerHasher>(public_inputs, true);

        self.verify_proof(
            proof,
            public_inputs_hash,
            inner_config,
            inner_verifier_data,
            inner_common_data,
        );
    }

    /// Recursively verifies an inner proof.
    pub fn verify_proof<C: GenericConfig<D, F = F>>(
        &mut self,
        proof: ProofTarget<D>,
        public_inputs_hash: HashOutTarget,
        inner_config: &CircuitConfig,
        inner_verifier_data: &VerifierCircuitTarget,
        inner_common_data: &CommonCircuitData<F, C, D>,
    ) where
        C::Hasher: AlgebraicHasher<F>,
    {
        let one = self.one_extension();

        let num_challenges = inner_config.num_challenges;

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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use itertools::Itertools;
    use log::{info, Level};

    use super::*;
    use crate::fri::reduction_strategies::FriReductionStrategy;
    use crate::fri::FriConfig;
    use crate::gates::noop::NoopGate;
    use crate::iop::witness::{PartialWitness, Witness};
    use crate::plonk::circuit_data::VerifierOnlyCircuitData;
    use crate::plonk::config::{
        GMiMCGoldilocksConfig, GenericConfig, KeccakGoldilocksConfig, PoseidonGoldilocksConfig,
    };
    use crate::plonk::proof::{CompressedProofWithPublicInputs, ProofWithPublicInputs};
    use crate::plonk::prover::prove;
    use crate::util::timing::TimingTree;

    // Set the targets in a `ProofTarget` to their corresponding values in a `Proof`.
    fn set_proof_target<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        proof: &ProofWithPublicInputs<F, C, D>,
        pt: &ProofWithPublicInputsTarget<D>,
        pw: &mut PartialWitness<F>,
    ) where
        C::Hasher: AlgebraicHasher<F>,
    {
        let ProofWithPublicInputs {
            proof,
            public_inputs,
        } = proof;
        let ProofWithPublicInputsTarget {
            proof: pt,
            public_inputs: pi_targets,
        } = pt;

        // Set public inputs.
        for (&pi_t, &pi) in pi_targets.iter().zip_eq(public_inputs) {
            pw.set_target(pi_t, pi);
        }

        pw.set_cap_target(&pt.wires_cap, &proof.wires_cap);
        pw.set_cap_target(
            &pt.plonk_zs_partial_products_cap,
            &proof.plonk_zs_partial_products_cap,
        );
        pw.set_cap_target(&pt.quotient_polys_cap, &proof.quotient_polys_cap);

        for (&t, &x) in pt.openings.wires.iter().zip_eq(&proof.openings.wires) {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt
            .openings
            .constants
            .iter()
            .zip_eq(&proof.openings.constants)
        {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt
            .openings
            .plonk_sigmas
            .iter()
            .zip_eq(&proof.openings.plonk_sigmas)
        {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt.openings.plonk_zs.iter().zip_eq(&proof.openings.plonk_zs) {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt
            .openings
            .plonk_zs_right
            .iter()
            .zip_eq(&proof.openings.plonk_zs_right)
        {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt
            .openings
            .partial_products
            .iter()
            .zip_eq(&proof.openings.partial_products)
        {
            pw.set_extension_target(t, x);
        }
        for (&t, &x) in pt
            .openings
            .quotient_polys
            .iter()
            .zip_eq(&proof.openings.quotient_polys)
        {
            pw.set_extension_target(t, x);
        }

        let fri_proof = &proof.opening_proof;
        let fpt = &pt.opening_proof;

        pw.set_target(fpt.pow_witness, fri_proof.pow_witness);

        for (&t, &x) in fpt.final_poly.0.iter().zip_eq(&fri_proof.final_poly.coeffs) {
            pw.set_extension_target(t, x);
        }

        for (t, x) in fpt
            .commit_phase_merkle_caps
            .iter()
            .zip_eq(&fri_proof.commit_phase_merkle_caps)
        {
            pw.set_cap_target(t, x);
        }

        for (qt, q) in fpt
            .query_round_proofs
            .iter()
            .zip_eq(&fri_proof.query_round_proofs)
        {
            for (at, a) in qt
                .initial_trees_proof
                .evals_proofs
                .iter()
                .zip_eq(&q.initial_trees_proof.evals_proofs)
            {
                for (&t, &x) in at.0.iter().zip_eq(&a.0) {
                    pw.set_target(t, x);
                }
                for (&t, &x) in at.1.siblings.iter().zip_eq(&a.1.siblings) {
                    pw.set_hash_target(t, x);
                }
            }

            for (st, s) in qt.steps.iter().zip_eq(&q.steps) {
                for (&t, &x) in st.evals.iter().zip_eq(&s.evals) {
                    pw.set_extension_target(t, x);
                }
                for (&t, &x) in st
                    .merkle_proof
                    .siblings
                    .iter()
                    .zip_eq(&s.merkle_proof.siblings)
                {
                    pw.set_hash_target(t, x);
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn test_recursive_verifier() -> Result<()> {
        init_logger();
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let config = CircuitConfig::standard_recursion_config();

        let (proof, vd, cd) = dummy_proof::<F, C, D>(&config, 4_000)?;
        let (proof, _vd, cd) =
            recursive_proof::<F, C, C, D>(proof, vd, cd, &config, &config, None, true, true)?;
        test_serialization(&proof, &cd)?;

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_recursive_recursive_verifier() -> Result<()> {
        init_logger();
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();

        // Start with a degree 2^14 proof
        let (proof, vd, cd) = dummy_proof::<F, C, D>(&config, 16_000)?;
        assert_eq!(cd.degree_bits, 14);

        // Shrink it to 2^13.
        let (proof, vd, cd) =
            recursive_proof::<F, C, C, D>(proof, vd, cd, &config, &config, Some(13), false, false)?;
        assert_eq!(cd.degree_bits, 13);

        // Shrink it to 2^12.
        let (proof, _vd, cd) =
            recursive_proof::<F, C, C, D>(proof, vd, cd, &config, &config, None, true, true)?;
        assert_eq!(cd.degree_bits, 12);

        test_serialization(&proof, &cd)?;

        Ok(())
    }

    /// Creates a chain of recursive proofs where the last proof is made as small as reasonably
    /// possible, using a high rate, high PoW bits, etc.
    #[test]
    #[ignore]
    fn test_size_optimized_recursion() -> Result<()> {
        init_logger();
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type KC = KeccakGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let standard_config = CircuitConfig::standard_recursion_config();

        // An initial dummy proof.
        let (proof, vd, cd) = dummy_proof::<F, C, D>(&standard_config, 4_000)?;
        assert_eq!(cd.degree_bits, 12);

        // A standard recursive proof.
        let (proof, vd, cd) = recursive_proof(
            proof,
            vd,
            cd,
            &standard_config,
            &standard_config,
            None,
            false,
            false,
        )?;
        assert_eq!(cd.degree_bits, 12);

        // A high-rate recursive proof, designed to be verifiable with fewer routed wires.
        let high_rate_config = CircuitConfig {
            fri_config: FriConfig {
                rate_bits: 7,
                proof_of_work_bits: 16,
                num_query_rounds: 12,
                ..standard_config.fri_config.clone()
            },
            ..standard_config
        };
        let (proof, vd, cd) = recursive_proof::<F, C, C, D>(
            proof,
            vd,
            cd,
            &standard_config,
            &high_rate_config,
            None,
            true,
            true,
        )?;
        assert_eq!(cd.degree_bits, 12);

        // A final proof, optimized for size.
        let final_config = CircuitConfig {
            num_routed_wires: 37,
            fri_config: FriConfig {
                rate_bits: 8,
                cap_height: 0,
                proof_of_work_bits: 20,
                reduction_strategy: FriReductionStrategy::MinSize(None),
                num_query_rounds: 10,
            },
            ..high_rate_config
        };
        let (proof, _vd, cd) = recursive_proof::<F, KC, C, D>(
            proof,
            vd,
            cd,
            &high_rate_config,
            &final_config,
            None,
            true,
            true,
        )?;
        assert_eq!(cd.degree_bits, 12, "final proof too large");

        test_serialization(&proof, &cd)?;

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_recursive_verifier_multi_hash() -> Result<()> {
        init_logger();
        const D: usize = 2;
        type PC = PoseidonGoldilocksConfig;
        type GC = GMiMCGoldilocksConfig;
        type KC = KeccakGoldilocksConfig;
        type F = <PC as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();
        let (proof, vd, cd) = dummy_proof::<F, PC, D>(&config, 4_000)?;

        let (proof, vd, cd) =
            recursive_proof::<F, PC, PC, D>(proof, vd, cd, &config, &config, None, false, false)?;
        test_serialization(&proof, &cd)?;

        let (proof, vd, cd) =
            recursive_proof::<F, GC, PC, D>(proof, vd, cd, &config, &config, None, false, false)?;
        test_serialization(&proof, &cd)?;

        let (proof, vd, cd) =
            recursive_proof::<F, GC, GC, D>(proof, vd, cd, &config, &config, None, false, false)?;
        test_serialization(&proof, &cd)?;

        let (proof, _vd, cd) =
            recursive_proof::<F, KC, GC, D>(proof, vd, cd, &config, &config, None, false, false)?;
        test_serialization(&proof, &cd)?;

        Ok(())
    }

    /// Creates a dummy proof which should have roughly `num_dummy_gates` gates.
    fn dummy_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        config: &CircuitConfig,
        num_dummy_gates: u64,
    ) -> Result<(
        ProofWithPublicInputs<F, C, D>,
        VerifierOnlyCircuitData<C, D>,
        CommonCircuitData<F, C, D>,
    )> {
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        for _ in 0..num_dummy_gates {
            builder.add_gate(NoopGate, vec![]);
        }

        let data = builder.build::<C>();
        let inputs = PartialWitness::new();
        let proof = data.prove(inputs)?;
        data.verify(proof.clone())?;

        Ok((proof, data.verifier_only, data.common))
    }

    fn recursive_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        InnerC: GenericConfig<D, F = F>,
        const D: usize,
    >(
        inner_proof: ProofWithPublicInputs<F, InnerC, D>,
        inner_vd: VerifierOnlyCircuitData<InnerC, D>,
        inner_cd: CommonCircuitData<F, InnerC, D>,
        inner_config: &CircuitConfig,
        config: &CircuitConfig,
        min_degree_bits: Option<usize>,
        print_gate_counts: bool,
        print_timing: bool,
    ) -> Result<(
        ProofWithPublicInputs<F, C, D>,
        VerifierOnlyCircuitData<C, D>,
        CommonCircuitData<F, C, D>,
    )>
    where
        InnerC::Hasher: AlgebraicHasher<F>,
    {
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let mut pw = PartialWitness::new();
        let pt = builder.add_virtual_proof_with_pis(&inner_cd);
        set_proof_target(&inner_proof, &pt, &mut pw);

        let inner_data = VerifierCircuitTarget {
            constants_sigmas_cap: builder.add_virtual_cap(inner_config.fri_config.cap_height),
        };
        pw.set_cap_target(
            &inner_data.constants_sigmas_cap,
            &inner_vd.constants_sigmas_cap,
        );

        builder.verify_proof_with_pis(pt, inner_config, &inner_data, &inner_cd);

        if print_gate_counts {
            builder.print_gate_counts(0);
        }

        if let Some(min_degree_bits) = min_degree_bits {
            // We don't want to pad all the way up to 2^min_degree_bits, as the builder will add a
            // few special gates afterward. So just pad to 2^(min_degree_bits - 1) + 1. Then the
            // builder will pad to the next power of two, 2^min_degree_bits.
            let min_gates = (1 << (min_degree_bits - 1)) + 1;
            for _ in builder.num_gates()..min_gates {
                builder.add_gate(NoopGate, vec![]);
            }
        }

        let data = builder.build::<C>();

        let mut timing = TimingTree::new("prove", Level::Debug);
        let proof = prove(&data.prover_only, &data.common, pw, &mut timing)?;
        if print_timing {
            timing.print();
        }

        data.verify(proof.clone())?;

        Ok((proof, data.verifier_only, data.common))
    }

    /// Test serialization and print some size info.
    fn test_serialization<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        proof: &ProofWithPublicInputs<F, C, D>,
        cd: &CommonCircuitData<F, C, D>,
    ) -> Result<()> {
        let proof_bytes = proof.to_bytes()?;
        info!("Proof length: {} bytes", proof_bytes.len());
        let proof_from_bytes = ProofWithPublicInputs::from_bytes(proof_bytes, cd)?;
        assert_eq!(proof, &proof_from_bytes);

        let now = std::time::Instant::now();
        let compressed_proof = proof.clone().compress(cd)?;
        let decompressed_compressed_proof = compressed_proof.clone().decompress(cd)?;
        info!("{:.4}s to compress proof", now.elapsed().as_secs_f64());
        assert_eq!(proof, &decompressed_compressed_proof);

        let compressed_proof_bytes = compressed_proof.to_bytes()?;
        info!(
            "Compressed proof length: {} bytes",
            compressed_proof_bytes.len()
        );
        let compressed_proof_from_bytes =
            CompressedProofWithPublicInputs::from_bytes(compressed_proof_bytes, cd)?;
        assert_eq!(compressed_proof, compressed_proof_from_bytes);

        Ok(())
    }

    fn init_logger() {
        let _ = env_logger::builder().format_timestamp(None).try_init();
    }
}
