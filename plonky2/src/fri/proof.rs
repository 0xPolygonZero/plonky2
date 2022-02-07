use std::collections::HashMap;

use itertools::izip;
use plonky2_field::extension_field::{flatten, unflatten, Extendable};
use plonky2_field::polynomial::PolynomialCoeffs;
use serde::{Deserialize, Serialize};

use crate::fri::FriParams;
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::hash::hash_types::MerkleCapTarget;
use crate::hash::hash_types::RichField;
use crate::hash::merkle_proofs::{MerkleProof, MerkleProofTarget};
use crate::hash::merkle_tree::MerkleCap;
use crate::hash::path_compression::{compress_merkle_proofs, decompress_merkle_proofs};
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::target::Target;
use crate::plonk::config::{GenericConfig, Hasher};
use crate::plonk::plonk_common::salt_size;
use crate::plonk::proof::{FriInferredElements, ProofChallenges};

/// Evaluations and Merkle proof produced by the prover in a FRI query step.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct FriQueryStep<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> {
    pub evals: Vec<F::Extension>,
    pub merkle_proof: MerkleProof<F, H>,
}

#[derive(Clone, Debug)]
pub struct FriQueryStepTarget<const D: usize> {
    pub evals: Vec<ExtensionTarget<D>>,
    pub merkle_proof: MerkleProofTarget,
}

/// Evaluations and Merkle proofs of the original set of polynomials,
/// before they are combined into a composition polynomial.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct FriInitialTreeProof<F: RichField, H: Hasher<F>> {
    pub evals_proofs: Vec<(Vec<F>, MerkleProof<F, H>)>,
}

impl<F: RichField, H: Hasher<F>> FriInitialTreeProof<F, H> {
    pub(crate) fn unsalted_eval(&self, oracle_index: usize, poly_index: usize, salted: bool) -> F {
        self.unsalted_evals(oracle_index, salted)[poly_index]
    }

    fn unsalted_evals(&self, oracle_index: usize, salted: bool) -> &[F] {
        let evals = &self.evals_proofs[oracle_index].0;
        &evals[..evals.len() - salt_size(salted)]
    }
}

#[derive(Clone, Debug)]
pub struct FriInitialTreeProofTarget {
    pub evals_proofs: Vec<(Vec<Target>, MerkleProofTarget)>,
}

impl FriInitialTreeProofTarget {
    pub(crate) fn unsalted_eval(
        &self,
        oracle_index: usize,
        poly_index: usize,
        salted: bool,
    ) -> Target {
        self.unsalted_evals(oracle_index, salted)[poly_index]
    }

    fn unsalted_evals(&self, oracle_index: usize, salted: bool) -> &[Target] {
        let evals = &self.evals_proofs[oracle_index].0;
        &evals[..evals.len() - salt_size(salted)]
    }
}

/// Proof for a FRI query round.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct FriQueryRound<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> {
    pub initial_trees_proof: FriInitialTreeProof<F, H>,
    pub steps: Vec<FriQueryStep<F, H, D>>,
}

#[derive(Clone, Debug)]
pub struct FriQueryRoundTarget<const D: usize> {
    pub initial_trees_proof: FriInitialTreeProofTarget,
    pub steps: Vec<FriQueryStepTarget<D>>,
}

/// Compressed proof of the FRI query rounds.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct CompressedFriQueryRounds<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> {
    /// Query indices.
    pub indices: Vec<usize>,
    /// Map from initial indices `i` to the `FriInitialProof` for the `i`th leaf.
    pub initial_trees_proofs: HashMap<usize, FriInitialTreeProof<F, H>>,
    /// For each FRI query step, a map from indices `i` to the `FriQueryStep` for the `i`th leaf.
    pub steps: Vec<HashMap<usize, FriQueryStep<F, H, D>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct FriProof<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> {
    /// A Merkle cap for each reduced polynomial in the commit phase.
    pub commit_phase_merkle_caps: Vec<MerkleCap<F, H>>,
    /// Query rounds proofs
    pub query_round_proofs: Vec<FriQueryRound<F, H, D>>,
    /// The final polynomial in coefficient form.
    pub final_poly: PolynomialCoeffs<F::Extension>,
    /// Witness showing that the prover did PoW.
    pub pow_witness: F,
}

