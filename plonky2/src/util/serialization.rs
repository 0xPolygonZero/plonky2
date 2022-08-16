use std::collections::HashMap;
use std::io::Cursor;
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::ops::Range;

use plonky2_field::extension::{Extendable, FieldExtension};
use plonky2_field::polynomial::PolynomialCoeffs;
use plonky2_field::types::{Field64, PrimeField64};

use crate::fri::proof::{
    CompressedFriProof, CompressedFriQueryRounds, FriInitialTreeProof, FriProof, FriQueryRound,
    FriQueryStep,
};
use crate::fri::reduction_strategies::FriReductionStrategy;
use crate::fri::{FriConfig, FriParams};
use crate::gates::gate::GateRef;
use crate::gates::selectors::SelectorsInfo;
use crate::hash::hash_types::RichField;
use crate::hash::merkle_proofs::MerkleProof;
use crate::hash::merkle_tree::MerkleCap;
use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData};
use crate::plonk::config::{GenericConfig, GenericHashOut, Hasher};
use crate::plonk::plonk_common::salt_size;
use crate::plonk::proof::{
    CompressedProof, CompressedProofWithPublicInputs, OpeningSet, Proof, ProofWithPublicInputs,
};
use crate::util::gate_serialization::GateSerializer;

#[derive(Debug)]
pub struct Buffer(Cursor<Vec<u8>>);

