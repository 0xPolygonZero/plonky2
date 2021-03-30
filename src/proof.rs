use crate::field::field::Field;
use crate::target::Target;

/// Represents a ~256 bit hash output.
#[derive(Copy, Clone)]
pub struct Hash<F: Field> {
    pub(crate) elements: [F; 4],
}

impl<F: Field> Hash<F> {
    pub(crate) fn from_partial(mut elements: Vec<F>) -> Self {
        debug_assert!(elements.len() <= 4);
        while elements.len() < 4 {
            elements.push(F::ZERO);
        }
        Self { elements: [elements[0], elements[1], elements[2], elements[3]] }
    }
}

pub struct HashTarget {
    elements: Vec<Target>,
}

pub struct Proof<F: Field> {
    /// Merkle root of LDEs of wire values.
    pub wires_root: Hash<F>,
    /// Merkle root of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_z_root: Hash<F>,
    /// Merkle root of LDEs of the quotient polynomial components.
    pub plonk_t_root: Hash<F>,

    /// Purported values of each polynomial at each challenge point.
    pub openings: Vec<OpeningSet<F>>,

    // TODO: FRI Merkle proofs.
}

pub struct ProofTarget {
    /// Merkle root of LDEs of wire values.
    pub wires_root: HashTarget,
    /// Merkle root of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_z_root: HashTarget,
    /// Merkle root of LDEs of the quotient polynomial components.
    pub plonk_t_root: HashTarget,

    /// Purported values of each polynomial at each challenge point.
    pub openings: Vec<OpeningSetTarget>,

    /// A FRI argument for each FRI query.
    pub fri_proofs: Vec<FriProofTarget>,
}

/// Represents a single FRI query, i.e. a path through the reduction tree.
pub struct FriProofTarget {
    /// Merkle proofs for the original purported codewords, i.e. the subject of the LDT.
    pub initial_merkle_proofs: Vec<MerkleProofTarget>,
    /// Merkle proofs for the reduced polynomials that were sent in the commit phase.
    pub intermediate_merkle_proofs: Vec<MerkleProofTarget>,
    /// The final polynomial in point-value form.
    pub final_poly: Vec<Target>,
}

pub struct MerkleProofTarget {
    pub leaf: Vec<Target>,
    pub siblings: Vec<Target>,
    // TODO: Also need left/right turn info.
}

/// The purported values of each polynomial at a single point.
pub struct OpeningSet<F: Field> {
    pub constants: Vec<F>,
    pub plonk_sigmas: Vec<F>,
    pub wires: Vec<F>,
    // TODO: One or multiple?
    pub plonk_z: Vec<F>,
    pub plonk_t: Vec<F>,
}

/// The purported values of each polynomial at a single point.
pub struct OpeningSetTarget {
    pub constants: Vec<Target>,
    pub plonk_sigmas: Vec<Target>,
    pub wires: Vec<Target>,
    // TODO: One or multiple?
    pub plonk_z: Vec<Target>,
    pub plonk_t: Vec<Target>,
}
