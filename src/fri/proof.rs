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
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::plonk_common::PolynomialsIndexBlinding;
use crate::polynomial::polynomial::PolynomialCoeffs;

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
    pub index: usize,
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
    // `initial_trees_leaves[i][j]` is the vector of leaves of the i-th initial tree at the j-th query.
    pub initial_trees_leaves: Vec<Vec<Vec<F>>>,
    // `initial_trees_proofs[i]` is the compressed Merkle proof for the i-th initial tree.
    pub initial_trees_proofs: Vec<CompressedMerkleProof<F>>,
    // `steps_evals[i][j]` is the vector of leaves of the i-th reduced tree at the j-th query.
    pub steps_evals: Vec<Vec<Vec<F::Extension>>>,
    // `steps_proofs[i]` is the compressed Merkle proof for the i-th reduced tree.
    pub steps_proofs: Vec<CompressedMerkleProof<F>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "")]
pub struct CompressedFriProof<F: Extendable<D>, const D: usize> {
    /// A Merkle cap for each reduced polynomial in the commit phase.
    pub commit_phase_merkle_caps: Vec<MerkleCap<F>>,
    /// Query rounds proof
    pub query_rounds_proof: CompressedFriQueryRounds<F, D>,
    /// The final polynomial in coefficient form.
    pub final_poly: PolynomialCoeffs<F::Extension>,
    /// Witness showing that the prover did PoW.
    pub pow_witness: F,
}

impl<F: Extendable<D>, const D: usize> FriProof<F, D> {
    pub fn compress(self, common_data: &CommonCircuitData<F, D>) -> CompressedFriProof<F, D> {
        let FriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
        } = self;
        let cap_height = common_data.config.cap_height;
        let reduction_arity_bits = &common_data.config.fri_config.reduction_arity_bits;
        let num_reductions = reduction_arity_bits.len();
        let num_initial_trees = query_round_proofs[0].initial_trees_proof.evals_proofs.len();

        // "Transpose" the query round proofs, so that information for each Merkle tree is collected together.
        let mut initial_trees_leaves = vec![vec![]; num_initial_trees];
        let mut initial_trees_proofs = vec![vec![]; num_initial_trees];
        let mut steps_evals = vec![vec![]; num_reductions];
        let mut steps_proofs = vec![vec![]; num_reductions];

        for qrp in query_round_proofs {
            let FriQueryRound {
                mut index,
                initial_trees_proof,
                steps,
            } = qrp;
            for (i, (leaves_data, proof)) in
                initial_trees_proof.evals_proofs.into_iter().enumerate()
            {
                initial_trees_leaves[i].push(leaves_data);
                initial_trees_proofs[i].push((index, proof));
            }
            for (i, query_step) in steps.into_iter().enumerate() {
                index >>= reduction_arity_bits[i];
                steps_evals[i].push(query_step.evals);
                steps_proofs[i].push((index, query_step.merkle_proof));
            }
        }

        // Compress all Merkle proofs.
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
}
