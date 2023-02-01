use alloc::vec;
use alloc::vec::Vec;
use core::convert::Infallible;
use core::fmt::{Debug, Display, Formatter};
use core::mem::size_of;

use hashbrown::HashMap;

use crate::field::extension::{Extendable, FieldExtension};
use crate::field::polynomial::PolynomialCoeffs;
use crate::field::types::{Field64, PrimeField64};
use crate::fri::proof::{
    CompressedFriProof, CompressedFriQueryRounds, FriInitialTreeProof, FriProof, FriQueryRound,
    FriQueryStep,
};
use crate::hash::hash_types::RichField;
use crate::hash::merkle_proofs::MerkleProof;
use crate::hash::merkle_tree::MerkleCap;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::config::{GenericConfig, GenericHashOut, Hasher};
use crate::plonk::plonk_common::salt_size;
use crate::plonk::proof::{
    CompressedProof, CompressedProofWithPublicInputs, OpeningSet, Proof, ProofWithPublicInputs,
};

/// A no_std compatible variant of `std::io::Error`
#[derive(Debug)]
pub struct IoError;

impl Display for IoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }
}

/// A no_std compatible variant of `std::io::Result`
pub type IoResult<T> = Result<T, IoError>;

/// A `Read` which is able to report how many bytes are remaining.
pub trait Remaining: Read {
    /// Returns the number of bytes remaining in the buffer.
    fn remaining(&self) -> usize;

    /// Returns whether zero bytes are remaining.
    fn is_empty(&self) -> bool {
        self.remaining() == 0
    }
}

/// Similar to `std::io::Read`, but works with no_std.
pub trait Read {
    /// Reads exactly the length of `bytes` from `self` and writes it to `bytes`.
    fn read_exact(&mut self, bytes: &mut [u8]) -> IoResult<()>;

