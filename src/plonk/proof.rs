use serde::{Deserialize, Serialize};

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::fri::commitment::{OpeningProof, OpeningProofTarget, PolynomialBatchCommitment};
use crate::hash::hash_types::{HashOut, HashOutTarget};
use crate::iop::target::Target;
use crate::plonk::circuit_data::CommonCircuitData;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "")]
pub struct Proof<F: Extendable<D>, const D: usize> {
    /// Merkle root of LDEs of wire values.
    pub wires_root: HashOut<F>,
    /// Merkle root of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_zs_partial_products_root: HashOut<F>,
    /// Merkle root of LDEs of the quotient polynomial components.
    pub quotient_polys_root: HashOut<F>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: OpeningSet<F, D>,
    /// A FRI argument for each FRI query.
    pub opening_proof: OpeningProof<F, D>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "")]
pub struct ProofWithPublicInputs<F: Extendable<D>, const D: usize> {
    pub proof: Proof<F, D>,
    pub public_inputs: Vec<F>,
}

pub struct ProofTarget<const D: usize> {
    pub wires_root: HashOutTarget,
    pub plonk_zs_partial_products_root: HashOutTarget,
    pub quotient_polys_root: HashOutTarget,
    pub openings: OpeningSetTarget<D>,
    pub opening_proof: OpeningProofTarget<D>,
}

pub struct ProofWithPublicInputsTarget<const D: usize> {
    pub proof: ProofTarget<D>,
    pub public_inputs: Vec<Target>,
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
        constants_sigmas_commitment: &PolynomialBatchCommitment<F>,
        wires_commitment: &PolynomialBatchCommitment<F>,
        zs_partial_products_commitment: &PolynomialBatchCommitment<F>,
        quotient_polys_commitment: &PolynomialBatchCommitment<F>,
        common_data: &CommonCircuitData<F, D>,
    ) -> Self {
        let eval_commitment = |z: F::Extension, c: &PolynomialBatchCommitment<F>| {
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
