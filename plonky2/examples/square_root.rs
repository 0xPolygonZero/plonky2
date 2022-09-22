use anyhow::Result;
use plonky2::field::types::Field;
use plonky2::iop::witness::PartialWitness;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

fn main() -> Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig::standard_recursion_config();

    let pw = PartialWitness::new();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    let x = F::rand();
    let x_squared = x * x;
    let x_target = builder.constant(x);
    let x_squared_target = builder.constant(x_squared);

    let x_squared_computed = builder.mul(x_target, x_target);
    builder.connect(x_squared_target, x_squared_computed);

    builder.register_public_input(x_target);

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;

    println!("Random field element: {}", x_squared);
    println!("Its square root: {}", proof.public_inputs[0]);

    data.verify(proof)
}