    /// Reads a `u8` value from `self`.
    #[inline]
    fn read_u8(&mut self) -> IoResult<u8> {
        let mut buf = [0; size_of::<u8>()];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Reads a `u32` value from `self`.
    #[inline]
    fn read_u32(&mut self) -> IoResult<u32> {
        let mut buf = [0; size_of::<u32>()];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Reads a element from the field `F` with size less than `2^64` from `self.`
    #[inline]
    fn read_field<F>(&mut self) -> IoResult<F>
    where
        F: Field64,
    {
        let mut buf = [0; size_of::<u64>()];
        self.read_exact(&mut buf)?;
        Ok(F::from_canonical_u64(u64::from_le_bytes(buf)))
    }

    /// Reads a vector of elements from the field `F` from `self`.
    #[inline]
    fn read_field_vec<F>(&mut self, length: usize) -> IoResult<Vec<F>>
    where
        F: Field64,
    {
        (0..length)
            .map(|_| self.read_field())
            .collect::<Result<Vec<_>, _>>()
    }

    /// Reads an element from the field extension of `F` from `self.`
    #[inline]
    fn read_field_ext<F, const D: usize>(&mut self) -> IoResult<F::Extension>
    where
        F: Field64 + Extendable<D>,
    {
        let mut arr = [F::ZERO; D];
        for a in arr.iter_mut() {
            *a = self.read_field()?;
        }
        Ok(<F::Extension as FieldExtension<D>>::from_basefield_array(
            arr,
        ))
    }

    /// Reads a vector of elements from the field extension of `F` from `self`.
    #[inline]
    fn read_field_ext_vec<F, const D: usize>(
        &mut self,
        length: usize,
    ) -> IoResult<Vec<F::Extension>>
    where
        F: RichField + Extendable<D>,
    {
        (0..length).map(|_| self.read_field_ext::<F, D>()).collect()
    }

    /// Reads a hash value from `self`.
    #[inline]
    fn read_hash<F, H>(&mut self) -> IoResult<H::Hash>
    where
        F: RichField,
        H: Hasher<F>,
    {
        let mut buf = vec![0; H::HASH_SIZE];
        self.read_exact(&mut buf)?;
        Ok(H::Hash::from_bytes(&buf))
    }

    /// Reads a value of type [`MerkleCap`] from `self` with the given `cap_height`.
    #[inline]
    fn read_merkle_cap<F, H>(&mut self, cap_height: usize) -> IoResult<MerkleCap<F, H>>
    where
        F: RichField,
        H: Hasher<F>,
    {
        let cap_length = 1 << cap_height;
        Ok(MerkleCap(
            (0..cap_length)
                .map(|_| self.read_hash::<F, H>())
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }

    /// Reads a value of type [`OpeningSet`] from `self` with the given `common_data`.
    #[inline]
    fn read_opening_set<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<OpeningSet<F, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let constants = self.read_field_ext_vec::<F, D>(common_data.num_constants)?;
        let plonk_sigmas = self.read_field_ext_vec::<F, D>(config.num_routed_wires)?;
        let wires = self.read_field_ext_vec::<F, D>(config.num_wires)?;
        let plonk_zs = self.read_field_ext_vec::<F, D>(config.num_challenges)?;
        let plonk_zs_next = self.read_field_ext_vec::<F, D>(config.num_challenges)?;
        let partial_products = self
            .read_field_ext_vec::<F, D>(common_data.num_partial_products * config.num_challenges)?;
        let quotient_polys = self.read_field_ext_vec::<F, D>(
            common_data.quotient_degree_factor * config.num_challenges,
        )?;
        Ok(OpeningSet {
            constants,
            plonk_sigmas,
            wires,
            plonk_zs,
            plonk_zs_next,
            partial_products,
            quotient_polys,
        })
    }

    /// Reads a value of type [`MerkleProof`] from `self`.
    #[inline]
    fn read_merkle_proof<F, H>(&mut self) -> IoResult<MerkleProof<F, H>>
    where
        F: RichField,
        H: Hasher<F>,
    {
        let length = self.read_u8()?;
        Ok(MerkleProof {
            siblings: (0..length)
                .map(|_| self.read_hash::<F, H>())
                .collect::<Result<_, _>>()?,
        })
    }

    /// Reads a value of type [`FriInitialTreeProof`] from `self` with the given `common_data`.
    #[inline]
    fn read_fri_initial_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<FriInitialTreeProof<F, C::Hasher>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let salt = salt_size(common_data.fri_params.hiding);
        let mut evals_proofs = Vec::with_capacity(4);

        let constants_sigmas_v =
            self.read_field_vec(common_data.num_constants + config.num_routed_wires)?;
        let constants_sigmas_p = self.read_merkle_proof()?;
        evals_proofs.push((constants_sigmas_v, constants_sigmas_p));

        let wires_v = self.read_field_vec(config.num_wires + salt)?;
        let wires_p = self.read_merkle_proof()?;
        evals_proofs.push((wires_v, wires_p));

        let zs_partial_v = self.read_field_vec(
            config.num_challenges * (1 + common_data.num_partial_products) + salt,
        )?;
        let zs_partial_p = self.read_merkle_proof()?;
        evals_proofs.push((zs_partial_v, zs_partial_p));

        let quotient_v =
            self.read_field_vec(config.num_challenges * common_data.quotient_degree_factor + salt)?;
        let quotient_p = self.read_merkle_proof()?;
        evals_proofs.push((quotient_v, quotient_p));

        Ok(FriInitialTreeProof { evals_proofs })
    }

    /// Reads a value of type [`FriQueryStep`] from `self` with the given `arity` and `compressed`
    /// flag.
    #[inline]
    fn read_fri_query_step<F, C, const D: usize>(
        &mut self,
        arity: usize,
        compressed: bool,
    ) -> IoResult<FriQueryStep<F, C::Hasher, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let evals = self.read_field_ext_vec::<F, D>(arity - usize::from(compressed))?;
        let merkle_proof = self.read_merkle_proof()?;
        Ok(FriQueryStep {
            evals,
            merkle_proof,
        })
    }

    /// Reads a vector of [`FriQueryRound`]s from `self` with `common_data`.
    #[inline]
    fn read_fri_query_rounds<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<Vec<FriQueryRound<F, C::Hasher, D>>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let mut fqrs = Vec::with_capacity(config.fri_config.num_query_rounds);
        for _ in 0..config.fri_config.num_query_rounds {
            let initial_trees_proof = self.read_fri_initial_proof::<F, C, D>(common_data)?;
            let steps = common_data
                .fri_params
                .reduction_arity_bits
                .iter()
                .map(|&ar| self.read_fri_query_step::<F, C, D>(1 << ar, false))
                .collect::<Result<_, _>>()?;
            fqrs.push(FriQueryRound {
                initial_trees_proof,
                steps,
            })
        }
        Ok(fqrs)
    }

