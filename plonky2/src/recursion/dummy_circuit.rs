use alloc::vec;
use alloc::vec::Vec;

use hashbrown::HashMap;
use plonky2_field::extension::Extendable;
use plonky2_util::ceil_div_usize;

use crate::gates::noop::NoopGate;
use crate::hash::hash_types::RichField;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::Target;
use crate::iop::witness::{PartialWitness, PartitionWitness, WitnessWrite};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{
    CircuitData, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
};
use crate::plonk::config::{AlgebraicHasher, GenericConfig};
use crate::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};

/// Creates a dummy proof which is suitable for use as a base proof in a cyclic recursion tree.
/// Such a base proof will not actually be verified, so most of its data is arbitrary. However, its
/// public inputs which encode the cyclic verification key must be set properly, and this method
/// takes care of that. It also allows the user to specify any other public inputs which should be
/// set in this base proof.
pub fn cyclic_base_proof<F, C, const D: usize>(
    common_data: &CommonCircuitData<F, D>,
    verifier_data: &VerifierOnlyCircuitData<C, D>,
    mut nonzero_public_inputs: HashMap<usize, F>,
) -> ProofWithPublicInputs<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    C::Hasher: AlgebraicHasher<C::F>,
{
    let pis_len = common_data.num_public_inputs;
    let cap_elements = common_data.config.fri_config.num_cap_elements();
    let start_vk_pis = pis_len - 4 - 4 * cap_elements;

    // Add the cyclic verifier data public inputs.
    nonzero_public_inputs.extend((start_vk_pis..).zip(verifier_data.circuit_digest.elements));
    for i in 0..cap_elements {
        let start = start_vk_pis + 4 + 4 * i;
        nonzero_public_inputs
            .extend((start..).zip(verifier_data.constants_sigmas_cap.0[i].elements));
    }

    // TODO: A bit wasteful to build a dummy circuit here. We could potentially use a proof that
    // just consists of zeros, apart from public inputs.
    dummy_proof(&dummy_circuit(common_data), nonzero_public_inputs).unwrap()
}

/// Generate a proof for a dummy circuit. The `public_inputs` parameter let the caller specify
/// certain public inputs (identified by their indices) which should be given specific values.
/// The rest will default to zero.
pub(crate) fn dummy_proof<F, C, const D: usize>(
    circuit: &CircuitData<F, C, D>,
    nonzero_public_inputs: HashMap<usize, F>,
) -> anyhow::Result<ProofWithPublicInputs<F, C, D>>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    let mut pw = PartialWitness::new();
    for i in 0..circuit.common.num_public_inputs {
        let pi = nonzero_public_inputs.get(&i).copied().unwrap_or_default();
        pw.set_target(circuit.prover_only.public_inputs[i], pi);
    }
    circuit.prove(pw)
}

/// Generate a circuit matching a given `CommonCircuitData`.
pub(crate) fn dummy_circuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    common_data: &CommonCircuitData<F, D>,
) -> CircuitData<F, C, D> {
    let config = common_data.config.clone();
    assert!(
        !common_data.config.zero_knowledge,
        "Degree calculation can be off if zero-knowledge is on."
    );

    // Number of `NoopGate`s to add to get a circuit of size `degree` in the end.
    // Need to account for public input hashing, a `PublicInputGate` and a `ConstantGate`.
    let degree = common_data.degree();
    let num_noop_gate = degree - ceil_div_usize(common_data.num_public_inputs, 8) - 2;

    let mut builder = CircuitBuilder::<F, D>::new(config);
    for _ in 0..num_noop_gate {
        builder.add_gate(NoopGate, vec![]);
    }
    for gate in &common_data.gates {
        builder.add_gate_to_gate_set(gate.clone());
    }
    for _ in 0..common_data.num_public_inputs {
        builder.add_virtual_public_input();
    }

    let circuit = builder.build::<C>();
    assert_eq!(&circuit.common, common_data);
    circuit
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub(crate) fn dummy_proof_and_vk<C: GenericConfig<D, F = F> + 'static>(
        &mut self,
        common_data: &CommonCircuitData<F, D>,
    ) -> anyhow::Result<(ProofWithPublicInputsTarget<D>, VerifierCircuitTarget)>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let dummy_circuit = dummy_circuit::<F, C, D>(common_data);
        let dummy_proof_with_pis = dummy_proof(&dummy_circuit, HashMap::new())?;
        let dummy_proof_with_pis_target = self.add_virtual_proof_with_pis::<C>(common_data);
        let dummy_verifier_data_target =
            self.add_virtual_verifier_data(self.config.fri_config.cap_height);

        self.add_simple_generator(DummyProofGenerator {
            proof_with_pis_target: dummy_proof_with_pis_target.clone(),
            proof_with_pis: dummy_proof_with_pis,
            verifier_data_target: dummy_verifier_data_target.clone(),
            verifier_data: dummy_circuit.verifier_only,
        });

        Ok((dummy_proof_with_pis_target, dummy_verifier_data_target))
    }
}

#[derive(Debug)]
pub(crate) struct DummyProofGenerator<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub(crate) proof_with_pis_target: ProofWithPublicInputsTarget<D>,
    pub(crate) proof_with_pis: ProofWithPublicInputs<F, C, D>,
    pub(crate) verifier_data_target: VerifierCircuitTarget,
    pub(crate) verifier_data: VerifierOnlyCircuitData<C, D>,
}

impl<F, C, const D: usize> SimpleGenerator<F> for DummyProofGenerator<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    C::Hasher: AlgebraicHasher<F>,
{
    fn dependencies(&self) -> Vec<Target> {
        vec![]
    }

    fn run_once(&self, _witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        out_buffer.set_proof_with_pis_target(&self.proof_with_pis_target, &self.proof_with_pis);
        out_buffer.set_verifier_data_target(&self.verifier_data_target, &self.verifier_data);
    }
}
