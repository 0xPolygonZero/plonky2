use itertools::izip;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{flatten, unflatten, Extendable};
use crate::field::field_types::{Field, RichField};
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::hash::hash_types::MerkleCapTarget;
use crate::hash::merkle_proofs::{MerkleProof, MerkleProofTarget};
use crate::hash::merkle_tree::MerkleCap;
use crate::hash::path_compression::{compress_merkle_proofs, decompress_merkle_proofs};
use crate::iop::target::Target;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::plonk_common::PolynomialsIndexBlinding;
use crate::polynomial::polynomial::PolynomialCoeffs;

/// Evaluations and Merkle proof produced by the prover in a FRI query step.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct FriQueryStep<F: Extendable<D>, const D: usize> {
    pub evals: Vec<F::Extension>,
    pub merkle_proof: MerkleProof<F>,
}

#[derive(Clone)]
pub struct FriQueryStepTarget<const D: usize> {
    pub evals: Vec<ExtensionTarget<D>>,
    pub merkle_proof: MerkleProofTarget,
}

/// Evaluations and Merkle proofs of the original set of polynomials,
/// before they are combined into a composition polynomial.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct FriInitialTreeProof<F: Field> {
    pub evals_proofs: Vec<(Vec<F>, MerkleProof<F>)>,
}

impl<F: Field> FriInitialTreeProof<F> {
    pub(crate) fn unsalted_evals(
        &self,
        polynomials: PolynomialsIndexBlinding,
        zero_knowledge: bool,
    ) -> &[F] {
        let evals = &self.evals_proofs[polynomials.index].0;
        &evals[..evals.len() - polynomials.salt_size(zero_knowledge)]
    }
}

#[derive(Clone)]
pub struct FriInitialTreeProofTarget {
    pub evals_proofs: Vec<(Vec<Target>, MerkleProofTarget)>,
}

impl FriInitialTreeProofTarget {
    pub(crate) fn unsalted_evals(
        &self,
        polynomials: PolynomialsIndexBlinding,
        zero_knowledge: bool,
    ) -> &[Target] {
        let evals = &self.evals_proofs[polynomials.index].0;
        &evals[..evals.len() - polynomials.salt_size(zero_knowledge)]
    }
}

/// Proof for a FRI query round.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct FriQueryRound<F: Extendable<D>, const D: usize> {
    pub index: usize,
    pub initial_trees_proof: FriInitialTreeProof<F>,
    pub steps: Vec<FriQueryStep<F, D>>,
}

#[derive(Clone)]
pub struct FriQueryRoundTarget<const D: usize> {
    pub initial_trees_proof: FriInitialTreeProofTarget,
    pub steps: Vec<FriQueryStepTarget<D>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct FriProof<F: Extendable<D>, const D: usize> {
    /// A Merkle cap for each reduced polynomial in the commit phase.
    pub commit_phase_merkle_caps: Vec<MerkleCap<F>>,
    /// Query rounds proofs
    pub query_round_proofs: Vec<FriQueryRound<F, D>>,
    /// The final polynomial in coefficient form.
    pub final_poly: PolynomialCoeffs<F::Extension>,
    /// Witness showing that the prover did PoW.
    pub pow_witness: F,
    /// Flag set to true if path compression has been applied to the proof's Merkle proofs.
    pub is_compressed: bool,
}

pub struct FriProofTarget<const D: usize> {
    pub commit_phase_merkle_caps: Vec<MerkleCapTarget>,
    pub query_round_proofs: Vec<FriQueryRoundTarget<D>>,
    pub final_poly: PolynomialCoeffsExtTarget<D>,
    pub pow_witness: Target,
}

impl<F: RichField + Extendable<D>, const D: usize> FriProof<F, D> {
    /// Compress all the Merkle paths in the FRI proof.
    pub fn compress(self, common_data: &CommonCircuitData<F, D>) -> Self {
        if self.is_compressed {
            panic!("Proof is already compressed.");
        }
        let FriProof {
            commit_phase_merkle_caps,
            mut query_round_proofs,
            final_poly,
            pow_witness,
            ..
        } = self;
        let cap_height = common_data.config.cap_height;
        let reduction_arity_bits = &common_data.config.fri_config.reduction_arity_bits;
        let num_reductions = reduction_arity_bits.len();
        let num_initial_trees = query_round_proofs[0].initial_trees_proof.evals_proofs.len();

        // "Transpose" the query round proofs, so that information for each Merkle tree is collected together.
        let mut initial_trees_indices = vec![vec![]; num_initial_trees];
        let mut initial_trees_leaves = vec![vec![]; num_initial_trees];
        let mut initial_trees_proofs = vec![vec![]; num_initial_trees];
        let mut steps_indices = vec![vec![]; num_reductions];
        let mut steps_evals = vec![vec![]; num_reductions];
        let mut steps_proofs = vec![vec![]; num_reductions];

        for qrp in &query_round_proofs {
            let FriQueryRound {
                mut index,
                initial_trees_proof,
                steps,
            } = qrp.clone();
            for (i, (leaves_data, proof)) in
                initial_trees_proof.evals_proofs.into_iter().enumerate()
            {
                initial_trees_indices[i].push(index);
                initial_trees_leaves[i].push(leaves_data);
                initial_trees_proofs[i].push(proof);
            }
            for (i, query_step) in steps.into_iter().enumerate() {
                index >>= reduction_arity_bits[i];
                steps_indices[i].push(index);
                steps_evals[i].push(query_step.evals);
                steps_proofs[i].push(query_step.merkle_proof);
            }
        }

        // Compress all Merkle proofs.
        let initial_trees_proofs = initial_trees_indices
            .iter()
            .zip(initial_trees_proofs)
            .map(|(is, ps)| compress_merkle_proofs(cap_height, is, &ps))
            .collect::<Vec<_>>();
        let steps_proofs = steps_indices
            .iter()
            .zip(steps_proofs)
            .map(|(is, ps)| compress_merkle_proofs(cap_height, is, &ps))
            .collect::<Vec<_>>();

        // Replace the query round proofs with the compressed versions.
        for (i, qrp) in query_round_proofs.iter_mut().enumerate() {
            qrp.initial_trees_proof = FriInitialTreeProof {
                evals_proofs: (0..num_initial_trees)
                    .map(|j| {
                        (
                            initial_trees_leaves[j][i].clone(),
                            initial_trees_proofs[j][i].clone(),
                        )
                    })
                    .collect(),
            };
            qrp.steps = (0..num_reductions)
                .map(|j| FriQueryStep {
                    evals: steps_evals[j][i].clone(),
                    merkle_proof: steps_proofs[j][i].clone(),
                })
                .collect();
        }

        FriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
            is_compressed: true,
        }
    }

