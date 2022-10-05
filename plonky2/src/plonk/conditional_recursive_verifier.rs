use itertools::Itertools;
use plonky2_field::extension::Extendable;

use crate::fri::proof::{
    FriInitialTreeProofTarget, FriProofTarget, FriQueryRoundTarget, FriQueryStepTarget,
};
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::hash::hash_types::{HashOutTarget, MerkleCapTarget, RichField};
use crate::hash::merkle_proofs::MerkleProofTarget;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{CommonCircuitData, VerifierCircuitTarget};
use crate::plonk::config::{AlgebraicHasher, GenericConfig};
use crate::plonk::proof::{OpeningSetTarget, ProofTarget, ProofWithPublicInputsTarget};

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn conditionally_verify_proof<C: GenericConfig<D, F = F>>(
        &mut self,
        condition: BoolTarget,
        proof_with_pis: ProofWithPublicInputsTarget<D>,
        inner_verifier_data: &VerifierCircuitTarget,
        inner_common_data: &CommonCircuitData<F, C, D>,
    ) where
        C::Hasher: AlgebraicHasher<F>,
    {
        let dummy_proof = self.add_virtual_proof_with_pis(inner_common_data);
        let dummy_verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: self
                .add_virtual_cap(inner_common_data.config.fri_config.cap_height),
        };
        let ProofWithPublicInputsTarget {
            proof:
                ProofTarget {
                    wires_cap,
                    plonk_zs_partial_products_cap,
                    quotient_polys_cap,
                    openings,
                    opening_proof,
                },
            public_inputs,
        } = proof_with_pis;
        let ProofWithPublicInputsTarget {
            proof:
                ProofTarget {
                    wires_cap: dummy_wires_cap,
                    plonk_zs_partial_products_cap: dummy_plonk_zs_partial_products_cap,
                    quotient_polys_cap: dummy_quotient_polys_cap,
                    openings: dummy_openings,

                    opening_proof: dummy_opening_proof,
                },
            public_inputs: dummy_public_inputs,
        } = dummy_proof;

        let selected_wires_cap = self.select_cap(condition, wires_cap, dummy_wires_cap);
        let selected_plonk_zs_partial_products_cap = self.select_cap(
            condition,
            plonk_zs_partial_products_cap,
            dummy_plonk_zs_partial_products_cap,
        );
        let selected_quotient_polys_cap =
            self.select_cap(condition, quotient_polys_cap, dummy_quotient_polys_cap);
        let selected_openings = self.select_opening_set(condition, openings, dummy_openings);
        let selected_opening_proof =
            self.select_opening_proof(condition, opening_proof, dummy_opening_proof);
        let selected_public_inputs = self.select_vec(condition, public_inputs, dummy_public_inputs);
        let selected_proof = ProofWithPublicInputsTarget {
            proof: ProofTarget {
                wires_cap: selected_wires_cap,
                plonk_zs_partial_products_cap: selected_plonk_zs_partial_products_cap,
                quotient_polys_cap: selected_quotient_polys_cap,
                openings: selected_openings,
                opening_proof: selected_opening_proof,
            },
            public_inputs: selected_public_inputs,
        };
        let selected_verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: self.select_cap(
                condition,
                inner_verifier_data.constants_sigmas_cap.clone(),
                dummy_verifier_data.constants_sigmas_cap,
            ),
        };

        self.verify_proof(selected_proof, &selected_verifier_data, inner_common_data);
    }

    fn select_vec(&mut self, b: BoolTarget, v0: Vec<Target>, v1: Vec<Target>) -> Vec<Target> {
        v0.into_iter()
            .zip_eq(v1)
            .map(|(t0, t1)| self.select(b, t0, t1))
            .collect()
    }

    fn select_cap(
        &mut self,
        b: BoolTarget,
        cap0: MerkleCapTarget,
        cap1: MerkleCapTarget,
    ) -> MerkleCapTarget {
        assert_eq!(cap0.0.len(), cap1.0.len());
        MerkleCapTarget(
            cap0.0
                .into_iter()
                .zip_eq(cap1.0)
                .map(|(h0, h1)| HashOutTarget {
                    elements: std::array::from_fn(|i| {
                        self.select(b, h0.elements[i], h1.elements[i])
                    }),
                })
                .collect(),
        )
    }

    fn select_vec_cap(
        &mut self,
        b: BoolTarget,
        v0: Vec<MerkleCapTarget>,
        v1: Vec<MerkleCapTarget>,
    ) -> Vec<MerkleCapTarget> {
        v0.into_iter()
            .zip_eq(v1)
            .map(|(c0, c1)| self.select_cap(b, c0, c1))
            .collect()
    }

    fn select_opening_set(
        &mut self,
        b: BoolTarget,
        os0: OpeningSetTarget<D>,
        os1: OpeningSetTarget<D>,
    ) -> OpeningSetTarget<D> {
        OpeningSetTarget {
            constants: self.select_vec_ext(b, os0.constants, os1.constants),
            plonk_sigmas: self.select_vec_ext(b, os0.plonk_sigmas, os1.plonk_sigmas),
            wires: self.select_vec_ext(b, os0.wires, os1.wires),
            plonk_zs: self.select_vec_ext(b, os0.plonk_zs, os1.plonk_zs),
            plonk_zs_next: self.select_vec_ext(b, os0.plonk_zs_next, os1.plonk_zs_next),
            partial_products: self.select_vec_ext(b, os0.partial_products, os1.partial_products),
            quotient_polys: self.select_vec_ext(b, os0.quotient_polys, os1.quotient_polys),
        }
    }

    fn select_vec_ext(
        &mut self,
        b: BoolTarget,
        v0: Vec<ExtensionTarget<D>>,
        v1: Vec<ExtensionTarget<D>>,
    ) -> Vec<ExtensionTarget<D>> {
        v0.into_iter()
            .zip_eq(v1)
            .map(|(e0, e1)| self.select_ext(b, e0, e1))
            .collect()
    }

    fn select_opening_proof(
        &mut self,
        b: BoolTarget,
        proof0: FriProofTarget<D>,
        proof1: FriProofTarget<D>,
    ) -> FriProofTarget<D> {
        FriProofTarget {
            commit_phase_merkle_caps: self.select_vec_cap(
                b,
                proof0.commit_phase_merkle_caps,
                proof1.commit_phase_merkle_caps,
            ),
            query_round_proofs: self.select_vec_query_round(
                b,
                proof0.query_round_proofs,
                proof1.query_round_proofs,
            ),
            final_poly: PolynomialCoeffsExtTarget(self.select_vec_ext(
                b,
                proof0.final_poly.0,
                proof1.final_poly.0,
            )),
            pow_witness: self.select(b, proof0.pow_witness, proof1.pow_witness),
        }
    }

    fn select_query_round(
        &mut self,
        b: BoolTarget,
        qr0: FriQueryRoundTarget<D>,
        qr1: FriQueryRoundTarget<D>,
    ) -> FriQueryRoundTarget<D> {
        FriQueryRoundTarget {
            initial_trees_proof: self.select_initial_tree_proof(
                b,
                qr0.initial_trees_proof,
                qr1.initial_trees_proof,
            ),
            steps: self.select_vec_query_step(b, qr0.steps, qr1.steps),
        }
    }

    fn select_vec_query_round(
        &mut self,
        b: BoolTarget,
        v0: Vec<FriQueryRoundTarget<D>>,
        v1: Vec<FriQueryRoundTarget<D>>,
    ) -> Vec<FriQueryRoundTarget<D>> {
        v0.into_iter()
            .zip_eq(v1)
            .map(|(qr0, qr1)| self.select_query_round(b, qr0, qr1))
            .collect()
    }

    fn select_initial_tree_proof(
        &mut self,
        b: BoolTarget,
        proof0: FriInitialTreeProofTarget,
        proof1: FriInitialTreeProofTarget,
    ) -> FriInitialTreeProofTarget {
        FriInitialTreeProofTarget {
            evals_proofs: proof0
                .evals_proofs
                .into_iter()
                .zip_eq(proof1.evals_proofs)
                .map(|((v0, p0), (v1, p1))| {
                    (
                        self.select_vec(b, v0, v1),
                        self.select_merkle_proof(b, p0, p1),
                    )
                })
                .collect(),
        }
    }

    fn select_merkle_proof(
        &mut self,
        b: BoolTarget,
        proof0: MerkleProofTarget,
        proof1: MerkleProofTarget,
    ) -> MerkleProofTarget {
        MerkleProofTarget {
            siblings: proof0
                .siblings
                .into_iter()
                .zip_eq(proof1.siblings)
                .map(|(h0, h1)| HashOutTarget {
                    elements: std::array::from_fn(|i| {
                        self.select(b, h0.elements[i], h1.elements[i])
                    }),
                })
                .collect(),
        }
    }

    fn select_query_step(
        &mut self,
        b: BoolTarget,
        qs0: FriQueryStepTarget<D>,
        qs1: FriQueryStepTarget<D>,
    ) -> FriQueryStepTarget<D> {
        FriQueryStepTarget {
            evals: self.select_vec_ext(b, qs0.evals, qs1.evals),
            merkle_proof: self.select_merkle_proof(b, qs0.merkle_proof, qs1.merkle_proof),
        }
    }

    fn select_vec_query_step(
        &mut self,
        b: BoolTarget,
        v0: Vec<FriQueryStepTarget<D>>,
        v1: Vec<FriQueryStepTarget<D>>,
    ) -> Vec<FriQueryStepTarget<D>> {
        v0.into_iter()
            .zip_eq(v1)
            .map(|(qs0, qs1)| self.select_query_step(b, qs0, qs1))
            .collect()
    }
}