#[derive(Debug)]
pub struct FriProofTarget<const D: usize> {
    pub commit_phase_merkle_caps: Vec<MerkleCapTarget>,
    pub query_round_proofs: Vec<FriQueryRoundTarget<D>>,
    pub final_poly: PolynomialCoeffsExtTarget<D>,
    pub pow_witness: Target,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct CompressedFriProof<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> {
    /// A Merkle cap for each reduced polynomial in the commit phase.
    pub commit_phase_merkle_caps: Vec<MerkleCap<F, H>>,
    /// Compressed query rounds proof.
    pub query_round_proofs: CompressedFriQueryRounds<F, H, D>,
    /// The final polynomial in coefficient form.
    pub final_poly: PolynomialCoeffs<F::Extension>,
    /// Witness showing that the prover did PoW.
    pub pow_witness: F,
}

impl<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> FriProof<F, H, D> {
    /// Compress all the Merkle paths in the FRI proof and remove duplicate indices.
    pub fn compress<C: GenericConfig<D, F = F, Hasher = H>>(
        self,
        indices: &[usize],
        params: &FriParams,
    ) -> CompressedFriProof<F, H, D> {
        let FriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
            ..
        } = self;
        let cap_height = params.config.cap_height;
        let reduction_arity_bits = &params.reduction_arity_bits;
        let num_reductions = reduction_arity_bits.len();
        let num_initial_trees = query_round_proofs[0].initial_trees_proof.evals_proofs.len();

        // "Transpose" the query round proofs, so that information for each Merkle tree is collected together.
        let mut initial_trees_indices = vec![vec![]; num_initial_trees];
        let mut initial_trees_leaves = vec![vec![]; num_initial_trees];
        let mut initial_trees_proofs = vec![vec![]; num_initial_trees];
        let mut steps_indices = vec![vec![]; num_reductions];
        let mut steps_evals = vec![vec![]; num_reductions];
        let mut steps_proofs = vec![vec![]; num_reductions];

        for (mut index, qrp) in indices.iter().cloned().zip(&query_round_proofs) {
            let FriQueryRound {
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
                let index_within_coset = index & ((1 << reduction_arity_bits[i]) - 1);
                index >>= reduction_arity_bits[i];
                steps_indices[i].push(index);
                let mut evals = query_step.evals;
                // Remove the element that can be inferred.
                evals.remove(index_within_coset);
                steps_evals[i].push(evals);
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

        let mut compressed_query_proofs = CompressedFriQueryRounds {
            indices: indices.to_vec(),
            initial_trees_proofs: HashMap::new(),
            steps: vec![HashMap::new(); num_reductions],
        };

        // Replace the query round proofs with the compressed versions.
        for (i, mut index) in indices.iter().copied().enumerate() {
            let initial_proof = FriInitialTreeProof {
                evals_proofs: (0..num_initial_trees)
                    .map(|j| {
                        (
                            initial_trees_leaves[j][i].clone(),
                            initial_trees_proofs[j][i].clone(),
                        )
                    })
                    .collect(),
            };
            compressed_query_proofs
                .initial_trees_proofs
                .entry(index)
                .or_insert(initial_proof);
            for j in 0..num_reductions {
                index >>= reduction_arity_bits[j];
                let query_step = FriQueryStep {
                    evals: steps_evals[j][i].clone(),
                    merkle_proof: steps_proofs[j][i].clone(),
                };
                compressed_query_proofs.steps[j]
                    .entry(index)
                    .or_insert(query_step);
            }
        }

        CompressedFriProof {
            commit_phase_merkle_caps,
            query_round_proofs: compressed_query_proofs,
            final_poly,
            pow_witness,
        }
    }
}

impl<F: RichField + Extendable<D>, H: Hasher<F>, const D: usize> CompressedFriProof<F, H, D> {
    /// Decompress all the Merkle paths in the FRI proof and reinsert duplicate indices.
    pub(crate) fn decompress<C: GenericConfig<D, F = F, Hasher = H>>(
        self,
        challenges: &ProofChallenges<F, D>,
        fri_inferred_elements: FriInferredElements<F, D>,
        params: &FriParams,
    ) -> FriProof<F, H, D> {
        let CompressedFriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
            ..
        } = self;
        let FriChallenges {
            fri_query_indices: indices,
            ..
        } = &challenges.fri_challenges;
        let mut fri_inferred_elements = fri_inferred_elements.0.into_iter();
        let cap_height = params.config.cap_height;
        let reduction_arity_bits = &params.reduction_arity_bits;
        let num_reductions = reduction_arity_bits.len();
        let num_initial_trees = query_round_proofs
            .initial_trees_proofs
            .values()
            .next()
            .unwrap()
            .evals_proofs
            .len();

