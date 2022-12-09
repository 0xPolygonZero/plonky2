use alloc::vec;
use alloc::vec::Vec;

use anyhow::ensure;
use maybe_rayon::*;
use serde::{Deserialize, Serialize};

use crate::field::extension::Extendable;
use crate::fri::oracle::PolynomialBatch;
use crate::fri::proof::{
    CompressedFriProof, FriChallenges, FriChallengesTarget, FriProof, FriProofTarget,
};
use crate::fri::structure::{
    FriOpeningBatch, FriOpeningBatchTarget, FriOpenings, FriOpeningsTarget,
};
use crate::fri::FriParams;
use crate::hash::hash_types::{MerkleCapTarget, RichField};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::target::Target;
use crate::plonk::circuit_data::{CommonCircuitData, VerifierOnlyCircuitData};
use crate::plonk::config::{GenericConfig, Hasher};
use crate::plonk::verifier::verify_with_challenges;
use crate::util::serialization::Write;
#[cfg(feature = "std")]
use crate::util::serialization::{Buffer, Read};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct Proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    /// Merkle cap of LDEs of wire values.
    pub wires_cap: MerkleCap<F, C::Hasher>,
    /// Merkle cap of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_zs_partial_products_cap: MerkleCap<F, C::Hasher>,
    /// Merkle cap of LDEs of the quotient polynomial components.
    pub quotient_polys_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: OpeningSet<F, D>,
    /// A batch FRI argument for all openings.
    pub opening_proof: FriProof<F, C::Hasher, D>,
}

