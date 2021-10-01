use std::convert::TryInto;
use std::fmt;
use std::io::Cursor;
use std::io::{Error, ErrorKind, Read, Result, Write};

use crate::field::crandall_field::CrandallField;
use crate::field::extension_field::quartic::QuarticExtension;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::{Field, PrimeField, RichField};
use crate::fri::proof::{FriInitialTreeProof, FriProof, FriQueryRound, FriQueryStep};
use crate::hash::hash_types::HashOut;
use crate::hash::merkle_proofs::MerkleProof;
use crate::hash::merkle_tree::{MerkleCap, MerkleTree};
use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData};
use crate::plonk::proof::{OpeningSet, Proof};
use crate::polynomial::polynomial::PolynomialCoeffs;

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

    pub fn write_u8(&mut self, x: u8) -> Result<()> {
        self.0.write_all(&[x])
    }
    pub fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        self.0.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn write_field<F: PrimeField>(&mut self, x: F) -> Result<()> {
        self.0.write_all(&x.to_canonical_u64().to_le_bytes())
    }
    pub fn read_field<F: PrimeField>(&mut self) -> Result<F> {
        let mut buf = [0; std::mem::size_of::<u64>()];
        self.0.read_exact(&mut buf)?;
        Ok(F::from_canonical_u64(u64::from_le_bytes(
            buf.try_into().unwrap(),
        )))
    }

    pub fn write_field_ext<F: Extendable<D>, const D: usize>(
        &mut self,
        x: F::Extension,
    ) -> Result<()> {
        for &a in &x.to_basefield_array() {
            self.write_field(a)?;
        }
        Ok(())
    }
    pub fn read_field_ext<F: Extendable<D>, const D: usize>(&mut self) -> Result<F::Extension> {
        let mut arr = [F::ZERO; D];
        for a in arr.iter_mut() {
            *a = self.read_field()?;
        }
        Ok(<F::Extension as FieldExtension<D>>::from_basefield_array(
            arr,
        ))
    }

    pub fn write_hash<F: PrimeField>(&mut self, h: HashOut<F>) -> Result<()> {
        for &a in &h.elements {
            self.write_field(a)?;
        }
        Ok(())
    }
    pub fn read_hash<F: PrimeField>(&mut self) -> Result<HashOut<F>> {
        let mut elements = [F::ZERO; 4];
        for a in elements.iter_mut() {
            *a = self.read_field()?;
        }
        Ok(HashOut { elements })
    }

    pub fn write_merkle_cap<F: PrimeField>(&mut self, cap: &MerkleCap<F>) -> Result<()> {
        for &a in &cap.0 {
            self.write_hash(a)?;
        }
        Ok(())
    }
    pub fn read_merkle_cap<F: PrimeField>(&mut self, cap_height: usize) -> Result<MerkleCap<F>> {
        let cap_length = 1 << cap_height;
        Ok(MerkleCap(
            (0..cap_length)
                .map(|_| self.read_hash())
                .collect::<Result<Vec<_>>>()?,
        ))
    }

    pub fn write_field_vec<F: PrimeField>(&mut self, v: &[F]) -> Result<()> {
        for &a in v {
            self.write_field(a)?;
        }
        Ok(())
    }
    pub fn read_field_vec<F: PrimeField>(&mut self, length: usize) -> Result<Vec<F>> {
        Ok((0..length)
            .map(|_| self.read_field())
            .collect::<Result<Vec<_>>>()?)
    }

    pub fn write_field_ext_vec<F: Extendable<D>, const D: usize>(
        &mut self,
        v: &[F::Extension],
    ) -> Result<()> {
        for &a in v {
            self.write_field_ext::<F, D>(a)?;
        }
        Ok(())
    }
    pub fn read_field_ext_vec<F: Extendable<D>, const D: usize>(
        &mut self,
        length: usize,
    ) -> Result<Vec<F::Extension>> {
        Ok((0..length)
            .map(|_| self.read_field_ext::<F, D>())
            .collect::<Result<Vec<_>>>()?)
    }

    pub fn write_opening_set<F: Extendable<D>, const D: usize>(
        &mut self,
        os: &OpeningSet<F, D>,
    ) -> Result<()> {
        self.write_field_ext_vec::<F, D>(&os.constants)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_sigmas)?;
        self.write_field_ext_vec::<F, D>(&os.wires)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_zs)?;
        self.write_field_ext_vec::<F, D>(&os.plonk_zs_right)?;
        self.write_field_ext_vec::<F, D>(&os.partial_products)?;
        self.write_field_ext_vec::<F, D>(&os.quotient_polys)
    }
    pub fn read_opening_set<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
        config: &CircuitConfig,
    ) -> Result<OpeningSet<F, D>> {
        let constants = self.read_field_ext_vec::<F, D>(common_data.num_constants)?;
        let plonk_sigmas = self.read_field_ext_vec::<F, D>(config.num_routed_wires)?;
        let wires = self.read_field_ext_vec::<F, D>(config.num_wires)?;
        let plonk_zs = self.read_field_ext_vec::<F, D>(config.num_challenges)?;
        let plonk_zs_right = self.read_field_ext_vec::<F, D>(config.num_challenges)?;
        let partial_products = self.read_field_ext_vec::<F, D>(
            common_data.num_partial_products.0 * config.num_challenges,
        )?;
        let quotient_polys = self.read_field_ext_vec::<F, D>(
            common_data.quotient_degree_factor * config.num_challenges,
        )?;
        Ok(OpeningSet {
            constants,
            plonk_sigmas,
            wires,
            plonk_zs,
            plonk_zs_right,
            partial_products,
            quotient_polys,
        })
    }

    pub fn write_merkle_proof<F: PrimeField>(&mut self, p: &MerkleProof<F>) -> Result<()> {
        let length = p.siblings.len();
        self.write_u8(
            length
                .try_into()
                .expect("Merkle proof length must fit in u8."),
        );
        for &h in &p.siblings {
            self.write_hash(h)?;
        }
        Ok(())
    }
    pub fn read_merkle_proof<F: PrimeField>(&mut self) -> Result<MerkleProof<F>> {
        let length = self.read_u8()?;
        Ok(MerkleProof {
            siblings: (0..length)
                .map(|_| self.read_hash())
                .collect::<Result<Vec<_>>>()?,
        })
    }

    pub fn write_fri_initial_proof<F: PrimeField>(
        &mut self,
        fitp: &FriInitialTreeProof<F>,
    ) -> Result<()> {
        for (v, p) in &fitp.evals_proofs {
            self.write_field_vec(v)?;
            self.write_merkle_proof(p)?;
        }
        Ok(())
    }
    pub fn read_fri_initial_proof<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
        config: &CircuitConfig,
    ) -> Result<FriInitialTreeProof<F>> {
        let mut evals_proofs = Vec::with_capacity(4);

        let constants_sigmas_v =
            self.read_field_vec(common_data.num_constants + config.num_routed_wires)?;
        let constants_sigmas_p = self.read_merkle_proof()?;
        evals_proofs.push((constants_sigmas_v, constants_sigmas_p));

        let wires_v = self.read_field_vec(config.num_wires)?;
        let wires_p = self.read_merkle_proof()?;
        evals_proofs.push((wires_v, wires_p));

        let zs_partial_v =
            self.read_field_vec(config.num_challenges * (1 + common_data.num_partial_products.0))?;
        let zs_partial_p = self.read_merkle_proof()?;
        evals_proofs.push((zs_partial_v, zs_partial_p));

        let quotient_v =
            self.read_field_vec(config.num_challenges * common_data.quotient_degree_factor)?;
        let quotient_p = self.read_merkle_proof()?;
        evals_proofs.push((quotient_v, quotient_p));

        Ok(FriInitialTreeProof { evals_proofs })
    }

    pub fn write_fri_query_steps<F: Extendable<D>, const D: usize>(
        &mut self,
        fqss: &[FriQueryStep<F, D>],
    ) -> Result<()> {
        for fqs in fqss {
            self.write_field_ext_vec::<F, D>(&fqs.evals)?;
            self.write_merkle_proof(&fqs.merkle_proof)?;
        }
        Ok(())
    }
    pub fn read_fri_query_steps<F: Extendable<D>, const D: usize>(
        &mut self,
        config: &CircuitConfig,
    ) -> Result<Vec<FriQueryStep<F, D>>> {
        let mut fqss = Vec::with_capacity(config.fri_config.reduction_arity_bits.len());
        for &arity_bits in &config.fri_config.reduction_arity_bits {
            let evals = self.read_field_ext_vec::<F, D>(1 << arity_bits)?;
            let merkle_proof = self.read_merkle_proof()?;
            fqss.push(FriQueryStep {
                evals,
                merkle_proof,
            })
        }
        Ok(fqss)
    }

    pub fn write_fri_query_rounds<F: Extendable<D>, const D: usize>(
        &mut self,
        fqrs: &[FriQueryRound<F, D>],
    ) -> Result<()> {
        for fqr in fqrs {
            self.write_fri_initial_proof(&fqr.initial_trees_proof)?;
            self.write_fri_query_steps(&fqr.steps)?;
        }
        Ok(())
    }
    pub fn read_fri_query_rounds<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
        config: &CircuitConfig,
    ) -> Result<Vec<FriQueryRound<F, D>>> {
        let mut fqrs = Vec::with_capacity(config.fri_config.num_query_rounds);
        for i in 0..config.fri_config.num_query_rounds {
            let initial_trees_proof = self.read_fri_initial_proof(common_data, config)?;
            let steps = self.read_fri_query_steps(config)?;
            fqrs.push(FriQueryRound {
                initial_trees_proof,
                steps,
            })
        }
        Ok(fqrs)
    }

    pub fn write_fri_proof<F: Extendable<D>, const D: usize>(
        &mut self,
        fp: &FriProof<F, D>,
    ) -> Result<()> {
        for cap in &fp.commit_phase_merkle_caps {
            self.write_merkle_cap(cap)?;
        }
        self.write_fri_query_rounds(&fp.query_round_proofs)?;
        self.write_field_ext_vec::<F, D>(&fp.final_poly.coeffs)?;
        self.write_field(fp.pow_witness)
    }
    pub fn read_fri_proof<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
        config: &CircuitConfig,
    ) -> Result<FriProof<F, D>> {
        let commit_phase_merkle_caps = (0..config.fri_config.reduction_arity_bits.len())
            .map(|_| self.read_merkle_cap(config.cap_height))
            .collect::<Result<Vec<_>>>()?;
        let query_round_proofs = self.read_fri_query_rounds(common_data, config)?;
        let final_poly = PolynomialCoeffs::new(self.read_field_ext_vec::<F, D>(
            1 << (common_data.degree_bits
                - config.fri_config.reduction_arity_bits.iter().sum::<usize>()),
        )?);
        let pow_witness = self.read_field()?;
        Ok(FriProof {
            commit_phase_merkle_caps,
            query_round_proofs,
            final_poly,
            pow_witness,
        })
    }

    pub fn write_proof<F: Extendable<D>, const D: usize>(
        &mut self,
        proof: &Proof<F, D>,
    ) -> Result<()> {
        self.write_merkle_cap(&proof.wires_cap)?;
        self.write_merkle_cap(&proof.plonk_zs_partial_products_cap)?;
        self.write_merkle_cap(&proof.quotient_polys_cap)?;
        self.write_opening_set(&proof.openings)?;
        self.write_fri_proof(&proof.opening_proof)
    }
    pub fn read_proof<F: RichField + Extendable<D>, const D: usize>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
        config: &CircuitConfig,
    ) -> Result<Proof<F, D>> {
        let wires_cap = self.read_merkle_cap(config.cap_height)?;
        let plonk_zs_partial_products_cap = self.read_merkle_cap(config.cap_height)?;
        let quotient_polys_cap = self.read_merkle_cap(config.cap_height)?;
        let openings = self.read_opening_set(common_data, config)?;
        let opening_proof = self.read_fri_proof(common_data, config)?;

        Ok(Proof {
            wires_cap,
            plonk_zs_partial_products_cap,
            quotient_polys_cap,
            openings,
            opening_proof,
        })
    }
}

#[test]
fn yo() {
    type F = CrandallField;
    type FF = QuarticExtension<F>;
    let mut buffer = Buffer::new(Vec::new());
    let x = FF::rand();
    buffer.write_field_ext::<F, 4>(x).unwrap();
    let mut buffer = Buffer::new(buffer.0.into_inner());
    let y: FF = buffer.read_field_ext::<F, 4>().unwrap();
}
