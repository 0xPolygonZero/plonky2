use std::marker::PhantomData;

use anyhow::Result;
use plonky2::field::types::{Field, PrimeField};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartialWitness, PartitionWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use plonky2_field::extension::Extendable;

/// A generator used by the prover to calculate the square root (`x`) of a given value
/// (`x_squared`), outside of the circuit, in order to supply it as an additional public input.
#[derive(Debug)]
struct SquareRootGenerator<F: RichField + Extendable<D>, const D: usize> {
    x: Target,
    x_squared: Target,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for SquareRootGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        vec![self.x_squared]
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let x_squared = witness.get_target(self.x_squared);
        let x = x_squared.sqrt().unwrap();

        println!("Square root: {x}");

        out_buffer.set_target(self.x, x);
    }
}

/// An example of using Plonky2 to prove a statement of the form
/// "I know the square root of this field element."
fn main() -> Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig::standard_recursion_config();

    let mut builder = CircuitBuilder::<F, D>::new(config);

    let x = builder.add_virtual_target();
    let x_squared = builder.square(x);

    builder.register_public_input(x_squared);

    builder.add_simple_generator(SquareRootGenerator::<F, D> {
        x,
        x_squared,
        _phantom: PhantomData,
    });

    // Randomly generate the value of x^2: any quadratic residue in the field works.
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
    let proof = data.prove(pw.clone())?;

    let x_squared_actual = proof.public_inputs[0];
    println!("Field element (square): {x_squared_actual}");

    data.verify(proof)
}
