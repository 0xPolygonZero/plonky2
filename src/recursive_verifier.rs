use std::sync::Arc;

use env_logger::builder;

use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::{CircuitConfig, CommonCircuitData, VerifierCircuitTarget};
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gates::gate::{GateRef, PrefixedGate};
use crate::plonk_challenger::RecursiveChallenger;
use crate::proof::{HashTarget, ProofTarget};
use crate::util::marking::MarkedTargets;
use crate::util::scaling::ReducingFactorTarget;
use crate::vanishing_poly::eval_vanishing_poly_recursively;
use crate::vars::EvaluationTargets;

const MIN_WIRES: usize = 120; // TODO: Double check.
const MIN_ROUTED_WIRES: usize = 28; // TODO: Double check.

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Recursively verifies an inner proof.
    pub fn add_recursive_verifier(
        &mut self,
        proof: ProofTarget<D>,
        inner_config: &CircuitConfig,
        inner_verifier_data: &VerifierCircuitTarget,
        inner_common_data: &CommonCircuitData<F, D>,
        marked: &mut Vec<MarkedTargets>,
    ) {
        assert!(self.config.num_wires >= MIN_WIRES);
        assert!(self.config.num_wires >= MIN_ROUTED_WIRES);
        let one = self.one_extension();

        let num_challenges = inner_config.num_challenges;

        let mut challenger = RecursiveChallenger::new(self);

        self.set_context("Challenger observes proof and generates challenges.");
        let digest =
            HashTarget::from_vec(self.constants(&inner_common_data.circuit_digest.elements));
        challenger.observe_hash(&digest);

        challenger.observe_hash(&proof.wires_root);
        let betas = challenger.get_n_challenges(self, num_challenges);
        let gammas = challenger.get_n_challenges(self, num_challenges);

        challenger.observe_hash(&proof.plonk_zs_root);
        let alphas = challenger.get_n_challenges(self, num_challenges);

        challenger.observe_hash(&proof.quotient_polys_root);
        let zeta = challenger.get_extension_challenge(self);

        let local_constants = &proof.openings.constants;
        let local_wires = &proof.openings.wires;
        let vars = EvaluationTargets {
            local_constants,
            local_wires,
        };
        let local_zs = &proof.openings.plonk_zs;
        let next_zs = &proof.openings.plonk_zs_right;
        let s_sigmas = &proof.openings.plonk_sigmas;
        let partial_products = &proof.openings.partial_products;

        let zeta_pow_deg = self.exp_u64_extension(zeta, inner_common_data.degree() as u64);
        self.set_context("Evaluate the vanishing polynomial at our challenge point, zeta.");
        let vanishing_polys_zeta = eval_vanishing_poly_recursively(
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
            marked,
        );

        marked.push(MarkedTargets {
            name: "vanishing polys".into(),
            targets: Arc::new(vanishing_polys_zeta[0].clone()),
        });

        self.set_context("Check vanishing and quotient polynomials.");
        let quotient_polys_zeta = &proof.openings.quotient_polys;
        let zeta_pow_deg = self.exp_u64_extension(zeta, 1 << inner_common_data.degree_bits as u64);
        let z_h_zeta = self.sub_extension(zeta_pow_deg, one);
        for (i, chunk) in quotient_polys_zeta
            .chunks(inner_common_data.quotient_degree_factor)
            .enumerate()
        {
            let mut scale = ReducingFactorTarget::new(zeta_pow_deg);
            let mut rhs = scale.reduce(chunk, self);
            rhs = self.mul_extension(z_h_zeta, rhs);
            self.named_route_extension(
                vanishing_polys_zeta[i],
                rhs,
                format!("Vanishing polynomial == Z_H * quotient, challenge {}", i),
            );
        }

        let evaluations = proof.openings.clone();

        let merkle_roots = &[
            inner_verifier_data.constants_sigmas_root,
            proof.wires_root,
            proof.plonk_zs_root,
            proof.quotient_polys_root,
        ];

        proof.opening_proof.verify(
            zeta,
            &evaluations,
            merkle_roots,
            &mut challenger,
            inner_common_data,
            self,
        );
        dbg!(self.num_gates());
        dbg!(self.generators.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::extension_field::target::ExtensionTarget;
    use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
    use crate::merkle_proofs::MerkleProofTarget;
    use crate::polynomial::commitment::OpeningProofTarget;
    use crate::proof::{
        FriInitialTreeProofTarget, FriProofTarget, FriQueryRoundTarget, FriQueryStepTarget,
        HashTarget, OpeningSetTarget, Proof,
    };
    use crate::target::Target;
    use crate::verifier::verify;
    use crate::witness::PartialWitness;

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
        for (v, merkle_proof) in &proof.opening_proof.fri_proof.query_round_proofs[0]
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
        for step in &proof.opening_proof.fri_proof.query_round_proofs[0].steps {
            query_round.steps.push(FriQueryStepTarget {
                evals: builder.add_virtual_extension_targets(step.evals.len()),
                merkle_proof: MerkleProofTarget {
                    siblings: builder.add_virtual_hashes(step.merkle_proof.siblings.len()),
                },
            });
        }
        query_round
    }

    fn proof_to_proof_target<F: Extendable<D>, const D: usize>(
        proof: &Proof<F, D>,
        builder: &mut CircuitBuilder<F, D>,
    ) -> ProofTarget<D> {
        let wires_root = builder.add_virtual_hash();
        let plonk_zs_root = builder.add_virtual_hash();
        let quotient_polys_root = builder.add_virtual_hash();

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
        let query_round_proofs = (0..proof.opening_proof.fri_proof.query_round_proofs.len())
            .map(|_| get_fri_query_round(proof, builder))
            .collect();
        let commit_phase_merkle_roots = (0..proof
            .opening_proof
            .fri_proof
            .commit_phase_merkle_roots
            .len())
            .map(|_| builder.add_virtual_hash())
            .collect();
        let opening_proof =
            OpeningProofTarget {
                fri_proof: FriProofTarget {
                    commit_phase_merkle_roots,
                    query_round_proofs,
                    final_poly: PolynomialCoeffsExtTarget(builder.add_virtual_extension_targets(
                        proof.opening_proof.fri_proof.final_poly.len(),
                    )),
                    pow_witness: builder.add_virtual_target(),
                },
            };

        ProofTarget {
            wires_root,
            plonk_zs_root,
            quotient_polys_root,
            openings,
            opening_proof,
        }
    }

    fn set_proof_target<F: Extendable<D>, const D: usize>(
        proof: &Proof<F, D>,
        pt: &ProofTarget<D>,
        pw: &mut PartialWitness<F>,
    ) {
        pw.set_hash_target(pt.wires_root, proof.wires_root);
        pw.set_hash_target(pt.plonk_zs_root, proof.plonk_zs_root);
        pw.set_hash_target(pt.quotient_polys_root, proof.quotient_polys_root);

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

        let fri_proof = &proof.opening_proof.fri_proof;
        let fpt = &pt.opening_proof.fri_proof;

        pw.set_target(fpt.pow_witness, fri_proof.pow_witness);

        for (&t, &x) in fpt.final_poly.0.iter().zip(&fri_proof.final_poly.coeffs) {
            pw.set_extension_target(t, x);
        }

        for (&t, &x) in fpt
            .commit_phase_merkle_roots
            .iter()
            .zip(&fri_proof.commit_phase_merkle_roots)
        {
            pw.set_hash_target(t, x);
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
    fn test_recursive_verifier() {
        env_logger::init();
        type F = CrandallField;
        type FF = QuarticCrandallField;
        const D: usize = 4;
        let (proof, vd, cd) = {
            let config = CircuitConfig::large_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);
            let zero = builder.zero();
            let hash = builder.hash_n_to_m(vec![zero], 2, true);
            let z = builder.mul(hash[0], hash[1]);
            let data = builder.build();
            (
                data.prove(PartialWitness::new()),
                data.verifier_only,
                data.common,
            )
        };
        verify(proof.clone(), &vd, &cd).unwrap();

        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let mut pw = PartialWitness::new();
        let mut marked = Vec::new();
        let pt = proof_to_proof_target(&proof, &mut builder);
        set_proof_target(&proof, &pt, &mut pw);

        let inner_data = VerifierCircuitTarget {
            constants_sigmas_root: builder.add_virtual_hash(),
        };
        pw.set_hash_target(inner_data.constants_sigmas_root, vd.constants_sigmas_root);

        builder.add_recursive_verifier(pt, &config, &inner_data, &cd, &mut marked);

        dbg!(builder.num_gates());
        dbg!(builder.marked_targets.len());
        let data = builder.build();
        let recursive_proof = data.prove(pw);

        verify(recursive_proof, &data.verifier_only, &data.common).unwrap();
    }
}
