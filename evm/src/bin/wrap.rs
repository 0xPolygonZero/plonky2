// use plonky2::field::types::Field;
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData,
};
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::proof::{Proof, ProofTarget, ProofWithPublicInputs};
use plonky2x::backend::circuit::Circuit;
use plonky2x::backend::function::VerifiableFunction;
use plonky2x::frontend::uint::uint256::U256Variable;
use plonky2x::prelude::{
    ArrayVariable, BoolVariable, Bytes32Variable, CircuitBuilder as CircuitBuilderX,
    CircuitVariable, Field, PlonkParameters,
};
use serde::{Deserialize, Serialize};

fn dummy_proof<L: PlonkParameters<D>, const D: usize>() -> (
    CircuitData<L::Field, L::Config, D>,
    ProofWithPublicInputs<L::Field, L::Config, D>,
)
where
    <L as PlonkParameters<D>>::Field: plonky2::hash::hash_types::RichField,
    <L as PlonkParameters<D>>::Field: plonky2::field::extension::Extendable<D>,
    <L as PlonkParameters<D>>::Config: GenericConfig<D, F = L::Field>,
{
    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<L::Field, D>::new(config);

    let mut public_input_targets = vec![];
    // The arithmetic circuit.
    for _ in 0..32 {
        let uint256_a_target = builder.add_virtual_target();
        public_input_targets.push(uint256_a_target);
        builder.register_public_input(uint256_a_target);
    }
    for _ in 0..32 {
        let uint256_b_target = builder.add_virtual_target();
        public_input_targets.push(uint256_b_target);
        builder.register_public_input(uint256_b_target);
    }

    // Provide initial values.
    let mut pw = PartialWitness::new();
    for offset in 0..32 {
        pw.set_target(public_input_targets[offset], L::Field::ZERO);
        pw.set_target(public_input_targets[32 + offset], L::Field::ONE);
    }

    let data = builder.build();
    let proof = data.prove(pw).unwrap();

    (data, proof)
}

// fn connect_public_inputs<L: PlonkParameters<D>, const D: usize>(
//     &mut builder: &mut CircuitBuilderX<L, D>,
//     public_input_targets: &Vec<Target>,
//     input_target_vec: &Vec<Target>,
// ) {
//     for (i, target) in input_target_vec.iter().enumerate() {
//         builder.api.connect(*target, public_input_targets[i]);
//     }
// }

// fn connect_proof<L: PlonkParameters<D>, const D: usize>(
//     &mut builder: &mut CircuitBuilderX<L, D>,
//     proof_target: &ProofTarget<D>,
//     constant_proof: &Proof<L::Field, L::Config, D>,
// ) {
//     todo!()
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrapCircuit;

impl Circuit for WrapCircuit {
    fn define<L: PlonkParameters<D>, const D: usize>(builder: &mut CircuitBuilderX<L, D>) {
        let u256_a = builder.evm_read::<U256Variable>();
        let u256_b = builder.evm_read::<U256Variable>();

        let mut input_target_vec = vec![];
        input_target_vec.extend(u256_a.targets());
        input_target_vec.extend(u256_b.targets());
        assert_eq!(input_target_vec.len(), 16);

        /*
        let (data, proof) = dummy_proof::<L, D>();

        // This would use the block circuit data
        let proof_targets = builder.api.add_virtual_proof_with_pis(&data.common);
        let verifier_targets = builder.api.constant_verifier_data::<L>(&data.verifier_only);

        // Sets proof_targets to a constant proof
        // In a production setting, the constant proof should be fetched from an API endpoint based on the public inputs using a `Hint` (i.e. generator)
        connect_proof(&mut builder.api, &proof_targets.proof, &proof.proof);

        // Connect the public inputs we read from on-chain to the proof_targets.public_inputs
        // TODO: there might be a better way to do this with CircuitBuilderX method
        connect_public_inputs(
            &mut builder,
            &proof_targets.public_inputs,
            &input_target_vec,
        );

        // Verify the final proof.
        builder
            .api
            .verify_proof::<L::Config>(&proof_targets, &verifier_targets, &data.common);

        */

        let sum = builder.add(u256_a, u256_b);
        builder.evm_write(sum);
    }
}

fn main() {
    VerifiableFunction::<WrapCircuit>::entrypoint();
}

#[cfg(test)]
mod tests {
    use std::env;

    use ethers::types::H256;
    use ethers::utils::hex;
    use plonky2x::backend::circuit::PublicInput;
    use plonky2x::prelude::{DefaultBuilder, GateRegistry, HintRegistry};

    use super::*;

    #[test]
    #[cfg_attr(feature = "ci", ignore)]
    fn test_serialization() {
        env::set_var("RUST_LOG", "debug");
        env_logger::try_init().unwrap_or_default();

        let mut builder = DefaultBuilder::new();

        log::debug!("Defining circuit");
        WrapCircuit::define(&mut builder);
        let circuit = builder.build();
        log::debug!("Done building circuit");

        let mut hint_registry = HintRegistry::new();
        let mut gate_registry = GateRegistry::new();
        WrapCircuit::register_generators(&mut hint_registry);
        WrapCircuit::register_gates(&mut gate_registry);

        circuit.test_serializers(&gate_registry, &hint_registry);
    }

    #[test]
    #[cfg_attr(feature = "ci", ignore)]
    fn test_wrapper_circuit_input_bytes() {
        env::set_var("RUST_LOG", "debug");
        env_logger::try_init().unwrap_or_default();

        let input_bytes =
            hex::decode("00000000000000000000000000000000000000000000000000000000000000001111111111111111111111111111111111111111111111111111111111111111")
                .unwrap();

        let mut builder = DefaultBuilder::new();

        log::debug!("Defining circuit");
        WrapCircuit::define(&mut builder);

        log::debug!("Building circuit");
        let circuit = builder.build();
        log::debug!("Done building circuit");

        let input = PublicInput::Bytes(input_bytes);
        let (_proof, mut output) = circuit.prove(&input);
        let sum = output.evm_read::<U256Variable>();
        println!("sum: {}", sum);
    }

    #[test]
    #[cfg_attr(feature = "ci", ignore)]
    fn test_wrapper_circuit() {
        env::set_var("RUST_LOG", "debug");
        env_logger::try_init().unwrap_or_default();

        let mut builder = DefaultBuilder::new();

        log::debug!("Defining circuit");
        WrapCircuit::define(&mut builder);

        log::debug!("Building circuit");
        let circuit = builder.build();
        log::debug!("Done building circuit");

        let mut input = circuit.input();
        input.evm_write::<U256Variable>(0.into());
        input.evm_write::<U256Variable>(1.into());

        log::debug!("Generating proof");
        let (proof, mut output) = circuit.prove(&input);
        log::debug!("Done generating proof");

        circuit.verify(&proof, &input, &output);
        let sum = output.evm_read::<U256Variable>();
        println!("sum: {}", sum);
    }
}