    /// Reads a value of type [`FriProof`] from `self` with `common_data`.
    #[inline]
    fn read_fri_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<FriProof<F, C::Hasher, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let commit_phase_merkle_caps = (0..common_data.fri_params.reduction_arity_bits.len())
            .map(|_| self.read_merkle_cap(config.fri_config.cap_height))
            .collect::<Result<Vec<_>, _>>()?;
        let query_round_proofs = self.read_fri_query_rounds::<F, C, D>(common_data)?;
        let final_poly = PolynomialCoeffs::new(
            self.read_field_ext_vec::<F, D>(common_data.fri_params.final_poly_len())?,
        );
        let pow_witness = self.read_field()?;
        Ok(FriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
        })
    }

    /// Reads a value of type [`Proof`] from `self` with `common_data`.
    #[inline]
    fn read_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<Proof<F, C, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let wires_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let plonk_zs_partial_products_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let quotient_polys_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let openings = self.read_opening_set::<F, C, D>(common_data)?;
        let opening_proof = self.read_fri_proof::<F, C, D>(common_data)?;
        Ok(Proof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        })
    }

    /// Reads a value of type [`ProofWithPublicInputs`] from `self` with `common_data`.
    #[inline]
    fn read_proof_with_public_inputs<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<ProofWithPublicInputs<F, C, D>>
    where
        Self: Remaining,
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let proof = self.read_proof(common_data)?;
        let public_inputs = self.read_field_vec(self.remaining() / size_of::<u64>())?;
        Ok(ProofWithPublicInputs {
            proof,
            public_inputs,
        })
    }

    /// Reads a value of type [`CompressedFriQueryRounds`] from `self` with `common_data`.
    #[inline]
    fn read_compressed_fri_query_rounds<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<CompressedFriQueryRounds<F, C::Hasher, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let original_indices = (0..config.fri_config.num_query_rounds)
            .map(|_| self.read_u32().map(|i| i as usize))
            .collect::<Result<Vec<_>, _>>()?;
        let mut indices = original_indices.clone();
        indices.sort_unstable();
        indices.dedup();
        let mut pairs = Vec::new();
        for &i in &indices {
            pairs.push((i, self.read_fri_initial_proof::<F, C, D>(common_data)?));
        }
        let initial_trees_proofs = HashMap::from_iter(pairs);

        let mut steps = Vec::with_capacity(common_data.fri_params.reduction_arity_bits.len());
        for &a in &common_data.fri_params.reduction_arity_bits {
            indices.iter_mut().for_each(|x| {
                *x >>= a;
            });
            indices.dedup();
            let query_steps = (0..indices.len())
                .map(|_| self.read_fri_query_step::<F, C, D>(1 << a, true))
                .collect::<Result<Vec<_>, _>>()?;
            steps.push(
                indices
                    .iter()
                    .copied()
                    .zip(query_steps)
                    .collect::<HashMap<_, _>>(),
            );
        }

        Ok(CompressedFriQueryRounds {
            indices: original_indices,
            initial_trees_proofs,
            steps,
        })
    }

    /// Reads a value of type [`CompressedFriProof`] from `self` with `common_data`.
    #[inline]
    fn read_compressed_fri_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<CompressedFriProof<F, C::Hasher, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let commit_phase_merkle_caps = (0..common_data.fri_params.reduction_arity_bits.len())
            .map(|_| self.read_merkle_cap(config.fri_config.cap_height))
            .collect::<Result<Vec<_>, _>>()?;
        let query_round_proofs = self.read_compressed_fri_query_rounds::<F, C, D>(common_data)?;
        let final_poly = PolynomialCoeffs::new(
            self.read_field_ext_vec::<F, D>(common_data.fri_params.final_poly_len())?,
        );
        let pow_witness = self.read_field()?;
        Ok(CompressedFriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
        })
    }

    /// Reads a value of type [`CompressedProof`] from `self` with `common_data`.
    #[inline]
    fn read_compressed_proof<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<CompressedProof<F, C, D>>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let config = &common_data.config;
        let wires_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let plonk_zs_partial_products_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let quotient_polys_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let openings = self.read_opening_set::<F, C, D>(common_data)?;
        let opening_proof = self.read_compressed_fri_proof::<F, C, D>(common_data)?;
        Ok(CompressedProof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        })
    }

    /// Reads a value of type [`CompressedProofWithPublicInputs`] from `self` with `common_data`.
    #[inline]
    fn read_compressed_proof_with_public_inputs<F, C, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<CompressedProofWithPublicInputs<F, C, D>>
    where
        Self: Remaining,
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let proof = self.read_compressed_proof(common_data)?;
        let public_inputs = self.read_field_vec(self.remaining() / size_of::<u64>())?;
        Ok(CompressedProofWithPublicInputs {
            proof,
            public_inputs,
        })
    }
}

