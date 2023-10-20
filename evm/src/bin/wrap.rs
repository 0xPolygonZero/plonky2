use std::marker::PhantomData;

use plonky2::iop::generator::{GeneratedValues, SimpleGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartitionWitness, WitnessWrite};
use plonky2::plonk::circuit_data::{CommonCircuitData, VerifierOnlyCircuitData};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, GenericHashOut};
use plonky2::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use plonky2::util::serialization::{Buffer, DefaultGateSerializer, IoResult, Read, Write};
use plonky2_evm::sample::get_sample_circuits;
use plonky2x::backend::circuit::Circuit;
use plonky2x::backend::function::Plonky2xFunction;
use plonky2x::frontend::uint::uint160::U160Variable;
use plonky2x::frontend::uint::uint256::U256Variable;
use plonky2x::prelude::{
    CircuitBuilder as CircuitBuilderX, CircuitVariable, Field, PlonkParameters, Variable,
};
use serde::{Deserialize, Serialize};

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
        _witness: &PartitionWitness<L::Field>,
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
        let state_root_before = builder.evm_read::<U256Variable>();
        let transactions_root_before = builder.evm_read::<U256Variable>();
        let receipts_root_before = builder.evm_read::<U256Variable>();

        let state_root_after = builder.evm_read::<U256Variable>();
        let transactions_root_after = builder.evm_read::<U256Variable>();
        let receipts_root_after = builder.evm_read::<U256Variable>();

        let block_beneficiary = builder.evm_read::<U160Variable>();
        let block_timestamp = builder.evm_read::<U256Variable>();
        let block_number = builder.evm_read::<U256Variable>();
        let block_difficulty = builder.evm_read::<U256Variable>();
        let block_random = builder.evm_read::<U256Variable>();
        let block_gaslimit = builder.evm_read::<U256Variable>();
        let block_chain_id = builder.evm_read::<U256Variable>();
        let block_base_fee = builder.evm_read::<U256Variable>();
        let block_gas_used = builder.evm_read::<U256Variable>();

        let block_bloom = (0..8)
            .map(|_| builder.evm_read::<U256Variable>())
            .collect::<Vec<_>>();

        let prev_hashes = (0..256)
            .map(|_| builder.evm_read::<U256Variable>())
            .collect::<Vec<_>>();
        let cur_hash = builder.evm_read::<U256Variable>();

        let genesis_state_trie_root = builder.evm_read::<U256Variable>();
        let txn_number_before = builder.evm_read::<U256Variable>();
        let txn_number_after = builder.evm_read::<U256Variable>();
        let gas_used_before = builder.evm_read::<U256Variable>();
        let gas_used_after = builder.evm_read::<U256Variable>();
        let block_boom_before = (0..8)
            .map(|_| builder.evm_read::<U256Variable>())
            .collect::<Vec<_>>();
        let block_boom_after = (0..8)
            .map(|_| builder.evm_read::<U256Variable>())
            .collect::<Vec<_>>();

        let mut input_target_vec = vec![];

        input_target_vec.extend(state_root_before.targets());
        input_target_vec.extend(transactions_root_before.targets());
        input_target_vec.extend(receipts_root_before.targets());

        input_target_vec.extend(state_root_after.targets());
        input_target_vec.extend(transactions_root_after.targets());
        input_target_vec.extend(receipts_root_after.targets());

        input_target_vec.extend(block_beneficiary.targets());

        let zero = builder.zero::<Variable>();

        // for block_timestamp, we'll read just the first u32 from the on-chain u256
        input_target_vec.push(block_timestamp.targets()[0]);
        let _ = (1..8).map(|i| builder.assert_is_equal(block_timestamp.variables()[i], zero));

        input_target_vec.push(block_number.targets()[0]);
        let _ = (1..8).map(|i| builder.assert_is_equal(block_number.variables()[i], zero));

        input_target_vec.push(block_difficulty.targets()[0]);
        let _ = (1..8).map(|i| builder.assert_is_equal(block_difficulty.variables()[i], zero));

        input_target_vec.extend(block_random.targets());

        input_target_vec.extend(block_gaslimit.targets().iter().take(2));
        let _ = (2..8).map(|i| builder.assert_is_equal(block_gaslimit.variables()[i], zero));

        input_target_vec.push(block_chain_id.targets()[0]);
        let _ = (1..8).map(|i| builder.assert_is_equal(block_chain_id.variables()[i], zero));

        input_target_vec.extend(block_base_fee.targets().iter().take(2));
        let _ = (2..8).map(|i| builder.assert_is_equal(block_base_fee.variables()[i], zero));

        input_target_vec.extend(block_gas_used.targets().iter().take(2));
        let _ = (2..8).map(|i| builder.assert_is_equal(block_gas_used.variables()[i], zero));

        input_target_vec.extend(block_bloom.iter().flat_map(|b| b.targets()));

        input_target_vec.extend(prev_hashes.iter().flat_map(|b| b.targets()));
        input_target_vec.extend(cur_hash.targets());

        input_target_vec.extend(genesis_state_trie_root.targets());

        input_target_vec.push(txn_number_before.targets()[0]);
        let _ = (1..8).map(|i| builder.assert_is_equal(txn_number_before.variables()[i], zero));

        input_target_vec.push(txn_number_after.targets()[0]);
        let _ = (1..8).map(|i| builder.assert_is_equal(txn_number_after.variables()[i], zero));

        input_target_vec.extend(gas_used_before.targets().iter().take(2));
        let _ = (2..8).map(|i| builder.assert_is_equal(gas_used_before.variables()[i], zero));

        input_target_vec.extend(gas_used_after.targets().iter().take(2));
        let _ = (2..8).map(|i| builder.assert_is_equal(gas_used_after.variables()[i], zero));

        input_target_vec.extend(block_boom_before.iter().flat_map(|b| b.targets()));
        input_target_vec.extend(block_boom_after.iter().flat_map(|b| b.targets()));

        let block_proof_json = std::fs::read_to_string("block_proof.json").unwrap();
        let block_proof: ProofWithPublicInputs<L::Field, L::Config, D> =
            serde_json::from_str(&block_proof_json).unwrap();

        let verifier_data_json = std::fs::read("serialized_verifier_only_data").unwrap();
        let common_data_json = std::fs::read("serialized_common_data").unwrap();
        let verifier_data =
            VerifierOnlyCircuitData::<L::Config, D>::from_bytes(verifier_data_json).unwrap();
        let common_data = CommonCircuitData::<L::Field, D>::from_bytes(
            common_data_json,
            &DefaultGateSerializer {},
        )
        .unwrap();

        for elem in verifier_data.circuit_digest.to_vec() {
            input_target_vec.push(builder.constant::<Variable>(elem).0);
        }
        for cap in verifier_data.constants_sigmas_cap.0.iter() {
            for elem in cap.to_vec() {
                input_target_vec.push(builder.constant::<Variable>(elem).0);
            }
        }

        // This would use the block circuit data
        let proof_targets = builder.api.add_virtual_proof_with_pis(&common_data);
        let verifier_targets = builder
            .api
            .constant_verifier_data::<L::Config>(&verifier_data);

        // Sets proof_targets to a constant proof
        // In a production setting, the constant proof should be fetched from an API endpoint based on the public inputs using a `Hint` (i.e. generator)
        let generator = ProofGenerator {
            proof_with_public_inputs_target: proof_targets.clone(),
            proof_with_public_inputs: block_proof,
            common_data,
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
    WrapCircuit::entrypoint();
}

