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

    let mut prev_target = builder.zero();
    let mut cur_target = builder.one();
    for _ in 0..99 {
        let temp = builder.add(prev_target, cur_target);
        prev_target = cur_target;
        cur_target = temp;
    }

    let fib_100 = F::from_canonical_u64(3736710860384812976);
    let fib_100_target = builder.constant(fib_100);
    builder.register_public_input(fib_100_target);

    builder.connect(fib_100_target, cur_target);

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;

    println!(
        "100th Fibonacci number (mod |F|) is: {}",
        proof.public_inputs[0]
    );

    data.verify(proof)
}
