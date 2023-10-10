use std::marker::PhantomData;

use plonky2::hash::hash_types::RichField;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartialWitness, PartitionWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, VerifierOnlyCircuitData,
};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};
use plonky2::plonk::proof::{
    Proof, ProofTarget, ProofWithPublicInputs, ProofWithPublicInputsTarget,
};
use plonky2::util::serialization::{Buffer, DefaultGateSerializer, IoResult, Read, Write};
use plonky2x::backend::circuit::Circuit;
use plonky2x::backend::function::VerifiableFunction;
use plonky2x::frontend::uint::uint256::U256Variable;
use plonky2x::prelude::{
    ArrayVariable, BoolVariable, Bytes32Variable, CircuitBuilder as CircuitBuilderX,
    CircuitVariable, Extendable, Field, PlonkParameters, Variable,
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
    <<L as PlonkParameters<D>>::Config as GenericConfig<D>>::Hasher:
        AlgebraicHasher<<L as PlonkParameters<D>>::Field>,
{
    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<L::Field, D>::new(config);

    let mut public_input_targets = vec![];
    // The arithmetic circuit.
    for _ in 0..8 {
        let uint256_a_target = builder.add_virtual_target();
        public_input_targets.push(uint256_a_target);
        builder.register_public_input(uint256_a_target);
    }
    for _ in 0..8 {
        let uint256_b_target = builder.add_virtual_target();
        public_input_targets.push(uint256_b_target);
        builder.register_public_input(uint256_b_target);
    }

    // Provide initial values.
    let mut pw = PartialWitness::new();
    // Sets uint256_a = 0
    for offset in 0..8 {
        pw.set_target(public_input_targets[offset], L::Field::ZERO);
    }
    // Sets uint256_b = 1, it is little endian, so we store 1 at the "first" byte
    // Set the last public input to 1 to make uint256_b = 1
    pw.set_target(public_input_targets[8], L::Field::ONE);
    for offset in 9..16 {
        pw.set_target(public_input_targets[offset], L::Field::ZERO);
    }

    let data = builder.build();
    let proof = data.prove(pw).unwrap();

    (data, proof)
}

fn connect_public_inputs<L: PlonkParameters<D>, const D: usize>(
    builder: &mut CircuitBuilderX<L, D>,
    public_input_targets: &Vec<Target>,
    input_target_vec: &Vec<Target>,
) {
    assert_eq!(public_input_targets.len(), input_target_vec.len());
    for (i, target) in input_target_vec.iter().enumerate() {
        builder.api.connect(*target, public_input_targets[i]);
    }
}

#[derive(Debug, Clone)]
pub struct ProofGenerator<L: PlonkParameters<D>, const D: usize> {
    pub proof_with_public_inputs_target: ProofWithPublicInputsTarget<D>,
    pub proof_with_public_inputs: ProofWithPublicInputs<L::Field, L::Config, D>,
    pub common_data: CommonCircuitData<L::Field, D>,
    pub _marker: PhantomData<L>,
}

impl<L: PlonkParameters<D>, const D: usize> ProofGenerator<L, D> {
    fn id() -> String {
        "ProofGenerator".to_string()
    }
}

impl<L: PlonkParameters<D>, const D: usize> SimpleGenerator<L::Field, D> for ProofGenerator<L, D>
where
    <<L as PlonkParameters<D>>::Config as GenericConfig<D>>::Hasher:
        AlgebraicHasher<<L as PlonkParameters<D>>::Field>,
{
    fn id(&self) -> String {
        Self::id()
    }

    fn serialize(
        &self,
        dst: &mut Vec<u8>,
        _common_data: &CommonCircuitData<L::Field, D>,
    ) -> IoResult<()> {
        dst.write_target_proof_with_public_inputs(&self.proof_with_public_inputs_target)?;
        let gate_serializer = DefaultGateSerializer {};
        dst.write_common_circuit_data(&self.common_data, &gate_serializer)?;
        dst.write_proof_with_public_inputs(&self.proof_with_public_inputs)
    }

    fn deserialize(
        src: &mut Buffer,
        _common_data: &CommonCircuitData<L::Field, D>,
    ) -> IoResult<Self> {
        let proof_with_public_inputs_target = src.read_target_proof_with_public_inputs()?;
        let gate_serializer = DefaultGateSerializer {};
        let common_data: CommonCircuitData<L::Field, D> =
            src.read_common_circuit_data(&gate_serializer)?;
        let proof_with_public_inputs = src.read_proof_with_public_inputs(&common_data)?;
        Ok(Self {
            proof_with_public_inputs_target,
            proof_with_public_inputs,
            common_data,
            _marker: PhantomData,
        })
    }

    fn dependencies(&self) -> Vec<Target> {
        vec![]
    }

    fn run_once(
        &self,
        witness: &PartitionWitness<L::Field>,
        out_buffer: &mut GeneratedValues<L::Field>,
    ) {
        out_buffer.set_proof_with_pis_target(
            &self.proof_with_public_inputs_target,
            &self.proof_with_public_inputs,
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrapCircuit;

impl Circuit for WrapCircuit {
    fn define<L: PlonkParameters<D>, const D: usize>(builder: &mut CircuitBuilderX<L, D>)
    where
        <<L as PlonkParameters<D>>::Config as GenericConfig<D>>::Hasher:
            AlgebraicHasher<<L as PlonkParameters<D>>::Field>,
    {
        let u256_a = builder.evm_read::<U256Variable>();
        let u256_b = builder.evm_read::<U256Variable>();

        let mut input_target_vec = vec![];
        input_target_vec.extend(u256_a.targets());
        input_target_vec.extend(u256_b.targets());
        assert_eq!(input_target_vec.len(), 16);

        let (data, proof) = dummy_proof::<L, D>();

        // This would use the block circuit data
        let proof_targets = builder.api.add_virtual_proof_with_pis(&data.common);
        let verifier_targets = builder
            .api
            .constant_verifier_data::<L::Config>(&data.verifier_only);

        // Sets proof_targets to a constant proof
        // In a production setting, the constant proof should be fetched from an API endpoint based on the public inputs using a `Hint` (i.e. generator)
        let generator = ProofGenerator {
            proof_with_public_inputs_target: proof_targets.clone(),
            proof_with_public_inputs: proof,
            common_data: data.common.clone(),
            _marker: PhantomData::<L>,
        };
        builder.add_simple_generator(generator);

        builder.watch_slice(
            &input_target_vec
                .iter()
                .map(|t| Variable(*t))
                .collect::<Vec<Variable>>(),
            "input_target_vec",
        );
        builder.watch_slice(
            &proof_targets
                .public_inputs
                .iter()
                .map(|t| Variable(*t))
                .collect::<Vec<Variable>>(),
            "proof_targets_public_inputs",
        );

        // Connect the public inputs we read from on-chain to the proof_targets.public_inputs
        connect_public_inputs(
            builder,
            &proof_targets.public_inputs.clone(),
            &input_target_vec,
        );

        // Verify the final proof.
        builder
            .api
            .verify_proof::<L::Config>(&proof_targets, &verifier_targets, &data.common);

        let sum = builder.add(u256_a, u256_b);
        builder.evm_write(sum);
    }

    fn register_generators<L: PlonkParameters<D>, const D: usize>(
        registry: &mut plonky2x::prelude::HintRegistry<L, D>,
    ) where
        <<L as PlonkParameters<D>>::Config as GenericConfig<D>>::Hasher: AlgebraicHasher<L::Field>,
    {
        registry.register_simple::<ProofGenerator<L, D>>(ProofGenerator::<L, D>::id());
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

        let input_bytes = hex::decode("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001").unwrap();

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
    fn test_wrapper_circuit_io() {
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
