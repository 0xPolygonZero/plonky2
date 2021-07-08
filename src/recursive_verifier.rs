use crate::circuit_builder::CircuitBuilder;
use crate::circuit_data::{CircuitConfig, VerifierCircuitTarget};
use crate::field::extension_field::Extendable;
use crate::gates::gate::GateRef;
use crate::proof::ProofTarget;

const MIN_WIRES: usize = 120; // TODO: Double check.
const MIN_ROUTED_WIRES: usize = 28; // TODO: Double check.

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Recursively verifies an inner proof.
    pub fn add_recursive_verifier(
        &mut self,
        inner_config: CircuitConfig,
        inner_circuit: VerifierCircuitTarget,
        inner_gates: Vec<GateRef<F, D>>,
        inner_proof: ProofTarget<D>,
    ) {
        assert!(self.config.num_wires >= MIN_WIRES);
        assert!(self.config.num_wires >= MIN_ROUTED_WIRES);

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
    use crate::merkle_proofs::MerkleProofTarget;
    use crate::polynomial::commitment::OpeningProofTarget;
    use crate::proof::{
        FriInitialTreeProofTarget, FriProofTarget, FriQueryRoundTarget, FriQueryStepTarget,
        HashTarget, OpeningSetTarget, Proof,
    };
    use crate::witness::PartialWitness;

    fn proof_to_proof_target<F: Extendable<D>, const D: usize>(
        proof: &Proof<F, D>,
        builder: &mut CircuitBuilder<F, D>,
    ) -> ProofTarget<D> {
        let wires_root = builder.add_virtual_hash();
        let plonk_zs_root = builder.add_virtual_hash();
        let quotient_polys_root = builder.add_virtual_hash();

        let openings = OpeningSetTarget {
            constants: builder.add_virtual_extension_targets(proof.openings.constants.len()),
            plonk_sigmas: builder
                .add_virtual_extension_targets(proof.openings.plonk_s_sigmas.len()),
            wires: builder.add_virtual_extension_targets(proof.openings.wires.len()),
            plonk_zs: builder.add_virtual_extension_targets(proof.openings.plonk_zs.len()),
            plonk_zs_right: builder
                .add_virtual_extension_targets(proof.openings.plonk_zs_right.len()),
            partial_products: builder
                .add_virtual_extension_targets(proof.openings.partial_products.len()),
            quotient_polys: builder
                .add_virtual_extension_targets(proof.openings.quotient_polys.len()),
        };

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

        let opening_proof =
            OpeningProofTarget {
                fri_proof: FriProofTarget {
                    commit_phase_merkle_roots: vec![
                        builder.add_virtual_hash();
                        proof
                            .opening_proof
                            .fri_proof
                            .commit_phase_merkle_roots
                            .len()
                    ],
                    query_round_proofs: vec![
                        query_round.clone();
                        proof.opening_proof.fri_proof.query_round_proofs.len()
                    ],
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
            .zip(&proof.openings.plonk_s_sigmas)
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
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let proof = {
            let config = CircuitConfig::large_config();
            let mut builder = CircuitBuilder::<F, 4>::new(config);
            let zero = builder.zero();
            let data = builder.build();
            data.prove(PartialWitness::new())
        };

        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let mut pw = PartialWitness::new();
        let pt = proof_to_proof_target(&proof, &mut builder);
        set_proof_target(&proof, &pt, &mut pw);
    }
}
