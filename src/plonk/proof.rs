use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::fri::commitment::PolynomialBatchCommitment;
use crate::fri::proof::{CompressedFriProof, FriProof, FriProofTarget};
use crate::hash::hash_types::{HashOut, MerkleCapTarget};
use crate::hash::hashing::hash_n_to_hash;
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::Target;
use crate::plonk::circuit_data::{CommonCircuitData, VerifierOnlyCircuitData};
use crate::plonk::verifier::verify_with_challenges;
use crate::util::serialization::Buffer;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct Proof<F: Extendable<D>, const D: usize> {
    /// Merkle cap of LDEs of wire values.
    pub wires_cap: MerkleCap<F>,
    /// Merkle cap of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_zs_partial_products_cap: MerkleCap<F>,
    /// Merkle cap of LDEs of the quotient polynomial components.
    pub quotient_polys_cap: MerkleCap<F>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: OpeningSet<F, D>,
    /// A batch FRI argument for all openings.
    pub opening_proof: FriProof<F, D>,
}

pub struct ProofTarget<const D: usize> {
    pub wires_cap: MerkleCapTarget,
    pub plonk_zs_partial_products_cap: MerkleCapTarget,
    pub quotient_polys_cap: MerkleCapTarget,
    pub openings: OpeningSetTarget<D>,
    pub opening_proof: FriProofTarget<D>,
}

impl<F: RichField + Extendable<D>, const D: usize> Proof<F, D> {
    /// Compress the proof.
    pub fn compress(
        self,
        indices: &[usize],
        common_data: &CommonCircuitData<F, D>,
    ) -> CompressedProof<F, D> {
        let Proof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        } = self;

        CompressedProof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof: opening_proof.compress(indices, common_data),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct ProofWithPublicInputs<F: RichField + Extendable<D>, const D: usize> {
    pub proof: Proof<F, D>,
    pub public_inputs: Vec<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> ProofWithPublicInputs<F, D> {
    pub fn compress(
        self,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<CompressedProofWithPublicInputs<F, D>> {
        let indices = self.fri_query_indices(common_data)?;
        let compressed_proof = self.proof.compress(&indices, common_data);
        Ok(CompressedProofWithPublicInputs {
            public_inputs: self.public_inputs,
            proof: compressed_proof,
        })
    }

    pub(crate) fn get_public_inputs_hash(&self) -> HashOut<F> {
        hash_n_to_hash(self.public_inputs.clone(), true)
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Buffer::new(Vec::new());
        buffer.write_proof_with_public_inputs(self)?;
        Ok(buffer.bytes())
    }

    pub fn from_bytes(
        bytes: Vec<u8>,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<Self> {
        let mut buffer = Buffer::new(bytes);
        let proof = buffer.read_proof_with_public_inputs(common_data)?;
        Ok(proof)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct CompressedProof<F: Extendable<D>, const D: usize> {
    /// Merkle cap of LDEs of wire values.
    pub wires_cap: MerkleCap<F>,
    /// Merkle cap of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_zs_partial_products_cap: MerkleCap<F>,
    /// Merkle cap of LDEs of the quotient polynomial components.
    pub quotient_polys_cap: MerkleCap<F>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: OpeningSet<F, D>,
    /// A compressed batch FRI argument for all openings.
    pub opening_proof: CompressedFriProof<F, D>,
}

impl<F: RichField + Extendable<D>, const D: usize> CompressedProof<F, D> {
    /// Decompress the proof.
    pub(crate) fn decompress(
        self,
        challenges: &ProofChallenges<F, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> Proof<F, D> {
        let CompressedProof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        } = self;

        Proof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof: opening_proof.decompress(challenges, common_data),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct CompressedProofWithPublicInputs<F: RichField + Extendable<D>, const D: usize> {
    pub proof: CompressedProof<F, D>,
    pub public_inputs: Vec<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> CompressedProofWithPublicInputs<F, D> {
    pub fn decompress(
        self,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, D>> {
        let challenges = self.get_challenges(common_data)?;
        let compressed_proof = self.proof.decompress(&challenges, common_data);
        Ok(ProofWithPublicInputs {
            public_inputs: self.public_inputs,
            proof: compressed_proof,
        })
    }

    pub(crate) fn verify(
        self,
        verifier_data: &VerifierOnlyCircuitData<F>,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<()> {
        let challenges = self.get_challenges(common_data)?;
        dbg!(&challenges.fri_query_inferred_elements);
        let compressed_proof = self.proof.decompress(&challenges, common_data);
        verify_with_challenges(
            ProofWithPublicInputs {
                public_inputs: self.public_inputs,
                proof: compressed_proof,
            },
            challenges,
            verifier_data,
            common_data,
        )
    }

    pub(crate) fn get_public_inputs_hash(&self) -> HashOut<F> {
        hash_n_to_hash(self.public_inputs.clone(), true)
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Buffer::new(Vec::new());
        buffer.write_compressed_proof_with_public_inputs(self)?;
        Ok(buffer.bytes())
    }

    pub fn from_bytes(
        bytes: Vec<u8>,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<Self> {
        let mut buffer = Buffer::new(bytes);
        let proof = buffer.read_compressed_proof_with_public_inputs(common_data)?;
        Ok(proof)
    }
}

pub(crate) struct ProofChallenges<F: RichField + Extendable<D>, const D: usize> {
    // Random values used in Plonk's permutation argument.
    pub plonk_betas: Vec<F>,

    // Random values used in Plonk's permutation argument.
    pub plonk_gammas: Vec<F>,

    // Random values used to combine PLONK constraints.
    pub plonk_alphas: Vec<F>,

    // Point at which the PLONK polynomials are opened.
    pub plonk_zeta: F::Extension,

    // Scaling factor to combine polynomials.
    pub fri_alpha: F::Extension,

    // Betas used in the FRI commit phase reductions.
    pub fri_betas: Vec<F::Extension>,

    pub fri_pow_response: F,

    pub fri_query_indices: Vec<usize>,

    pub fri_query_inferred_elements: Option<Vec<Vec<F::Extension>>>,
}

pub struct ProofWithPublicInputsTarget<const D: usize> {
    pub proof: ProofTarget<D>,
    pub public_inputs: Vec<Target>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
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

impl<F: RichField + Extendable<D>, const D: usize> OpeningSet<F, D> {
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
                .par_iter()
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

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::fri::reduction_strategies::FriReductionStrategy;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    #[test]
    fn test_proof_compression() -> Result<()> {
        type F = CrandallField;
        const D: usize = 4;

        let mut config = CircuitConfig::large_config();
        config.fri_config.reduction_strategy = FriReductionStrategy::Fixed(vec![1]);
        config.fri_config.num_query_rounds = 50;

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        // Build dummy circuit to get a valid proof.
        let x = F::rand();
        let y = F::rand();
        let z = x * y;
        let xt = builder.constant(x);
        let yt = builder.constant(y);
        let zt = builder.constant(z);
        let comp_zt = builder.mul(xt, yt);
        builder.connect(zt, comp_zt);
        let data = builder.build();
        let proof = data.prove(pw)?;

        // Verify that `decompress ∘ compress = identity`.
        let compressed_proof = proof.clone().compress(&data.common)?;
        let decompressed_compressed_proof = compressed_proof.clone().decompress(&data.common)?;
        assert_eq!(proof, decompressed_compressed_proof);

        verify(proof, &data.verifier_only, &data.common)?;
        compressed_proof.verify(&data.verifier_only, &data.common)
    }
}
