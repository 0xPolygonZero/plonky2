use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::fri::FriConfig;
use crate::merkle_proofs::{MerkleProof, MerkleProofTarget};
use crate::polynomial::commitment::{ListPolynomialCommitment, OpeningProof};
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

pub struct Proof<F: Field + Extendable<D>, const D: usize> {
    /// Merkle root of LDEs of wire values.
    pub wires_root: Hash<F>,
    /// Merkle root of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_zs_root: Hash<F>,
    /// Merkle root of LDEs of the quotient polynomial components.
    pub quotient_polys_root: Hash<F>,

    /// Purported values of each polynomial at the challenge point.
    pub openings: OpeningSet<F, D>,

    /// A FRI argument for each FRI query.
    pub opening_proof: OpeningProof<F, D>,
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
pub struct FriQueryStep<F: Field + Extendable<D>, const D: usize> {
    pub evals: Vec<F::Extension>,
    pub merkle_proof: MerkleProof<F>,
}

/// Evaluations and Merkle proofs of the original set of polynomials,
/// before they are combined into a composition polynomial.
// TODO: Implement FriInitialTreeProofTarget
pub struct FriInitialTreeProof<F: Field> {
    pub evals_proofs: Vec<(Vec<F>, MerkleProof<F>)>,
}

impl<F: Field> FriInitialTreeProof<F> {
    pub(crate) fn unsalted_evals(&self, i: usize, config: &FriConfig) -> &[F] {
        let evals = &self.evals_proofs[i].0;
        &evals[..evals.len() - config.salt_size(i)]
    }
}

/// Proof for a FRI query round.
// TODO: Implement FriQueryRoundTarget
pub struct FriQueryRound<F: Field + Extendable<D>, const D: usize> {
    pub initial_trees_proof: FriInitialTreeProof<F>,
    pub steps: Vec<FriQueryStep<F, D>>,
}

pub struct FriProof<F: Field + Extendable<D>, const D: usize> {
    /// A Merkle root for each reduced polynomial in the commit phase.
    pub commit_phase_merkle_roots: Vec<Hash<F>>,
    /// Query rounds proofs
    pub query_round_proofs: Vec<FriQueryRound<F, D>>,
    /// The final polynomial in coefficient form.
    pub final_poly: PolynomialCoeffs<F::Extension>,
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
pub struct OpeningSet<F: Field + Extendable<D>, const D: usize> {
    pub constants: Vec<F::Extension>,
    pub plonk_sigmas: Vec<F::Extension>,
    pub wires: Vec<F::Extension>,
    pub plonk_zs: Vec<F::Extension>,
    pub plonk_zs_right: Vec<F::Extension>,
    pub quotient_polys: Vec<F::Extension>,
}

impl<F: Field + Extendable<D>, const D: usize> OpeningSet<F, D> {
    pub fn new(
        z: F::Extension,
        g: F::Extension,
        constant_commitment: &ListPolynomialCommitment<F>,
        plonk_sigmas_commitment: &ListPolynomialCommitment<F>,
        wires_commitment: &ListPolynomialCommitment<F>,
        plonk_zs_commitment: &ListPolynomialCommitment<F>,
        quotient_polys_commitment: &ListPolynomialCommitment<F>,
    ) -> Self {
        let eval_commitment = |z: F::Extension, c: &ListPolynomialCommitment<F>| {
            c.polynomials
                .iter()
                .map(|p| p.to_extension().eval(z))
                .collect::<Vec<_>>()
        };
        Self {
            constants: eval_commitment(z, constant_commitment),
            plonk_sigmas: eval_commitment(z, plonk_sigmas_commitment),
            wires: eval_commitment(z, wires_commitment),
            plonk_zs: eval_commitment(z, plonk_zs_commitment),
            plonk_zs_right: eval_commitment(g * z, plonk_zs_commitment),
            quotient_polys: eval_commitment(z, quotient_polys_commitment),
        }
    }
}

/// The purported values of each polynomial at a single point.
pub struct OpeningSetTarget {
    pub constants: Vec<Target>,
    pub plonk_sigmas: Vec<Target>,
    pub wires: Vec<Target>,
    pub plonk_zs: Vec<Target>,
    pub quotient_polys: Vec<Target>,
}
