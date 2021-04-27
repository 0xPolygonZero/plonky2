use crate::field::field::Field;
use crate::merkle_proofs::{MerkleProof, MerkleProofTarget};
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::target::Target;
use std::convert::TryInto;

/// Represents a ~256 bit hash output.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Hash<F: Field> {
    pub(crate) elements: [F; 4],
}

impl<F: Field> Hash<F> {
    pub(crate) fn from_vec(elements: Vec<F>) -> Self {
        debug_assert!(elements.len() == 4);
        Self {
            elements: elements.try_into().unwrap(),
        }
    }

    pub(crate) fn from_partial(mut elements: Vec<F>) -> Self {
        debug_assert!(elements.len() <= 4);
        while elements.len() < 4 {
            elements.push(F::ZERO);
        }
        Self {
            elements: [elements[0], elements[1], elements[2], elements[3]],
        }
    }
}

/// Represents a ~256 bit hash output.
pub struct HashTarget {
    pub(crate) elements: [Target; 4],
}

impl HashTarget {
    pub(crate) fn from_vec(elements: Vec<Target>) -> Self {
        debug_assert!(elements.len() == 4);
        Self {
            elements: elements.try_into().unwrap(),
        }
    }

    pub(crate) fn from_partial(mut elements: Vec<Target>, zero: Target) -> Self {
        debug_assert!(elements.len() <= 4);
        while elements.len() < 4 {
            elements.push(zero);
        }
        Self {
            elements: [elements[0], elements[1], elements[2], elements[3]],
        }
    }
}

pub struct Proof<F: Field> {
    /// Merkle root of LDEs of wire values.
    pub wires_root: Hash<F>,
    /// Merkle root of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_zs_root: Hash<F>,
    /// Merkle root of LDEs of the quotient polynomial components.
    pub quotient_polys_root: Hash<F>,

    /// Purported values of each polynomial at each challenge point.
    pub openings: Vec<OpeningSet<F>>,

    /// A FRI argument for each FRI query.
    pub fri_proofs: Vec<FriProof<F>>,
}

pub struct ProofTarget {
    /// Merkle root of LDEs of wire values.
    pub wires_root: HashTarget,
    /// Merkle root of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_zs_root: HashTarget,
    /// Merkle root of LDEs of the quotient polynomial components.
    pub quotient_polys_root: HashTarget,

    /// Purported values of each polynomial at each challenge point.
    pub openings: Vec<OpeningSetTarget>,

    /// A FRI argument for each FRI query.
    pub fri_proofs: Vec<FriProofTarget>,
}

/// Evaluations and Merkle proof produced by the prover in a FRI query step.
// TODO: Implement FriQueryStepTarget
pub struct FriQueryStep<F: Field> {
    pub evals: Vec<F>,
    pub merkle_proof: MerkleProof<F>,
}

/// Proof for a FRI query round.
// TODO: Implement FriQueryRoundTarget
pub struct FriQueryRound<F: Field> {
    pub steps: Vec<FriQueryStep<F>>,
}

pub struct FriProof<F: Field> {
    /// A Merkle root for each reduced polynomial in the commit phase.
    pub commit_phase_merkle_roots: Vec<Hash<F>>,
    /// Merkle proofs for the original purported codewords, i.e. the subject of the LDT.
    pub initial_merkle_proofs: Vec<MerkleProof<F>>,
    /// Query rounds proofs
    pub query_round_proofs: Vec<FriQueryRound<F>>,
    /// The final polynomial in coefficient form.
    pub final_poly: PolynomialCoeffs<F>,
    /// Witness showing that the prover did PoW.
    pub pow_witness: F,
}

/// Represents a single FRI query, i.e. a path through the reduction tree.
pub struct FriProofTarget {
    /// A Merkle root for each reduced polynomial in the commit phase.
    pub commit_phase_merkle_roots: Vec<HashTarget>,
    /// Merkle proofs for the original purported codewords, i.e. the subject of the LDT.
    pub initial_merkle_proofs: Vec<MerkleProofTarget>,
    /// Merkle proofs for the reduced polynomials that were sent in the commit phase.
    pub intermediate_merkle_proofs: Vec<MerkleProofTarget>,
    /// The final polynomial in coefficient form.
    pub final_poly: Vec<Target>,
}

/// The purported values of each polynomial at a single point.
pub struct OpeningSet<F: Field> {
    pub constants: Vec<F>,
    pub plonk_sigmas: Vec<F>,
    pub wires: Vec<F>,
    pub plonk_zs: Vec<F>,
    pub quotient_polys: Vec<F>,
}

/// The purported values of each polynomial at a single point.
pub struct OpeningSetTarget {
    pub constants: Vec<Target>,
    pub plonk_sigmas: Vec<Target>,
    pub wires: Vec<Target>,
    pub plonk_zs: Vec<Target>,
    pub quotient_polys: Vec<Target>,
}