/// Writing
pub trait Write {
    /// Error Type
    type Error;

    /// Writes all `bytes` to `self`.
    fn write_all(&mut self, bytes: &[u8]) -> IoResult<()>;

    /// Writes a byte `x` to `self`.
    #[inline]
    fn write_u8(&mut self, x: u8) -> IoResult<()> {
        self.write_all(&[x])
    }

    /// Writes a word `x` to `self.`
    #[inline]
    fn write_u32(&mut self, x: u32) -> IoResult<()> {
        self.write_all(&x.to_le_bytes())
    }

    /// Writes an element `x` from the field `F` to `self`.
    #[inline]
    fn write_field<F>(&mut self, x: F) -> IoResult<()>
    where
        F: PrimeField64,
    {
        self.write_all(&x.to_canonical_u64().to_le_bytes())
    }

    /// Writes a vector `v` of elements from the field `F` to `self`.
    #[inline]
    fn write_field_vec<F>(&mut self, v: &[F]) -> IoResult<()>
    where
        F: PrimeField64,
    {
        for &a in v {
            self.write_field(a)?;
        }
        Ok(())
    }

    /// Writes an element `x` from the field extension of `F` to `self`.
    #[inline]
    fn write_field_ext<F, const D: usize>(&mut self, x: F::Extension) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
    {
        for &a in &x.to_basefield_array() {
            self.write_field(a)?;
        }
        Ok(())
    }

    /// Writes a vector `v` of elements from the field extension of `F` to `self`.
    #[inline]
    fn write_field_ext_vec<F, const D: usize>(&mut self, v: &[F::Extension]) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
    {
        for &a in v {
            self.write_field_ext::<F, D>(a)?;
        }
        Ok(())
    }

    /// Writes a hash `h` to `self`.
    #[inline]
    fn write_hash<F, H>(&mut self, h: H::Hash) -> IoResult<()>
    where
        F: RichField,
        H: Hasher<F>,
    {
        self.write_all(&h.to_bytes())
    }

