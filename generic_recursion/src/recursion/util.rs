use plonky2::hash::hash_types::{MerkleCapTarget, RichField};
use plonky2::iop::target::Target;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, VerifierCircuitTarget, VerifierOnlyCircuitData};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
use plonky2::field::extension::Extendable;

use crate::recursion::merge_circuit::CircuitSetTarget;

pub(crate) struct VerifierOnlyCircuitDataWrapper<C: GenericConfig<D>, const D: usize>(
    pub(crate) VerifierOnlyCircuitData<C, D>,
);

impl<C: GenericConfig<D>, const D: usize> From<&VerifierOnlyCircuitData<C, D>>
    for VerifierOnlyCircuitDataWrapper<C, D>
{
    fn from(vd: &VerifierOnlyCircuitData<C, D>) -> Self {
        VerifierOnlyCircuitDataWrapper(VerifierOnlyCircuitData {
            constants_sigmas_cap: vd.constants_sigmas_cap.clone(),
            circuit_digest: vd.circuit_digest.clone(),
        })
    }
}

impl<C: GenericConfig<D>, const D: usize> Clone for VerifierOnlyCircuitDataWrapper<C, D> {
    fn clone(&self) -> Self {
        VerifierOnlyCircuitDataWrapper(VerifierOnlyCircuitData {
            constants_sigmas_cap: self.0.constants_sigmas_cap.clone(),
            circuit_digest: self.0.circuit_digest.clone(),
        })
    }
}

// get the list of targets composing a `MerkleCapTarget`
pub(crate) fn merkle_cap_to_targets(merkle_cap: &MerkleCapTarget) -> Vec<Target> {
    merkle_cap.0.iter().flat_map(|h| h.elements).collect()
}

pub(crate) fn num_targets_for_circuit_set<F: RichField + Extendable<D>, const D: usize>(
    config: CircuitConfig,
) -> usize {
    let mut builder = CircuitBuilder::<F, D>::new(config);
    let target = CircuitSetTarget::build_target(&mut builder);
    target.to_targets().len()
}

// check in the circuit that the circuit digest in `verifier_data` is correctly computed from
// `verifier_data.constants_sigmas_cap` and the degree bits of the circuit
pub(crate) fn check_circuit_digest_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    builder: &mut CircuitBuilder<F, D>,
    verifier_data: &VerifierCircuitTarget,
    degree_bits: usize,
) where
    C::Hasher: AlgebraicHasher<F>,
{
    let cap_targets = merkle_cap_to_targets(&verifier_data.constants_sigmas_cap);
    // we assume the circuit was generated without a domain generator
    let domain_separator_target = builder
        .constant_hash(C::Hasher::hash_pad(&vec![]))
        .elements
        .to_vec();
    let degree_target = vec![builder.constant(F::from_canonical_usize(degree_bits))];
    let cap_hash = builder.hash_n_to_hash_no_pad::<C::Hasher>(
        [cap_targets, domain_separator_target, degree_target].concat(),
    );
    builder.connect_hashes(verifier_data.circuit_digest, cap_hash);
}
