use plonky2::field::extension_field::Extendable;
use plonky2::fri::oracle::PolynomialBatch;
use plonky2::fri::proof::{CompressedFriProof, FriProof};
use plonky2::hash::hash_types::RichField;
use plonky2::hash::merkle_tree::MerkleCap;
use plonky2::plonk::config::GenericConfig;
use rayon::prelude::*;

pub struct StarkProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: StarkOpeningSet<F, D>,
    /// A batch FRI argument for all openings.
    pub opening_proof: FriProof<F, C::Hasher, D>,
}

pub struct CompressedStarkProof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    /// Merkle cap of LDEs of trace values.
    pub trace_cap: MerkleCap<F, C::Hasher>,
    /// Purported values of each polynomial at the challenge point.
    pub openings: StarkOpeningSet<F, D>,
    /// A batch FRI argument for all openings.
    pub opening_proof: CompressedFriProof<F, C::Hasher, D>,
}

/// Purported values of each polynomial at the challenge point.
pub struct StarkOpeningSet<F: RichField + Extendable<D>, const D: usize> {
    pub local_values: Vec<F::Extension>,
    pub next_values: Vec<F::Extension>,
    pub permutation_zs: Vec<F::Extension>,
    pub quotient_polys: Vec<F::Extension>,
}

impl<F: RichField + Extendable<D>, const D: usize> StarkOpeningSet<F, D> {
    pub fn new<C: GenericConfig<D, F = F>>(
        zeta: F::Extension,
        g: F::Extension,
        trace_commitment: &PolynomialBatch<F, C, D>,
        quotient_commitment: &PolynomialBatch<F, C, D>,
    ) -> Self {
        let eval_commitment = |z: F::Extension, c: &PolynomialBatch<F, C, D>| {
            c.polynomials
                .par_iter()
                .map(|p| p.to_extension().eval(z))
                .collect::<Vec<_>>()
        };
        Self {
            local_values: eval_commitment(zeta, trace_commitment),
            next_values: eval_commitment(zeta * g, trace_commitment),
            permutation_zs: vec![/*TODO*/],
            quotient_polys: eval_commitment(zeta, quotient_commitment),
        }
    }
}