        // "Transpose" the query round proofs, so that information for each Merkle tree is collected together.
        let mut initial_trees_indices = vec![vec![]; num_initial_trees];
        let mut initial_trees_leaves = vec![vec![]; num_initial_trees];
        let mut initial_trees_proofs = vec![vec![]; num_initial_trees];
        let mut steps_indices = vec![vec![]; num_reductions];
        let mut steps_evals = vec![vec![]; num_reductions];
        let mut steps_proofs = vec![vec![]; num_reductions];
        let height = params.degree_bits + params.config.rate_bits;
        let heights = reduction_arity_bits
            .iter()
            .scan(height, |acc, &bits| {
                *acc -= bits;
                Some(*acc)
            })
            .collect::<Vec<_>>();

        // Holds the `evals` vectors that have already been reconstructed at each reduction depth.
        let mut evals_by_depth =
            vec![HashMap::<usize, Vec<_>>::new(); params.reduction_arity_bits.len()];
        for &(mut index) in indices {
            let initial_trees_proof = query_round_proofs.initial_trees_proofs[&index].clone();
            for (i, (leaves_data, proof)) in
                initial_trees_proof.evals_proofs.into_iter().enumerate()
            {
                initial_trees_indices[i].push(index);
                initial_trees_leaves[i].push(leaves_data);
                initial_trees_proofs[i].push(proof);
            }
            for i in 0..num_reductions {
                let index_within_coset = index & ((1 << reduction_arity_bits[i]) - 1);
                index >>= reduction_arity_bits[i];
                let FriQueryStep {
                    mut evals,
                    merkle_proof,
                } = query_round_proofs.steps[i][&index].clone();
                steps_indices[i].push(index);
                if let Some(v) = evals_by_depth[i].get(&index) {
                    // If this index has already been seen, get `evals` from the `HashMap`.
                    evals = v.to_vec();
                } else {
                    // Otherwise insert the next inferred element.
                    evals.insert(index_within_coset, fri_inferred_elements.next().unwrap());
                    evals_by_depth[i].insert(index, evals.clone());
                }
                steps_evals[i].push(flatten(&evals));
                steps_proofs[i].push(merkle_proof);
            }
        }

        // Decompress all Merkle proofs.
        let initial_trees_proofs = izip!(
            &initial_trees_leaves,
            &initial_trees_indices,
            initial_trees_proofs
        )
        .map(|(ls, is, ps)| decompress_merkle_proofs(ls, is, &ps, height, cap_height))
        .collect::<Vec<_>>();
        let steps_proofs = izip!(&steps_evals, &steps_indices, steps_proofs, heights)
            .map(|(ls, is, ps, h)| decompress_merkle_proofs(ls, is, &ps, h, cap_height))
            .collect::<Vec<_>>();

        let mut decompressed_query_proofs = Vec::with_capacity(num_reductions);
        for i in 0..indices.len() {
            let initial_trees_proof = FriInitialTreeProof {
                evals_proofs: (0..num_initial_trees)
                    .map(|j| {
                        (
                            initial_trees_leaves[j][i].clone(),
                            initial_trees_proofs[j][i].clone(),
                        )
                    })
                    .collect(),
            };
            let steps = (0..num_reductions)
                .map(|j| FriQueryStep {
                    evals: unflatten(&steps_evals[j][i]),
                    merkle_proof: steps_proofs[j][i].clone(),
                })
                .collect();
            decompressed_query_proofs.push(FriQueryRound {
                initial_trees_proof,
                steps,
            })
        }

        FriProof {
            commit_phase_merkle_caps,
            query_round_proofs: decompressed_query_proofs,
            final_poly,
            pow_witness,
        }
    }
}

pub struct FriChallenges<F: RichField + Extendable<D>, const D: usize> {
    // Scaling factor to combine polynomials.
    pub fri_alpha: F::Extension,

    // Betas used in the FRI commit phase reductions.
    pub fri_betas: Vec<F::Extension>,

    pub fri_pow_response: F,

    // Indices at which the oracle is queried in FRI.
    pub fri_query_indices: Vec<usize>,
}

pub struct FriChallengesTarget<const D: usize> {
    pub fri_alpha: ExtensionTarget<D>,
    pub fri_betas: Vec<ExtensionTarget<D>>,
    pub fri_pow_response: Target,
    pub fri_query_indices: Vec<Target>,
}