#[cfg(test)]
mod tests {
    use std::env;

    use ethereum_types::U256;
    use ethers::utils::hex;
    use plonky2x::backend::circuit::PublicInput;
    use plonky2x::frontend::uint::uint160::U160;
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
        let (_proof, _output) = circuit.prove(&input);
    }

    fn hex_str_to_u256(hex: &str) -> U256 {
        let value = U256::from_str_radix(&hex[2..], 16).expect("Failed to convert to U256");
        value
    }

    fn hex_str_to_u160(hex: &str) -> U160 {
        U160::from_u32_limbs([
            u32::from_str_radix(&hex[34..42], 16).expect("Failed to convert to u32"),
            u32::from_str_radix(&hex[26..34], 16).expect("Failed to convert to u32"),
            u32::from_str_radix(&hex[18..26], 16).expect("Failed to convert to u32"),
            u32::from_str_radix(&hex[10..18], 16).expect("Failed to convert to u32"),
            u32::from_str_radix(&hex[2..10], 16).expect("Failed to convert to u32"),
        ])
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
        let circuit = Box::new(builder.build());
        log::debug!("Done building circuit");

        let mut input = Box::new(circuit.input());

        // trie_roots_before
        // state_root
        input.evm_write::<U256Variable>(hex_str_to_u256(
            "0x92648889955b1d41b36ea681a16ef94852e34e6011d029f278439adb4e9e30b4",
        ));
        // transactions_root
        input.evm_write::<U256Variable>(hex_str_to_u256(
            "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        ));
        // receipts_root
        input.evm_write::<U256Variable>(hex_str_to_u256(
            "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
        ));

        // trie_roots_after
        // state_root
        input.evm_write::<U256Variable>(hex_str_to_u256(
            "0x049e45aef8dac161e0cec0edacd8af5b3399700affad6ede63b33c5d0ec796f5",
        ));
        // transactions_root
        input.evm_write::<U256Variable>(hex_str_to_u256(
            "0xc523d7b87c0e49a24dae53b3e3be716e5a6808c1e05216497655c0ad84b12236",
        ));
        // receipts_root
        input.evm_write::<U256Variable>(hex_str_to_u256(
            "0xfc047c9c96ea3d317bf5b0896e85c242ecc625efd3f7da721c439aff8331b2ab",
        ));

        // block_metadata
        // block_beneficiary
        let val = hex_str_to_u160("0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba");
        input.evm_write::<U160Variable>(val);
        // block_timestamp
        input.evm_write::<U256Variable>(U256::from(1000));
        // block_number
        input.evm_write::<U256Variable>(U256::from(0));
        // block_difficulty
        input.evm_write::<U256Variable>(U256::from(131072));
        // block_random
        input.evm_write::<U256Variable>(U256::from(0));
        // block_gaslimit
        input.evm_write::<U256Variable>(U256::from(4478310));
        // block_chain_id
        input.evm_write::<U256Variable>(U256::from(1));
        // block_base_fee
        input.evm_write::<U256Variable>(U256::from(10));
        // block_gas_used
        input.evm_write::<U256Variable>(U256::from(43570));
        // block_bloom
        input.evm_write::<U256Variable>(U256::from(0));
        input.evm_write::<U256Variable>(U256::from(0));
        input.evm_write::<U256Variable>(
            U256::from_dec_str(
                "55213970774324510299479508399853534522527075462195808724319849722937344",
            )
            .unwrap(),
        );
        input.evm_write::<U256Variable>(
            U256::from_dec_str("1361129467683753853853498429727072845824").unwrap(),
        );
        input.evm_write::<U256Variable>(U256::from(33554432));
        input.evm_write::<U256Variable>(U256::from_dec_str("9223372036854775808").unwrap());
        input.evm_write::<U256Variable>(
            U256::from_dec_str(
                "3618502788666131106986593281521497120414687020801267626233049500247285563392",
            )
            .unwrap(),
        );
        input.evm_write::<U256Variable>(
            U256::from_dec_str("2722259584404615024560450425766186844160").unwrap(),
        );

        // block_hashes
        // prev_hashes
        for _ in 0..256 {
            input.evm_write::<U256Variable>(U256::from(0));
        }
        // cur_hash
        input.evm_write::<U256Variable>(U256::from(0));

        // extra_block_data
        // genesis_state_trie_root
        input.evm_write::<U256Variable>(hex_str_to_u256(
            "0x92648889955b1d41b36ea681a16ef94852e34e6011d029f278439adb4e9e30b4",
        ));
        // txn_number_before
        input.evm_write::<U256Variable>(U256::from(0));
        // txn_number_after
        input.evm_write::<U256Variable>(U256::from(2));
        // gas_used_before
        input.evm_write::<U256Variable>(U256::from(0));
        // gas_used_after
        input.evm_write::<U256Variable>(U256::from(43570));
        // block_boom_before
        for _ in 0..8 {
            input.evm_write::<U256Variable>(U256::from(0));
        }
        // block_boom_after
        input.evm_write::<U256Variable>(U256::from(0));
        input.evm_write::<U256Variable>(U256::from(0));
        input.evm_write::<U256Variable>(
            U256::from_dec_str(
                "55213970774324510299479508399853534522527075462195808724319849722937344",
            )
            .unwrap(),
        );
        input.evm_write::<U256Variable>(
            U256::from_dec_str("1361129467683753853853498429727072845824").unwrap(),
        );
        input.evm_write::<U256Variable>(U256::from(33554432));
        input.evm_write::<U256Variable>(U256::from_dec_str("9223372036854775808").unwrap());
        input.evm_write::<U256Variable>(
            U256::from_dec_str(
                "3618502788666131106986593281521497120414687020801267626233049500247285563392",
            )
            .unwrap(),
        );
        input.evm_write::<U256Variable>(
            U256::from_dec_str("2722259584404615024560450425766186844160").unwrap(),
        );

        log::debug!("Generating proof");
        let (proof, output) = circuit.prove(&input);
        log::debug!("Done generating proof");

        circuit.verify(&proof, &input, &output);
    }
}