    /// Decompress all the Merkle paths in the FRI proof.
    pub fn decompress(self, common_data: &CommonCircuitData<F, D>) -> Self {
        if !self.is_compressed {
            panic!("Proof is not compressed.");
        }
        let FriProof {
            commit_phase_merkle_caps,
            mut query_round_proofs,
            final_poly,
            pow_witness,
            ..
        } = self;
        let cap_height = common_data.config.cap_height;
        let reduction_arity_bits = &common_data.config.fri_config.reduction_arity_bits;
        let num_reductions = reduction_arity_bits.len();
        let num_initial_trees = query_round_proofs[0].initial_trees_proof.evals_proofs.len();

        // "Transpose" the query round proofs, so that information for each Merkle tree is collected together.
        let mut initial_trees_indices = vec![vec![]; num_initial_trees];
        let mut initial_trees_leaves = vec![vec![]; num_initial_trees];
        let mut initial_trees_proofs = vec![vec![]; num_initial_trees];
        let mut steps_indices = vec![vec![]; num_reductions];
        let mut steps_evals = vec![vec![]; num_reductions];
        let mut steps_proofs = vec![vec![]; num_reductions];
        let height = common_data.degree_bits + common_data.config.rate_bits;
        let heights = reduction_arity_bits
            .iter()
            .scan(height, |acc, &bits| {
                *acc -= bits;
                Some(*acc)
            })
            .collect::<Vec<_>>();

        for qrp in &query_round_proofs {
            let FriQueryRound {
                mut index,
                initial_trees_proof,
                steps,
            } = qrp.clone();
            for (i, (leaves_data, proof)) in
                initial_trees_proof.evals_proofs.into_iter().enumerate()
            {
                initial_trees_indices[i].push(index);
                initial_trees_leaves[i].push(leaves_data);
                initial_trees_proofs[i].push(proof);
            }
            for (i, query_step) in steps.into_iter().enumerate() {
                index >>= reduction_arity_bits[i];
                steps_indices[i].push(index);
                steps_evals[i].push(flatten(&query_step.evals));
                steps_proofs[i].push(query_step.merkle_proof);
            }
        }

        // Decompress all Merkle proofs.
        let initial_trees_proofs = izip!(
            &initial_trees_leaves,
            &initial_trees_indices,
            initial_trees_proofs
        )
        .map(|(ls, is, ps)| decompress_merkle_proofs(&ls, is, &ps, height, cap_height))
        .collect::<Vec<_>>();
        let steps_proofs = izip!(&steps_evals, &steps_indices, steps_proofs, heights)
            .map(|(ls, is, ps, h)| decompress_merkle_proofs(ls, is, &ps, h, cap_height))
            .collect::<Vec<_>>();

        // Replace the query round proofs with the decompressed versions.
        for (i, qrp) in query_round_proofs.iter_mut().enumerate() {
            qrp.initial_trees_proof = FriInitialTreeProof {
                evals_proofs: (0..num_initial_trees)
                    .map(|j| {
                        (
                            initial_trees_leaves[j][i].clone(),
                            initial_trees_proofs[j][i].clone(),
                        )
                    })
                    .collect(),
            };
            qrp.steps = (0..num_reductions)
                .map(|j| FriQueryStep {
                    evals: unflatten(&steps_evals[j][i]),
                    merkle_proof: steps_proofs[j][i].clone(),
                })
                .collect();
        }

        FriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
            is_compressed: false,
        }
    }
}