    /// Writes `cap`, a value of type [`MerkleCap`], to `self`.
    #[inline]
    fn write_merkle_cap<F, H>(&mut self, cap: &MerkleCap<F, H>) -> IoResult<()>
    where
        F: RichField,
        H: Hasher<F>,
    {
        for &a in &cap.0 {
            self.write_hash::<F, H>(a)?;
        }
        Ok(())
    }

    /// Writes a value `os` of type [`OpeningSet`] to `self.`
    #[inline]
    fn write_opening_set<F, const D: usize>(&mut self, os: &OpeningSet<F, D>) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
    {
        self.write_field_ext_vec::<F, D>(&os.constants)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_sigmas)?;
        self.write_field_ext_vec::<F, D>(&os.wires)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_zs)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_zs_next)?;
        self.write_field_ext_vec::<F, D>(&os.partial_products)?;
        self.write_field_ext_vec::<F, D>(&os.quotient_polys)
    }

    /// Writes a value `p` of type [`MerkleProof`] to `self.`
    #[inline]
    fn write_merkle_proof<F, H>(&mut self, p: &MerkleProof<F, H>) -> IoResult<()>
    where
        F: RichField,
        H: Hasher<F>,
    {
        let length = p.siblings.len();
        self.write_u8(
            length
                .try_into()
                .expect("Merkle proof length must fit in u8."),
        )?;
        for &h in &p.siblings {
            self.write_hash::<F, H>(h)?;
        }
        Ok(())
    }

    /// Writes a value `fitp` of type [`FriInitialTreeProof`] to `self.`
    #[inline]
    fn write_fri_initial_proof<F, C, const D: usize>(
        &mut self,
        fitp: &FriInitialTreeProof<F, C::Hasher>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for (v, p) in &fitp.evals_proofs {
            self.write_field_vec(v)?;
            self.write_merkle_proof(p)?;
        }
        Ok(())
    }

    /// Writes a value `fqs` of type [`FriQueryStep`] to `self.`
    #[inline]
    fn write_fri_query_step<F, C, const D: usize>(
        &mut self,
        fqs: &FriQueryStep<F, C::Hasher, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        self.write_field_ext_vec::<F, D>(&fqs.evals)?;
        self.write_merkle_proof(&fqs.merkle_proof)
    }

    /// Writes a value `fqrs` of type [`FriQueryRound`] to `self.`
    #[inline]
    fn write_fri_query_rounds<F, C, const D: usize>(
        &mut self,
        fqrs: &[FriQueryRound<F, C::Hasher, D>],
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for fqr in fqrs {
            self.write_fri_initial_proof::<F, C, D>(&fqr.initial_trees_proof)?;
            for fqs in &fqr.steps {
                self.write_fri_query_step::<F, C, D>(fqs)?;
            }
        }
        Ok(())
    }

    /// Writes a value `fq` of type [`FriProof`] to `self.`
    #[inline]
    fn write_fri_proof<F, C, const D: usize>(
        &mut self,
        fp: &FriProof<F, C::Hasher, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for cap in &fp.commit_phase_merkle_caps {
            self.write_merkle_cap(cap)?;
        }
        self.write_fri_query_rounds::<F, C, D>(&fp.query_round_proofs)?;
        self.write_field_ext_vec::<F, D>(&fp.final_poly.coeffs)?;
        self.write_field(fp.pow_witness)
    }

    /// Writes a value `proof` of type [`Proof`] to `self.`
    #[inline]
    fn write_proof<F, C, const D: usize>(&mut self, proof: &Proof<F, C, D>) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        self.write_merkle_cap(&proof.wires_cap)?;
        self.write_merkle_cap(&proof.plonk_zs_partial_products_cap)?;
        self.write_merkle_cap(&proof.quotient_polys_cap)?;
        self.write_opening_set(&proof.openings)?;
        self.write_fri_proof::<F, C, D>(&proof.opening_proof)
    }

    /// Writes a value `proof_with_pis` of type [`ProofWithPublicInputs`] to `self.`
    #[inline]
    fn write_proof_with_public_inputs<F, C, const D: usize>(
        &mut self,
        proof_with_pis: &ProofWithPublicInputs<F, C, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let ProofWithPublicInputs {
            proof,
            public_inputs,
        } = proof_with_pis;
        self.write_proof(proof)?;
        self.write_field_vec(public_inputs)
    }

    /// Writes a value `cfqrs` of type [`CompressedFriQueryRounds`] to `self.`
    #[inline]
    fn write_compressed_fri_query_rounds<F, C, const D: usize>(
        &mut self,
        cfqrs: &CompressedFriQueryRounds<F, C::Hasher, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for &i in &cfqrs.indices {
            self.write_u32(i as u32)?;
        }
        let mut initial_trees_proofs = cfqrs.initial_trees_proofs.iter().collect::<Vec<_>>();
        initial_trees_proofs.sort_by_key(|&x| x.0);
        for (_, itp) in initial_trees_proofs {
            self.write_fri_initial_proof::<F, C, D>(itp)?;
        }
        for h in &cfqrs.steps {
            let mut fri_query_steps = h.iter().collect::<Vec<_>>();
            fri_query_steps.sort_by_key(|&x| x.0);
            for (_, fqs) in fri_query_steps {
                self.write_fri_query_step::<F, C, D>(fqs)?;
            }
        }
        Ok(())
    }

    /// Writes a value `fq` of type [`CompressedFriProof`] to `self.`
    #[inline]
    fn write_compressed_fri_proof<F, C, const D: usize>(
        &mut self,
        fp: &CompressedFriProof<F, C::Hasher, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        for cap in &fp.commit_phase_merkle_caps {
            self.write_merkle_cap(cap)?;
        }
        self.write_compressed_fri_query_rounds::<F, C, D>(&fp.query_round_proofs)?;
        self.write_field_ext_vec::<F, D>(&fp.final_poly.coeffs)?;
        self.write_field(fp.pow_witness)
    }

    /// Writes a value `proof` of type [`CompressedProof`] to `self.`
    #[inline]
    fn write_compressed_proof<F, C, const D: usize>(
        &mut self,
        proof: &CompressedProof<F, C, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        self.write_merkle_cap(&proof.wires_cap)?;
        self.write_merkle_cap(&proof.plonk_zs_partial_products_cap)?;
        self.write_merkle_cap(&proof.quotient_polys_cap)?;
        self.write_opening_set(&proof.openings)?;
        self.write_compressed_fri_proof::<F, C, D>(&proof.opening_proof)
    }

    /// Writes a value `proof_with_pis` of type [`CompressedProofWithPublicInputs`] to `self.`
    #[inline]
    fn write_compressed_proof_with_public_inputs<F, C, const D: usize>(
        &mut self,
        proof_with_pis: &CompressedProofWithPublicInputs<F, C, D>,
    ) -> IoResult<()>
    where
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
    {
        let CompressedProofWithPublicInputs {
            proof,
            public_inputs,
        } = proof_with_pis;
        self.write_compressed_proof(proof)?;
        self.write_field_vec(public_inputs)
    }
}

impl Write for Vec<u8> {
    type Error = Infallible;

    #[inline]
    fn write_all(&mut self, bytes: &[u8]) -> IoResult<()> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

/// Buffer
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct Buffer {
    bytes: Vec<u8>,
    pos: usize,
}

#[cfg(feature = "std")]
impl Buffer {
    /// Builds a new [`Buffer`] over `buffer`.
    #[inline]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Returns the inner buffer.
    #[inline]
    pub fn bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(feature = "std")]
impl Remaining for Buffer {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }
}

#[cfg(feature = "std")]
impl Read for Buffer {
    #[inline]
    fn read_exact(&mut self, bytes: &mut [u8]) -> IoResult<()> {
        let n = bytes.len();
        if self.remaining() < n {
            Err(IoError)
        } else {
            bytes.copy_from_slice(&self.bytes[self.pos..][..n]);
            self.pos += n;
            Ok(())
        }
    }
}
