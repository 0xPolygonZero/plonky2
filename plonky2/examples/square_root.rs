use std::marker::PhantomData;

use anyhow::Result;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartialWitness, PartitionWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use plonky2_field::extension::Extendable;

#[derive(Debug)]
struct SquareGenerator<F: RichField + Extendable<D>, const D: usize> {
    x: Target,
    x_squared: Target,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F> for SquareGenerator<F, D> {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.x]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let x = witness.get_target(self.x);
        let x_squared = x * x;

        out_buffer.set_target(self.x_squared, x_squared);
    }
}

fn main() -> Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig::standard_recursion_config();

    let mut builder = CircuitBuilder::<F, D>::new(config);

    let x = builder.add_virtual_target();
    let x_squared = builder.mul(x, x);

    builder.register_public_input(x);
    builder.register_public_input(x_squared);

    builder.add_simple_generator(SquareGenerator::<F, D> {
        x,
        x_squared,
        _phantom: PhantomData,
    });

    let x_value = F::rand();

    let mut pw = PartialWitness::new();
    pw.set_target(x, x_value);

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;

    println!("Random field element: {}", proof.public_inputs[1]);
    println!("Its square root: {}", proof.public_inputs[0]);

    data.verify(proof)
}
