use anyhow::Result;
use plonky2::field::types::Field;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig, PoseidonBn254GoldilocksConfig};

/// An example of using Plonky2 to prove that a given value lies in a given range.
fn main() -> Result<()> {
    const D: usize = 2;
    type C = PoseidonBn254GoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    // The secret value.
    let value = builder.add_virtual_target();
    builder.register_public_input(value);

    let log_max = 6;
    builder.range_check(value, log_max);

    let mut pw = PartialWitness::new();
    pw.set_target(value, F::from_canonical_usize(42));

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;

    println!(
        "Value {} is less than 2^{}",
        proof.public_inputs[0], log_max,
    );

    data.verify(proof)
}
