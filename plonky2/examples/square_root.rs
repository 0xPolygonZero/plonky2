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
use plonky2_field::goldilocks_field::GoldilocksField;

#[derive(Debug)]
struct SquareRootGenerator<F: RichField + Extendable<D>, const D: usize> {
    x: Target,
    x_squared: Target,
    _phantom: PhantomData<F>,
}

impl SimpleGenerator<GoldilocksField> for SquareRootGenerator<GoldilocksField, 2> {
    fn dependencies(&self) -> Vec<Target> {
        vec![self.x_squared]
    }

    fn run_once(
        &self,
        witness: &PartitionWitness<GoldilocksField>,
        out_buffer: &mut GeneratedValues<GoldilocksField>,
    ) {
        let x_squared = witness.get_target(self.x_squared);
        dbg!(x_squared);
        let x = x_squared.sqrt().unwrap();
        dbg!(x);

        out_buffer.set_target(self.x, x);
    }
}

/// An example of using Plonky2 to prove a statement of the form
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

    builder.add_simple_generator(SquareRootGenerator::<F, D> {
        x,
        x_squared,
        _phantom: PhantomData,
    });

    let x_squared_value = {
        let mut val = F::rand();
        while !val.is_quadratic_residue() {
            val = F::rand();
        }
        val
    };

    let mut pw = PartialWitness::new();
    pw.set_target(x_squared, x_squared_value);

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;

    let x_actual = proof.public_inputs[0];
    let x_squared_actual = proof.public_inputs[1];
    println!("Random field element: {}", x_squared_actual);
    println!("Its square root: {}", x_actual);

    assert!(x_actual * x_actual == x_squared_actual);

    data.verify(proof)
}
