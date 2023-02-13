use plonky2::gates::noop::NoopGate;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData, CommonCircuitData};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
use plonky2::field::extension::Extendable;

use crate::recursion::util::num_targets_for_circuit_set;
use crate::recursion::wrap_circuit::WrapCircuit;

// degree bits of a base circuit guaranteeing that 2 wrap steps are necessary to shrink a proof
// generated for such a circuit up to the recursion threshold
const SHRINK_LIMIT: usize = 15;

fn dummy_circuit<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    config: CircuitConfig,
    num_gates: usize,
    num_public_inputs: usize,
) -> CircuitData<F, C, D>
{
    let mut builder = CircuitBuilder::new(config);
    for _ in 0..num_public_inputs {
        let target = builder.add_virtual_target();
        builder.register_public_input(target);
    }
    // pad the number of gates of the circuit up to `num_gates` with noop operations
    for _ in 0..(num_gates - num_public_inputs) {
        builder.add_gate(NoopGate, vec![]);
    }

    builder.build::<C>()
}

pub(crate) fn build_data_for_recursive_aggregation<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    config: CircuitConfig,
    num_public_inputs: usize,
) -> CommonCircuitData<F, D>
where
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    let num_public_inputs = num_public_inputs + num_targets_for_circuit_set::<F, D>(config.clone());
    let circuit_data =
        dummy_circuit::<F, C, D>(config.clone(), 1 << SHRINK_LIMIT, num_public_inputs);

    let wrap_circuit = WrapCircuit::<F, C, D>::build_wrap_circuit(
        &circuit_data.verifier_only,
        &circuit_data.common,
        &config,
    );

    wrap_circuit.final_proof_circuit_data().common.clone()
}

#[cfg(test)]
mod test {
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use rstest::rstest;

    use crate::recursion::common_data_for_recursion::build_data_for_recursive_aggregation;
    use crate::recursion::test_circuits::logger;
    use crate::recursion::RECURSION_THRESHOLD;

    #[rstest]
    fn test_common_data_for_recursion(_logger: ()) {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let cd = build_data_for_recursive_aggregation::<F, C, D>(
            CircuitConfig::standard_recursion_config(),
            3,
        );

        assert_eq!(dbg!(cd).degree_bits(), RECURSION_THRESHOLD);
    }
}
