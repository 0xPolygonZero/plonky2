use std::convert::TryInto;

use serde::{Deserialize, Serialize};

use crate::circuit_data::CommonCircuitData;
use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field::Field;
use crate::gadgets::polynomial::PolynomialCoeffsExtTarget;
use crate::merkle_proofs::{MerkleProof, MerkleProofTarget};
use crate::plonk_common::PolynomialsIndexBlinding;
use crate::polynomial::commitment::{ListPolynomialCommitment, OpeningProof, OpeningProofTarget};
use crate::polynomial::polynomial::PolynomialCoeffs;
use crate::target::Target;

/// Represents a ~256 bit hash output.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(bound = "")]
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
#[derive(Copy, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "")]
pub struct Proof<F: Extendable<D>, const D: usize> {
    /// Merkle root of LDEs of wire values.
    pub wires_root: Hash<F>,
    /// Merkle root of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_zs_partial_products_root: Hash<F>,
    /// Merkle root of LDEs of the quotient polynomial components.
    pub quotient_polys_root: Hash<F>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: OpeningSet<F, D>,
    /// A FRI argument for each FRI query.
    pub opening_proof: OpeningProof<F, D>,
}

pub struct ProofTarget<const D: usize> {
    pub wires_root: HashTarget,
    pub plonk_zs_partial_products_root: HashTarget,
    pub quotient_polys_root: HashTarget,
    pub openings: OpeningSetTarget<D>,
    pub opening_proof: OpeningProofTarget<D>,
}

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
    pub(crate) fn unsalted_evals(&self, polynomials: PolynomialsIndexBlinding) -> &[F] {
        let evals = &self.evals_proofs[polynomials.index].0;
        &evals[..evals.len() - polynomials.salt_size()]
    }
}

#[derive(Clone)]
pub struct FriInitialTreeProofTarget {
    pub evals_proofs: Vec<(Vec<Target>, MerkleProofTarget)>,
}

impl FriInitialTreeProofTarget {
    pub(crate) fn unsalted_evals(&self, polynomials: PolynomialsIndexBlinding) -> &[Target] {
        let evals = &self.evals_proofs[polynomials.index].0;
        &evals[..evals.len() - polynomials.salt_size()]
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
    /// A Merkle root for each reduced polynomial in the commit phase.
    pub commit_phase_merkle_roots: Vec<Hash<F>>,
    /// Query rounds proofs
    pub query_round_proofs: Vec<FriQueryRound<F, D>>,
    /// The final polynomial in coefficient form.
    pub final_poly: PolynomialCoeffs<F::Extension>,
    /// Witness showing that the prover did PoW.
    pub pow_witness: F,
}

pub struct FriProofTarget<const D: usize> {
    pub commit_phase_merkle_roots: Vec<HashTarget>,
    pub query_round_proofs: Vec<FriQueryRoundTarget<D>>,
    pub final_poly: PolynomialCoeffsExtTarget<D>,
    pub pow_witness: Target,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// The purported values of each polynomial at a single point.
pub struct OpeningSet<F: Extendable<D>, const D: usize> {
    pub constants: Vec<F::Extension>,
    pub plonk_sigmas: Vec<F::Extension>,
    pub wires: Vec<F::Extension>,
    pub plonk_zs: Vec<F::Extension>,
    pub plonk_zs_right: Vec<F::Extension>,
    pub partial_products: Vec<F::Extension>,
    pub quotient_polys: Vec<F::Extension>,
}

impl<F: Extendable<D>, const D: usize> OpeningSet<F, D> {
    pub fn new(
        z: F::Extension,
        g: F::Extension,
        constants_sigmas_commitment: &ListPolynomialCommitment<F>,
        wires_commitment: &ListPolynomialCommitment<F>,
        zs_partial_products_commitment: &ListPolynomialCommitment<F>,
        quotient_polys_commitment: &ListPolynomialCommitment<F>,
        common_data: &CommonCircuitData<F, D>,
    ) -> Self {
        let eval_commitment = |z: F::Extension, c: &ListPolynomialCommitment<F>| {
            c.polynomials
                .iter()
                .map(|p| p.to_extension().eval(z))
                .collect::<Vec<_>>()
        };
        let constants_sigmas_eval = eval_commitment(z, constants_sigmas_commitment);
        let zs_partial_products_eval = eval_commitment(z, zs_partial_products_commitment);
        Self {
            constants: constants_sigmas_eval[common_data.constants_range()].to_vec(),
            plonk_sigmas: constants_sigmas_eval[common_data.sigmas_range()].to_vec(),
            wires: eval_commitment(z, wires_commitment),
            plonk_zs: zs_partial_products_eval[common_data.zs_range()].to_vec(),
            plonk_zs_right: eval_commitment(g * z, zs_partial_products_commitment)
                [common_data.zs_range()]
            .to_vec(),
            partial_products: zs_partial_products_eval[common_data.partial_products_range()]
                .to_vec(),
            quotient_polys: eval_commitment(z, quotient_polys_commitment),
        }
    }
}

/// The purported values of each polynomial at a single point.
#[derive(Clone, Debug)]
pub struct OpeningSetTarget<const D: usize> {
    pub constants: Vec<ExtensionTarget<D>>,
    pub plonk_sigmas: Vec<ExtensionTarget<D>>,
    pub wires: Vec<ExtensionTarget<D>>,
    pub plonk_zs: Vec<ExtensionTarget<D>>,
    pub plonk_zs_right: Vec<ExtensionTarget<D>>,
    pub partial_products: Vec<ExtensionTarget<D>>,
    pub quotient_polys: Vec<ExtensionTarget<D>>,
}
