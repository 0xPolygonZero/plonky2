use serde::{Deserialize, Serialize};

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::Field;
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::hash::hash_types::MerkleCapTarget;
use crate::hash::merkle_proofs::{MerkleProof, MerkleProofTarget};
use crate::hash::merkle_tree::MerkleCap;
use crate::hash::path_compression::{compress_merkle_proofs, CompressedMerkleProof};
use crate::iop::target::Target;
use crate::plonk::plonk_common::PolynomialsIndexBlinding;
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::util::log2_strict;

/// Evaluations and Merkle proof produced by the prover in a FRI query step.
#[derive(Serialize, Deserialize, Clone, Debug)]
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
#[derive(Serialize, Deserialize, Clone, Debug)]
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
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "")]
pub struct FriQueryRound<F: Extendable<D>, const D: usize> {
    pub initial_trees_proof: FriInitialTreeProof<F>,
    pub steps: Vec<FriQueryStep<F, D>>,
}

#[derive(Clone)]
pub struct FriQueryRoundTarget<const D: usize> {
    pub initial_trees_proof: FriInitialTreeProofTarget,
    pub steps: Vec<FriQueryStepTarget<D>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
}

pub struct FriProofTarget<const D: usize> {
    pub commit_phase_merkle_caps: Vec<MerkleCapTarget>,
    pub query_round_proofs: Vec<FriQueryRoundTarget<D>>,
    pub final_poly: PolynomialCoeffsExtTarget<D>,
    pub pow_witness: Target,
}

/// Compressed proof for all FRI query rounds.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "")]
pub struct CompressedFriQueryRounds<F: Extendable<D>, const D: usize> {
    pub initial_trees_leaves: Vec<Vec<Vec<F>>>,
    pub initial_trees_proofs: Vec<CompressedMerkleProof<F>>,
    pub steps_evals: Vec<Vec<Vec<F::Extension>>>,
    pub steps_proofs: Vec<CompressedMerkleProof<F>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "")]
pub struct CompressedFriProof<F: Extendable<D>, const D: usize> {
    /// A Merkle cap for each reduced polynomial in the commit phase.
    pub commit_phase_merkle_caps: Vec<MerkleCap<F>>,
    /// Query rounds proofs
    pub query_rounds_proof: CompressedFriQueryRounds<F, D>,
    /// The final polynomial in coefficient form.
    pub final_poly: PolynomialCoeffs<F::Extension>,
    /// Witness showing that the prover did PoW.
    pub pow_witness: F,
}

pub fn compress_fri_proof<F: Extendable<D>, const D: usize>(
    proof: FriProof<F, D>,
    indices: &[usize],
) -> CompressedFriProof<F, D> {
    let FriProof {
        commit_phase_merkle_caps,
        query_round_proofs,
        final_poly,
        pow_witness,
    } = proof;
    let cap_height = log2_strict(commit_phase_merkle_caps[0].0.len());

    let num_initial_trees = query_round_proofs[0].initial_trees_proof.evals_proofs.len();
    let mut initial_trees_leaves = vec![vec![]; num_initial_trees];
    let mut initial_trees_proofs = vec![vec![]; num_initial_trees];
    let num_reductions = query_round_proofs[0].steps.len();
    let mut steps_evals = vec![vec![]; num_reductions];
    let mut steps_proofs = vec![vec![]; num_reductions];

    for (&index, qrp) in indices.iter().zip(query_round_proofs) {
        let mut index = index;
        for i in 0..num_initial_trees {
            initial_trees_leaves[i].push(qrp.initial_trees_proof.evals_proofs[i].0.clone());
            initial_trees_proofs[i]
                .push((index, qrp.initial_trees_proof.evals_proofs[i].1.clone()));
        }
        for i in 0..num_reductions {
            steps_evals[i].push(qrp.steps[i].evals.clone());
            steps_proofs[i].push((index, qrp.steps[i].merkle_proof.clone()));
            index >>= 1;
        }
    }

    let initial_trees_proofs = initial_trees_proofs
        .into_iter()
        .map(|ps| compress_merkle_proofs(cap_height, ps))
        .collect();
    let steps_proofs = steps_proofs
        .into_iter()
        .map(|ps| compress_merkle_proofs(cap_height, ps))
        .collect();

    CompressedFriProof {
        commit_phase_merkle_caps,
        query_rounds_proof: CompressedFriQueryRounds {
            initial_trees_leaves,
            initial_trees_proofs,
            steps_evals,
            steps_proofs,
        },
        final_poly,
        pow_witness,
    }
}
