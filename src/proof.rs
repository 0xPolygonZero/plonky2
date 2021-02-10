use crate::field::field::Field;
use crate::target::Target2;

pub struct Hash<F: Field> {
    elements: Vec<F>,
}

pub struct HashTarget {
    elements: Vec<Target2>,
}

pub struct Proof2<F: Field> {
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

pub struct ProofTarget2 {
    /// Merkle root of LDEs of wire values.
    pub wires_root: HashTarget,
    /// Merkle root of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_z_root: HashTarget,
    /// Merkle root of LDEs of the quotient polynomial components.
    pub plonk_t_root: HashTarget,

    /// Purported values of each polynomial at each challenge point.
    pub openings: Vec<OpeningSetTarget>,

    // TODO: FRI Merkle proofs.
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
    pub constants: Vec<Target2>,
    pub plonk_sigmas: Vec<Target2>,
    pub wires: Vec<Target2>,
    // TODO: One or multiple?
    pub plonk_z: Vec<Target2>,
    pub plonk_t: Vec<Target2>,
}