#[derive(Clone, Debug)]
pub struct ProofTarget<const D: usize> {
    pub wires_cap: MerkleCapTarget,
    pub plonk_zs_partial_products_cap: MerkleCapTarget,
    pub quotient_polys_cap: MerkleCapTarget,
    pub openings: OpeningSetTarget<D>,
    pub opening_proof: FriProofTarget<D>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> Proof<F, C, D> {
    /// Compress the proof.
    pub fn compress(self, indices: &[usize], params: &FriParams) -> CompressedProof<F, C, D> {
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
            opening_proof: opening_proof.compress::<C>(indices, params),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct ProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub proof: Proof<F, C, D>,
    pub public_inputs: Vec<F>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    ProofWithPublicInputs<F, C, D>
{
    pub fn compress(
        self,
        circuit_digest: &<<C as GenericConfig<D>>::Hasher as Hasher<C::F>>::Hash,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<CompressedProofWithPublicInputs<F, C, D>> {
        let indices = self.fri_query_indices(circuit_digest, common_data)?;
        let compressed_proof = self.proof.compress(&indices, &common_data.fri_params);
        Ok(CompressedProofWithPublicInputs {
            public_inputs: self.public_inputs,
            proof: compressed_proof,
        })
    }

    pub(crate) fn get_public_inputs_hash(
        &self,
    ) -> <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash {
        C::InnerHasher::hash_no_pad(&self.public_inputs)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer
            .write_proof_with_public_inputs(self)
            .expect("Writing to a byte-vector cannot fail.");
        buffer
    }

    #[cfg(feature = "std")]
    pub fn from_bytes(
        bytes: Vec<u8>,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<Self> {
        let mut buffer = Buffer::new(bytes);
        let proof = buffer
            .read_proof_with_public_inputs(common_data)
            .map_err(anyhow::Error::msg)?;
        Ok(proof)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct CompressedProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
{
    /// Merkle cap of LDEs of wire values.
    pub wires_cap: MerkleCap<F, C::Hasher>,
    /// Merkle cap of LDEs of Z, in the context of Plonk's permutation argument.
    pub plonk_zs_partial_products_cap: MerkleCap<F, C::Hasher>,
    /// Merkle cap of LDEs of the quotient polynomial components.
    pub quotient_polys_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: OpeningSet<F, D>,
    /// A compressed batch FRI argument for all openings.
    pub opening_proof: CompressedFriProof<F, C::Hasher, D>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    CompressedProof<F, C, D>
{
    /// Decompress the proof.
    pub(crate) fn decompress(
        self,
        challenges: &ProofChallenges<F, D>,
        fri_inferred_elements: FriInferredElements<F, D>,
        params: &FriParams,
    ) -> Proof<F, C, D> {
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
            opening_proof: opening_proof.decompress::<C>(challenges, fri_inferred_elements, params),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(bound = "")]
pub struct CompressedProofWithPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub proof: CompressedProof<F, C, D>,
    pub public_inputs: Vec<F>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    CompressedProofWithPublicInputs<F, C, D>
{
    pub fn decompress(
        self,
        circuit_digest: &<<C as GenericConfig<D>>::Hasher as Hasher<C::F>>::Hash,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<ProofWithPublicInputs<F, C, D>> {
        let challenges =
            self.get_challenges(self.get_public_inputs_hash(), circuit_digest, common_data)?;
        let fri_inferred_elements = self.get_inferred_elements(&challenges, common_data);
        let decompressed_proof =
            self.proof
                .decompress(&challenges, fri_inferred_elements, &common_data.fri_params);
        Ok(ProofWithPublicInputs {
            public_inputs: self.public_inputs,
            proof: decompressed_proof,
        })
    }

    pub(crate) fn verify(
        self,
        verifier_data: &VerifierOnlyCircuitData<C, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<()> {
        ensure!(
            self.public_inputs.len() == common_data.num_public_inputs,
            "Number of public inputs doesn't match circuit data."
        );
        let public_inputs_hash = self.get_public_inputs_hash();
        let challenges = self.get_challenges(
            public_inputs_hash,
            &verifier_data.circuit_digest,
            common_data,
        )?;
        let fri_inferred_elements = self.get_inferred_elements(&challenges, common_data);
        let decompressed_proof =
            self.proof
                .decompress(&challenges, fri_inferred_elements, &common_data.fri_params);
        verify_with_challenges(
            decompressed_proof,
            public_inputs_hash,
            challenges,
            verifier_data,
            common_data,
        )
    }

    pub(crate) fn get_public_inputs_hash(
        &self,
    ) -> <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash {
        C::InnerHasher::hash_no_pad(&self.public_inputs)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer
            .write_compressed_proof_with_public_inputs(self)
            .expect("Writing to a byte-vector cannot fail.");
        buffer
    }

    #[cfg(feature = "std")]
    pub fn from_bytes(
        bytes: Vec<u8>,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<Self> {
        let mut buffer = Buffer::new(bytes);
        let proof = buffer
            .read_compressed_proof_with_public_inputs(common_data)
            .map_err(anyhow::Error::msg)?;
        Ok(proof)
    }
}

pub(crate) struct ProofChallenges<F: RichField + Extendable<D>, const D: usize> {
    /// Random values used in Plonk's permutation argument.
    pub plonk_betas: Vec<F>,

    /// Random values used in Plonk's permutation argument.
    pub plonk_gammas: Vec<F>,

    /// Random values used to combine PLONK constraints.
    pub plonk_alphas: Vec<F>,

    /// Point at which the PLONK polynomials are opened.
    pub plonk_zeta: F::Extension,

    pub fri_challenges: FriChallenges<F, D>,
}

pub(crate) struct ProofChallengesTarget<const D: usize> {
    pub plonk_betas: Vec<Target>,
    pub plonk_gammas: Vec<Target>,
    pub plonk_alphas: Vec<Target>,
    pub plonk_zeta: ExtensionTarget<D>,
    pub fri_challenges: FriChallengesTarget<D>,
}

/// Coset elements that can be inferred in the FRI reduction steps.
pub(crate) struct FriInferredElements<F: RichField + Extendable<D>, const D: usize>(
    pub Vec<F::Extension>,
);

#[derive(Clone, Debug)]
pub struct ProofWithPublicInputsTarget<const D: usize> {
    pub proof: ProofTarget<D>,
    pub public_inputs: Vec<Target>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
/// The purported values of each polynomial at a single point.
pub struct OpeningSet<F: RichField + Extendable<D>, const D: usize> {
    pub constants: Vec<F::Extension>,
    pub plonk_sigmas: Vec<F::Extension>,
    pub wires: Vec<F::Extension>,
    pub plonk_zs: Vec<F::Extension>,
    pub plonk_zs_next: Vec<F::Extension>,
    pub partial_products: Vec<F::Extension>,
    pub quotient_polys: Vec<F::Extension>,
}

impl<F: RichField + Extendable<D>, const D: usize> OpeningSet<F, D> {
    pub fn new<C: GenericConfig<D, F = F>>(
        zeta: F::Extension,
        g: F::Extension,
        constants_sigmas_commitment: &PolynomialBatch<F, C, D>,
        wires_commitment: &PolynomialBatch<F, C, D>,
        zs_partial_products_commitment: &PolynomialBatch<F, C, D>,
        quotient_polys_commitment: &PolynomialBatch<F, C, D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> Self {
        let eval_commitment = |z: F::Extension, c: &PolynomialBatch<F, C, D>| {
            c.polynomials
                .par_iter()
                .map(|p| p.to_extension().eval(z))
                .collect::<Vec<_>>()
        };
        let constants_sigmas_eval = eval_commitment(zeta, constants_sigmas_commitment);
        let zs_partial_products_eval = eval_commitment(zeta, zs_partial_products_commitment);
        Self {
            constants: constants_sigmas_eval[common_data.constants_range()].to_vec(),
            plonk_sigmas: constants_sigmas_eval[common_data.sigmas_range()].to_vec(),
            wires: eval_commitment(zeta, wires_commitment),
            plonk_zs: zs_partial_products_eval[common_data.zs_range()].to_vec(),
            plonk_zs_next: eval_commitment(g * zeta, zs_partial_products_commitment)
                [common_data.zs_range()]
            .to_vec(),
            partial_products: zs_partial_products_eval[common_data.partial_products_range()]
                .to_vec(),
            quotient_polys: eval_commitment(zeta, quotient_polys_commitment),
        }
    }

    pub(crate) fn to_fri_openings(&self) -> FriOpenings<F, D> {
        let zeta_batch = FriOpeningBatch {
            values: [
                self.constants.as_slice(),
                self.plonk_sigmas.as_slice(),
                self.wires.as_slice(),
                self.plonk_zs.as_slice(),
                self.partial_products.as_slice(),
                self.quotient_polys.as_slice(),
            ]
            .concat(),
        };
        let zeta_next_batch = FriOpeningBatch {
            values: self.plonk_zs_next.clone(),
        };
        FriOpenings {
            batches: vec![zeta_batch, zeta_next_batch],
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
    pub plonk_zs_next: Vec<ExtensionTarget<D>>,
    pub partial_products: Vec<ExtensionTarget<D>>,
    pub quotient_polys: Vec<ExtensionTarget<D>>,
}

impl<const D: usize> OpeningSetTarget<D> {
    pub(crate) fn to_fri_openings(&self) -> FriOpeningsTarget<D> {
        let zeta_batch = FriOpeningBatchTarget {
            values: [
                self.constants.as_slice(),
                self.plonk_sigmas.as_slice(),
                self.wires.as_slice(),
                self.plonk_zs.as_slice(),
                self.partial_products.as_slice(),
                self.quotient_polys.as_slice(),
            ]
            .concat(),
        };
        let zeta_next_batch = FriOpeningBatchTarget {
            values: self.plonk_zs_next.clone(),
        };
        FriOpeningsTarget {
            batches: vec![zeta_batch, zeta_next_batch],
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::types::Sample;
    use crate::fri::reduction_strategies::FriReductionStrategy;
    use crate::gates::noop::NoopGate;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::plonk::verifier::verify;

    #[test]
    fn test_proof_compression() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let mut config = CircuitConfig::standard_recursion_config();
        config.fri_config.reduction_strategy = FriReductionStrategy::Fixed(vec![1, 1]);
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
        for _ in 0..100 {
            builder.add_gate(NoopGate, vec![]);
        }
        let data = builder.build::<C>();
        let proof = data.prove(pw)?;
        verify(proof.clone(), &data.verifier_only, &data.common)?;

        // Verify that `decompress ∘ compress = identity`.
        let compressed_proof = data.compress(proof.clone())?;
        let decompressed_compressed_proof = data.decompress(compressed_proof.clone())?;
        assert_eq!(proof, decompressed_compressed_proof);

        verify(proof, &data.verifier_only, &data.common)?;
        data.verify_compressed(compressed_proof)
    }
}
