use anyhow::Result;
use plonky2::field::types::Field;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig, PoseidonBn254GoldilocksConfig};

/// An example of using Plonky2 to prove a statement of the form
/// "I know n * (n + 1) * ... * (n + 99)".
/// When n == 1, this is proving knowledge of 100!.
fn main() -> Result<()> {
    const D: usize = 2;
    type C = PoseidonBn254GoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    // The arithmetic circuit.
    let initial = builder.add_virtual_target();
    let mut cur_target = initial;
    for i in 2..101 {
        let i_target = builder.constant(F::from_canonical_u32(i));
        cur_target = builder.mul(cur_target, i_target);
    }

    // Public inputs are the initial value (provided below) and the result (which is generated).
    builder.register_public_input(initial);
    builder.register_public_input(cur_target);

    let mut pw = PartialWitness::new();
    pw.set_target(initial, F::ONE);

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;

    println!(
        "Factorial starting at {} is {}",
        proof.public_inputs[0], proof.public_inputs[1]
    );

    data.verify(proof)
}
