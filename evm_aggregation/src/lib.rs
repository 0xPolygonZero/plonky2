use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData};
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::recursive_verifier::{add_virtual_recursive_all_proof, RecursiveAllProof};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub struct AggregationCircuitData {
    aggregation_circuit: CircuitData<F, C, D>,
}

fn wrapped_evm_circuit() -> CircuitData<F, C, D> {
    let stark_config = StarkConfig::standard_fast_config();
    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);
    let recursive_all_proof_target = add_virtual_recursive_all_proof();
    let verifier_data = todo!();
    RecursiveAllProof::verify_circuit(&mut builder, recursive_all_proof_target, verifier_data, &stark_config);
    builder.build()
}

pub fn aggregation_circuit() -> AggregationCircuitData {
    let evm_circuit = wrapped_evm_circuit();
    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    let left_is_agg = builder.add_virtual_bool_target_safe();
    let right_is_agg = builder.add_virtual_bool_target_safe();

    let common = &evm_circuit.common;
    let evm_vk_target = builder.constant_verifier_data(&evm_circuit.verifier_only);

    let left_agg_proof = builder.add_virtual_proof_with_pis::<C>(common);
    let left_evm_proof = builder.add_virtual_proof_with_pis::<C>(common);
    let right_agg_proof = builder.add_virtual_proof_with_pis::<C>(common);
    let right_evm_proof = builder.add_virtual_proof_with_pis::<C>(common);

    builder
        .conditionally_verify_cyclic_proof::<C>(
            left_is_agg,
            &left_agg_proof,
            &left_evm_proof,
            &evm_vk_target,
            common,
        )
        .expect("Failed to build cyclic recursion circuit");
    builder
        .conditionally_verify_cyclic_proof::<C>(
            right_is_agg,
            &right_agg_proof,
            &right_evm_proof,
            &evm_vk_target,
            common,
        )
        .expect("Failed to build cyclic recursion circuit");

    let aggregation_circuit = builder.build::<C>();
    AggregationCircuitData {
        aggregation_circuit,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn no_child_proofs() {
        // TODO: Test an agg proof for an empty block, which will have no child proofs.
    }

    #[test]
    fn one_txn_child() {
        // TODO: Test an agg proof for a block with one txn, which will have one child proof.
    }

    #[test]
    fn two_child_proofs() {
        // TODO: Test an agg proof for a block with one txn, which will have one child proof.
    }
}
