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

    let mut cur_target = builder.one();
    for i in 2..101 {
        let i_target = builder.constant(F::from_canonical_u32(i));
        cur_target = builder.mul(cur_target, i_target);
    }
    builder.register_public_input(cur_target);

    let fact_100 = F::from_canonical_u64(3822706312645553057);
    let fact_100_target = builder.constant(fact_100);
    builder.connect(fact_100_target, cur_target);

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;

    println!("100 factorial (mod |F|) is: {}", proof.public_inputs[0]);

    data.verify(proof)
}
