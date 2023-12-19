use core::marker::PhantomData;

use anyhow::Result;
use env_logger::{DEFAULT_FILTER_ENV, try_init_from_env, Env};
use plonky2::field::types::{PrimeField, Sample};
use plonky2::gates::arithmetic_base::ArithmeticBaseGenerator;
use plonky2::gates::poseidon::{PoseidonGenerator, PoseidonGate};
use plonky2::gates::poseidon_mds::PoseidonMdsGenerator;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::poseidon::{Poseidon, SPONGE_WIDTH};
use plonky2::iop::generator::{
    ConstantGenerator, GeneratedValues, RandomValueGenerator, SimpleGenerator, generate_partial_witness,
};
use plonky2::iop::target::Target;
use plonky2::iop::wire::Wire;
use plonky2::iop::witness::{PartialWitness, PartitionWitness, Witness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData, CommonCircuitData};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig};
use plonky2::recursion::dummy_circuit::DummyProofGenerator;
use plonky2::util::serialization::{
    Buffer, DefaultGateSerializer, IoResult, Read, WitnessGeneratorSerializer, Write,
};
use plonky2::util::timing::TimingTree;
use plonky2::{get_generator_tag_impl, impl_generator_serializer, read_generator_impl};
use plonky2_field::extension::Extendable;
use plonky2::field::types::Field;
use ark_std::{end_timer, start_timer};

pub struct CustomGeneratorSerializer<C: GenericConfig<D>, const D: usize> {
    pub _phantom: PhantomData<C>,
}

impl<F, C, const D: usize> WitnessGeneratorSerializer<F, D> for CustomGeneratorSerializer<C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    C::Hasher: AlgebraicHasher<F>,
{
    impl_generator_serializer! {
        CustomGeneratorSerializer,
        DummyProofGenerator<F, C, D>,
        ArithmeticBaseGenerator<F, D>,
        ConstantGenerator<F>,
        PoseidonGenerator<F, D>,
        PoseidonMdsGenerator<D>,
        RandomValueGenerator
    }
}

/// An example of using Plonky2 to prove a statement of the form
/// "I know the square root of this field element."
fn main() -> Result<()> {

    const NUM_POSEIDON: usize = 134400;
    println!("NUM_POSEIDON: {}", NUM_POSEIDON);

    // first part taken from plonky2/src/gates/poseidon.rs: test *generated_output*
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig {
        num_wires: 143,
        ..CircuitConfig::standard_recursion_config()
    };

    let mut builder = CircuitBuilder::<F, D>::new(config);

    type Gate = PoseidonGate<F, D>;
    let gate = Gate::new();
    let mut row = builder.add_gate(gate, vec![]);

    // instead of building for only the prover, we want to verify it as well
    // let circuit = builder.build_prover::<C>();

    let permutation_inputs = (0..SPONGE_WIDTH)
    .map(F::from_canonical_usize)
    .collect::<Vec<_>>();

    let mut inputs = PartialWitness::new();
    inputs.set_wire(
        Wire {
            row,
            column: Gate::WIRE_SWAP,
        },
        F::ZERO,
    );
    for i in 0..SPONGE_WIDTH {
        inputs.set_wire(
            Wire {
                row,
                column: Gate::wire_input(i),
            },
            permutation_inputs[i],
        );
    }

    //  ***************** the more poseidon circuit *****************

    for _ in 1..NUM_POSEIDON {
        let gate = Gate::new();
        row = builder.add_gate(gate, vec![]);

        inputs.set_wire(
            Wire {
                row,
                column: Gate::WIRE_SWAP,
            },
            F::ZERO,
        );

        // connect wires and enforce copy constraints
        for i in 0..SPONGE_WIDTH {
            builder.connect(
                Target::Wire(Wire {
                    row: row - 1,
                    column: Gate::wire_output(i),
                }),
                Target::Wire(Wire {
                    row,
                    column: Gate::wire_input(i),
                }),
            )
        }
    }

    //  ***************** the more poseidon circuit *****************

    // next thing is to get the results and verify it against F::poseidon
    // by using register_public_input

    for i in 0..SPONGE_WIDTH {
        builder.register_public_input(
            Target::Wire(
                Wire {
                    row,
                    column: Gate::wire_output(i),
                }
            )
        );
    }

    // build the circuit here, after setting the wirings
    let data = builder.build::<C>();
    println!("");
    let proof = data.prove(inputs)?;


    // ----------------- verify it against F::poseidon -----------------
    let mut expected_outputs: [F; SPONGE_WIDTH] =
    F::poseidon(permutation_inputs.try_into().unwrap());
    for _ in 1..NUM_POSEIDON {
        expected_outputs = F::poseidon(expected_outputs);
    }


    for i in 0..SPONGE_WIDTH {
        let out = proof.public_inputs[i];
        assert_eq!(out, expected_outputs[i]);
    }
    // ----------------- verify it against F::poseidon -----------------

    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "debug"));
    let timing = TimingTree::new("verify", log::Level::Debug);
    let result = data.verify(proof);
    println!("");
    timing.print();

    println!("proof verified");
    return result
 
    // // Test serialization
    // {
    //     let gate_serializer = DefaultGateSerializer;
    //     let generator_serializer = CustomGeneratorSerializer {
    //         _phantom: PhantomData::<C>,
    //     };

    //     let data_bytes = data
    //         .to_bytes(&gate_serializer, &generator_serializer)
    //         .map_err(|_| anyhow::Error::msg("CircuitData serialization failed."))?;

    //     let data_from_bytes = CircuitData::<F, C, D>::from_bytes(
    //         &data_bytes,
    //         &gate_serializer,
    //         &generator_serializer,
    //     )
    //     .map_err(|_| anyhow::Error::msg("CircuitData deserialization failed."))?;

    //     assert_eq!(data, data_from_bytes);
    // }

    // data.verify(proof)
}