impl Buffer {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self(Cursor::new(buffer))
    }

    pub fn len(&self) -> usize {
        self.0.get_ref().len()
    }

    pub fn bytes(self) -> Vec<u8> {
        self.0.into_inner()
    }

    pub(crate) fn write_bool(&mut self, x: bool) -> Result<()> {
        self.write_u8(u8::from(x))
    }
    pub(crate) fn read_bool(&mut self) -> Result<bool> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::from(ErrorKind::InvalidData)),
        }
    }

    fn write_u8(&mut self, x: u8) -> Result<()> {
        self.0.write_all(&[x])
    }
    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0; std::mem::size_of::<u8>()];
        self.0.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub(crate) fn write_u32(&mut self, x: u32) -> Result<()> {
        self.0.write_all(&x.to_le_bytes())
    }
    pub(crate) fn read_u32(&mut self) -> Result<u32> {
        let mut buf = [0; std::mem::size_of::<u32>()];
        self.0.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn write_usize(&mut self, x: usize) -> Result<()> {
        self.0.write_all(&(x as u64).to_le_bytes())
    }
    pub fn read_usize(&mut self) -> Result<usize> {
        let mut buf = [0; std::mem::size_of::<u64>()];
        self.0.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf) as usize)
    }

    fn write_field<F: PrimeField64>(&mut self, x: F) -> Result<()> {
        self.0.write_all(&x.to_canonical_u64().to_le_bytes())
    }
    fn read_field<F: Field64>(&mut self) -> Result<F> {
        let mut buf = [0; std::mem::size_of::<u64>()];
        self.0.read_exact(&mut buf)?;
        Ok(F::from_canonical_u64(u64::from_le_bytes(
            buf.try_into().unwrap(),
        )))
    }

    fn write_field_ext<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        x: F::Extension,
    ) -> Result<()> {
        for &a in &x.to_basefield_array() {
            self.write_field(a)?;
        }
        Ok(())
    }
    fn read_field_ext<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
    ) -> Result<F::Extension> {
        let mut arr = [F::ZERO; D];
        for a in arr.iter_mut() {
            *a = self.read_field()?;
        }
        Ok(<F::Extension as FieldExtension<D>>::from_basefield_array(
            arr,
        ))
    }

    fn write_hash<F: RichField, H: Hasher<F>>(&mut self, h: H::Hash) -> Result<()> {
        self.0.write_all(&h.to_bytes())
    }

    fn read_hash<F: RichField, H: Hasher<F>>(&mut self) -> Result<H::Hash> {
        let mut buf = vec![0; H::HASH_SIZE];
        self.0.read_exact(&mut buf)?;
        Ok(H::Hash::from_bytes(&buf))
    }

    fn write_merkle_cap<F: RichField, H: Hasher<F>>(
        &mut self,
        cap: &MerkleCap<F, H>,
    ) -> Result<()> {
        for &a in &cap.0 {
            self.write_hash::<F, H>(a)?;
        }
        Ok(())
    }
    fn read_merkle_cap<F: RichField, H: Hasher<F>>(
        &mut self,
        cap_height: usize,
    ) -> Result<MerkleCap<F, H>> {
        let cap_length = 1 << cap_height;
        Ok(MerkleCap(
            (0..cap_length)
                .map(|_| self.read_hash::<F, H>())
                .collect::<Result<Vec<_>>>()?,
        ))
    }

    pub fn write_usize_vec(&mut self, v: &[usize]) -> Result<()> {
        self.write_usize(v.len())?;
        for &elem in v.iter() {
            self.write_usize(elem)?;
        }

        Ok(())
    }
    pub fn read_usize_vec(&mut self) -> Result<Vec<usize>> {
        let len = self.read_usize()?;
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            res.push(self.read_usize()?);
        }

        Ok(res)
    }

    pub fn write_field_vec<F: PrimeField64>(&mut self, v: &[F]) -> Result<()> {
        for &a in v {
            self.write_field(a)?;
        }
        Ok(())
    }
    pub fn read_field_vec<F: Field64>(&mut self, length: usize) -> Result<Vec<F>> {
        (0..length)
            .map(|_| self.read_field())
            .collect::<Result<Vec<_>>>()
    }

    fn write_field_ext_vec<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        v: &[F::Extension],
    ) -> Result<()> {
        for &a in v {
            self.write_field_ext::<F, D>(a)?;
        }
        Ok(())
    }
    fn read_field_ext_vec<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        length: usize,
    ) -> Result<Vec<F::Extension>> {
        (0..length)
            .map(|_| self.read_field_ext::<F, D>())
            .collect::<Result<Vec<_>>>()
    }

    fn write_opening_set<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        os: &OpeningSet<F, D>,
    ) -> Result<()> {
        self.write_field_ext_vec::<F, D>(&os.constants)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_sigmas)?;
        self.write_field_ext_vec::<F, D>(&os.wires)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_zs)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_zs_next)?;
        self.write_field_ext_vec::<F, D>(&os.partial_products)?;
        self.write_field_ext_vec::<F, D>(&os.quotient_polys)
    }
    fn read_opening_set<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<OpeningSet<F, D>> {
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

    fn write_merkle_proof<F: RichField, H: Hasher<F>>(
        &mut self,
        p: &MerkleProof<F, H>,
    ) -> Result<()> {
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
    fn read_merkle_proof<F: RichField, H: Hasher<F>>(&mut self) -> Result<MerkleProof<F, H>> {
        let length = self.read_u8()?;
        Ok(MerkleProof {
            siblings: (0..length)
                .map(|_| self.read_hash::<F, H>())
                .collect::<Result<Vec<_>>>()?,
        })
    }

    fn write_fri_initial_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        fitp: &FriInitialTreeProof<F, C::Hasher>,
    ) -> Result<()> {
        for (v, p) in &fitp.evals_proofs {
            self.write_field_vec(v)?;
            self.write_merkle_proof(p)?;
        }
        Ok(())
    }
    fn read_fri_initial_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<FriInitialTreeProof<F, C::Hasher>> {
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

    fn write_fri_query_step<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        fqs: &FriQueryStep<F, C::Hasher, D>,
    ) -> Result<()> {
        self.write_field_ext_vec::<F, D>(&fqs.evals)?;
        self.write_merkle_proof(&fqs.merkle_proof)
    }
    fn read_fri_query_step<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        arity: usize,
        compressed: bool,
    ) -> Result<FriQueryStep<F, C::Hasher, D>> {
        let evals = self.read_field_ext_vec::<F, D>(arity - if compressed { 1 } else { 0 })?;
        let merkle_proof = self.read_merkle_proof()?;
        Ok(FriQueryStep {
            evals,
            merkle_proof,
        })
    }

    fn write_fri_query_rounds<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        fqrs: &[FriQueryRound<F, C::Hasher, D>],
    ) -> Result<()> {
        for fqr in fqrs {
            self.write_fri_initial_proof::<F, C, D>(&fqr.initial_trees_proof)?;
            for fqs in &fqr.steps {
                self.write_fri_query_step::<F, C, D>(fqs)?;
            }
        }
        Ok(())
    }
    fn read_fri_query_rounds<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<Vec<FriQueryRound<F, C::Hasher, D>>> {
        let config = &common_data.config;
        let mut fqrs = Vec::with_capacity(config.fri_config.num_query_rounds);
        for _ in 0..config.fri_config.num_query_rounds {
            let initial_trees_proof = self.read_fri_initial_proof(common_data)?;
            let steps = common_data
                .fri_params
                .reduction_arity_bits
                .iter()
                .map(|&ar| self.read_fri_query_step::<F, C, D>(1 << ar, false))
                .collect::<Result<_>>()?;
            fqrs.push(FriQueryRound {
                initial_trees_proof,
                steps,
            })
        }
        Ok(fqrs)
    }

    fn write_fri_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
        fp: &FriProof<F, C::Hasher, D>,
    ) -> Result<()> {
        for cap in &fp.commit_phase_merkle_caps {
            self.write_merkle_cap(cap)?;
        }
        self.write_fri_query_rounds::<F, C, D>(&fp.query_round_proofs)?;
        self.write_field_ext_vec::<F, D>(&fp.final_poly.coeffs)?;
        self.write_field(fp.pow_witness)
    }
    fn read_fri_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<FriProof<F, C::Hasher, D>> {
        let config = &common_data.config;
        let commit_phase_merkle_caps = (0..common_data.fri_params.reduction_arity_bits.len())
            .map(|_| self.read_merkle_cap(config.fri_config.cap_height))
            .collect::<Result<Vec<_>>>()?;
        let query_round_proofs = self.read_fri_query_rounds(common_data)?;
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

    pub fn write_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
        proof: &Proof<F, C, D>,
    ) -> Result<()> {
        self.write_merkle_cap(&proof.wires_cap)?;
        self.write_merkle_cap(&proof.plonk_zs_partial_products_cap)?;
        self.write_merkle_cap(&proof.quotient_polys_cap)?;
        self.write_opening_set(&proof.openings)?;
        self.write_fri_proof::<F, C, D>(&proof.opening_proof)
    }
    pub fn read_proof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<Proof<F, C, D>> {
        let config = &common_data.config;
        let wires_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let plonk_zs_partial_products_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let quotient_polys_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let openings = self.read_opening_set(common_data)?;
        let opening_proof = self.read_fri_proof(common_data)?;

        Ok(Proof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        })
    }

    pub fn write_proof_with_public_inputs<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        proof_with_pis: &ProofWithPublicInputs<F, C, D>,
    ) -> Result<()> {
        let ProofWithPublicInputs {
            proof,
            public_inputs,
        } = proof_with_pis;
        self.write_proof(proof)?;
        self.write_field_vec(public_inputs)
    }
    pub fn read_proof_with_public_inputs<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>> {
        let proof = self.read_proof(common_data)?;
        let public_inputs = self.read_field_vec(
            (self.len() - self.0.position() as usize) / std::mem::size_of::<u64>(),
        )?;

        Ok(ProofWithPublicInputs {
            proof,
            public_inputs,
        })
    }

    fn write_compressed_fri_query_rounds<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        cfqrs: &CompressedFriQueryRounds<F, C::Hasher, D>,
    ) -> Result<()> {
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
    fn read_compressed_fri_query_rounds<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<CompressedFriQueryRounds<F, C::Hasher, D>> {
        let config = &common_data.config;
        let original_indices = (0..config.fri_config.num_query_rounds)
            .map(|_| self.read_u32().map(|i| i as usize))
            .collect::<Result<Vec<_>>>()?;
        let mut indices = original_indices.clone();
        indices.sort_unstable();
        indices.dedup();
        let mut pairs = Vec::new();
        for &i in &indices {
            pairs.push((i, self.read_fri_initial_proof(common_data)?));
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
                .collect::<Result<Vec<_>>>()?;
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

    fn write_compressed_fri_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        fp: &CompressedFriProof<F, C::Hasher, D>,
    ) -> Result<()> {
        for cap in &fp.commit_phase_merkle_caps {
            self.write_merkle_cap(cap)?;
        }
        self.write_compressed_fri_query_rounds::<F, C, D>(&fp.query_round_proofs)?;
        self.write_field_ext_vec::<F, D>(&fp.final_poly.coeffs)?;
        self.write_field(fp.pow_witness)
    }
    fn read_compressed_fri_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<CompressedFriProof<F, C::Hasher, D>> {
        let config = &common_data.config;
        let commit_phase_merkle_caps = (0..common_data.fri_params.reduction_arity_bits.len())
            .map(|_| self.read_merkle_cap(config.fri_config.cap_height))
            .collect::<Result<Vec<_>>>()?;
        let query_round_proofs = self.read_compressed_fri_query_rounds(common_data)?;
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

    pub fn write_compressed_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        proof: &CompressedProof<F, C, D>,
    ) -> Result<()> {
        self.write_merkle_cap(&proof.wires_cap)?;
        self.write_merkle_cap(&proof.plonk_zs_partial_products_cap)?;
        self.write_merkle_cap(&proof.quotient_polys_cap)?;
        self.write_opening_set(&proof.openings)?;
        self.write_compressed_fri_proof::<F, C, D>(&proof.opening_proof)
    }
    pub fn read_compressed_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<CompressedProof<F, C, D>> {
        let config = &common_data.config;
        let wires_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let plonk_zs_partial_products_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let quotient_polys_cap = self.read_merkle_cap(config.fri_config.cap_height)?;
        let openings = self.read_opening_set(common_data)?;
        let opening_proof = self.read_compressed_fri_proof(common_data)?;

        Ok(CompressedProof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        })
    }

    pub fn write_compressed_proof_with_public_inputs<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        proof_with_pis: &CompressedProofWithPublicInputs<F, C, D>,
    ) -> Result<()> {
        let CompressedProofWithPublicInputs {
            proof,
            public_inputs,
        } = proof_with_pis;
        self.write_compressed_proof(proof)?;
        self.write_field_vec(public_inputs)
    }
    pub fn read_compressed_proof_with_public_inputs<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<CompressedProofWithPublicInputs<F, C, D>> {
        let proof = self.read_compressed_proof(common_data)?;
        let public_inputs = self.read_field_vec(
            (self.len() - self.0.position() as usize) / std::mem::size_of::<u64>(),
        )?;

        Ok(CompressedProofWithPublicInputs {
            proof,
            public_inputs,
        })
    }

    // circuit data serialization

    pub fn write_fri_reduction_strategy(
        &mut self,
        reduction_strategy: &FriReductionStrategy,
    ) -> Result<()> {
        match reduction_strategy {
            FriReductionStrategy::Fixed(seq) => {
                self.write_u8(0)?;
                self.write_usize_vec(seq.as_slice())?;

                Ok(())
            }
            FriReductionStrategy::ConstantArityBits(arity_bits, final_poly_bits) => {
                self.write_u8(1)?;
                self.write_usize(*arity_bits)?;
                self.write_usize(*final_poly_bits)?;

                Ok(())
            }
            FriReductionStrategy::MinSize(max) => {
                self.write_u8(2)?;
                if let Some(max) = max {
                    self.write_u8(1)?;
                    self.write_usize(*max)?;
                } else {
                    self.write_u8(0)?;
                }

                Ok(())
            }
        }
    }
    pub fn read_fri_reduction_strategy(&mut self) -> Result<FriReductionStrategy> {
        let variant = self.read_u8()?;
        match variant {
            0 => {
                let arities = self.read_usize_vec()?;
                Ok(FriReductionStrategy::Fixed(arities))
            }
            1 => {
                let arity_bits = self.read_usize()?;
                let final_poly_bits = self.read_usize()?;

                Ok(FriReductionStrategy::ConstantArityBits(
                    arity_bits,
                    final_poly_bits,
                ))
            }
            2 => {
                let is_some = self.read_u8()?;
                match is_some {
                    0 => Ok(FriReductionStrategy::MinSize(None)),
                    1 => {
                        let max = self.read_usize()?;
                        Ok(FriReductionStrategy::MinSize(Some(max)))
                    }
                    _ => Err(Error::from(ErrorKind::InvalidData)),
                }
            }
            _ => Err(Error::from(ErrorKind::InvalidData)),
        }
    }

    pub fn write_fri_config(&mut self, config: &FriConfig) -> Result<()> {
        let FriConfig {
            rate_bits,
            cap_height,
            num_query_rounds,
            proof_of_work_bits,
            reduction_strategy,
        } = &config;

        self.write_usize(*rate_bits)?;
        self.write_usize(*cap_height)?;
        self.write_usize(*num_query_rounds)?;
        self.write_u32(*proof_of_work_bits)?;
        self.write_fri_reduction_strategy(reduction_strategy)?;

        Ok(())
    }
    pub fn read_fri_config(&mut self) -> Result<FriConfig> {
        let rate_bits = self.read_usize()?;
        let cap_height = self.read_usize()?;
        let num_query_rounds = self.read_usize()?;
        let proof_of_work_bits = self.read_u32()?;
        let reduction_strategy = self.read_fri_reduction_strategy()?;

        Ok(FriConfig {
            rate_bits,
            cap_height,
            num_query_rounds,
            proof_of_work_bits,
            reduction_strategy,
        })
    }

    pub fn write_circuit_config(&mut self, config: &CircuitConfig) -> Result<()> {
        let CircuitConfig {
            num_wires,
            num_routed_wires,
            num_constants,
            security_bits,
            num_challenges,
            max_quotient_degree_factor,
            use_base_arithmetic_gate,
            zero_knowledge,
            fri_config,
        } = config;

        self.write_usize(*num_wires)?;
        self.write_usize(*num_routed_wires)?;
        self.write_usize(*num_constants)?;
        self.write_usize(*security_bits)?;
        self.write_usize(*num_challenges)?;
        self.write_usize(*max_quotient_degree_factor)?;
        self.write_bool(*use_base_arithmetic_gate)?;
        self.write_bool(*zero_knowledge)?;
        self.write_fri_config(fri_config)?;

        Ok(())
    }
    pub fn read_circuit_config(&mut self) -> Result<CircuitConfig> {
        let num_wires = self.read_usize()?;
        let num_routed_wires = self.read_usize()?;
        let num_constants = self.read_usize()?;
        let security_bits = self.read_usize()?;
        let num_challenges = self.read_usize()?;
        let max_quotient_degree_factor = self.read_usize()?;
        let use_base_arithmetic_gate = self.read_bool()?;
        let zero_knowledge = self.read_bool()?;
        let fri_config = self.read_fri_config()?;

        Ok(CircuitConfig {
            num_wires,
            num_routed_wires,
            num_constants,
            security_bits,
            num_challenges,
            max_quotient_degree_factor,
            use_base_arithmetic_gate,
            zero_knowledge,
            fri_config,
        })
    }

    pub fn write_fri_params(&mut self, fri_params: &FriParams) -> Result<()> {
        let FriParams {
            config,
            reduction_arity_bits,
            degree_bits,
            hiding,
        } = fri_params;

        self.write_fri_config(config)?;
        self.write_usize_vec(reduction_arity_bits.as_slice())?;
        self.write_usize(*degree_bits)?;
        self.write_bool(*hiding)?;

        Ok(())
    }
    pub fn read_fri_params(&mut self) -> Result<FriParams> {
        let config = self.read_fri_config()?;
        let reduction_arity_bits = self.read_usize_vec()?;
        let degree_bits = self.read_usize()?;
        let hiding = self.read_bool()?;

        Ok(FriParams {
            config,
            reduction_arity_bits,
            degree_bits,
            hiding,
        })
    }

    pub fn write_gate<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
        gate: &GateRef<F, D>,
        gate_serializer: &dyn GateSerializer<F, D>,
    ) -> Result<()> {
        gate_serializer.write_gate(self, gate)
    }

    pub fn read_gate<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
        gate_serializer: &dyn GateSerializer<F, D>,
    ) -> Result<GateRef<F, D>> {
        gate_serializer.read_gate(self)
    }

    pub fn write_selectors_info(&mut self, selectors_info: &SelectorsInfo) -> Result<()> {
        let SelectorsInfo {
            selector_indices,
            groups,
        } = selectors_info;

        self.write_usize_vec(selector_indices.as_slice())?;
        self.write_usize(groups.len())?;
        for group in groups.iter() {
            self.write_usize(group.start)?;
            self.write_usize(group.end)?;
        }
        Ok(())
    }
    pub fn read_selectors_info(&mut self) -> Result<SelectorsInfo> {
        let selector_indices = self.read_usize_vec()?;
        let groups_len = self.read_usize()?;
        let mut groups = Vec::with_capacity(groups_len);
        for _ in 0..groups_len {
            let start = self.read_usize()?;
            let end = self.read_usize()?;
            groups.push(Range { start, end });
        }

        Ok(SelectorsInfo {
            selector_indices,
            groups,
        })
    }

    pub fn write_common_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        common_data: &CommonCircuitData<F, C, D>,
        gate_serializer: &dyn GateSerializer<F, D>,
    ) -> Result<()> {
        let CommonCircuitData {
            degree_bits,
            quotient_degree_factor,
            num_gate_constraints,
            num_constants,
            num_virtual_targets,
            num_public_inputs,
            num_partial_products,
            circuit_digest,
            k_is,
            gates,
            config,
            fri_params,
            selectors_info,
        } = common_data;

        self.write_usize(*degree_bits)?;
        self.write_usize(*quotient_degree_factor)?;
        self.write_usize(*num_gate_constraints)?;
        self.write_usize(*num_constants)?;
        self.write_usize(*num_virtual_targets)?;
        self.write_usize(*num_public_inputs)?;
        self.write_usize(*num_partial_products)?;
        self.write_hash::<F, C::Hasher>(*circuit_digest)?;

        self.write_usize(k_is.len())?;
        self.write_field_vec(k_is.as_slice())?;

        self.write_usize(gates.len())?;
        for gate in gates.iter() {
            self.write_gate::<F, C, D>(gate, gate_serializer)?;
        }

        self.write_circuit_config(config)?;
        self.write_fri_params(fri_params)?;
        self.write_selectors_info(selectors_info)?;

        Ok(())
    }
    pub fn read_common_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        &mut self,
        gate_serializer: &dyn GateSerializer<F, D>,
    ) -> Result<CommonCircuitData<F, C, D>> {
        let degree_bits = self.read_usize()?;
        let quotient_degree_factor = self.read_usize()?;
        let num_gate_constraints = self.read_usize()?;
        let num_constants = self.read_usize()?;
        let num_virtual_targets = self.read_usize()?;
        let num_public_inputs = self.read_usize()?;
        let num_partial_products = self.read_usize()?;
        let circuit_digest = self.read_hash::<F, C::Hasher>()?;

        let k_is_len = self.read_usize()?;
        let k_is = self.read_field_vec(k_is_len)?;

        let gates_len = self.read_usize()?;
        let mut gates = Vec::with_capacity(gates_len);
        for _ in 0..gates_len {
            let gate = self.read_gate::<F, C, D>(gate_serializer)?;
            gates.push(gate);
        }

        let config = self.read_circuit_config()?;
        let fri_params = self.read_fri_params()?;
        let selectors_info = self.read_selectors_info()?;

        Ok(CommonCircuitData {
            degree_bits,
            quotient_degree_factor,
            num_gate_constraints,
            num_constants,
            num_virtual_targets,
            num_public_inputs,
            num_partial_products,
            circuit_digest,
            k_is,
            gates,
            config,
            fri_params,
            selectors_info,
        })
    }
}
