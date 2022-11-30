use alloc::vec;

use hashbrown::HashMap;
use plonky2_field::extension::Extendable;
use plonky2_util::ceil_div_usize;

use crate::gates::noop::NoopGate;
use crate::hash::hash_types::RichField;
use crate::iop::witness::{PartialWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{CircuitData, CommonCircuitData};
use crate::plonk::config::GenericConfig;
use crate::plonk::proof::ProofWithPublicInputs;

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
