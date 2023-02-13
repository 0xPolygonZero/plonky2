#![feature(generic_const_exprs)]

use std::iter::once;
use generic_recursion::public_input_aggregation::PublicInputAggregation;
use plonky2::iop::target::Target;
use generic_recursion::public_input_aggregation::shared_state::{SharedStatePublicInput, State};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData, VerifierCircuitData};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher, PoseidonGoldilocksConfig};
use plonky2::plonk::proof::ProofWithPublicInputs;
use plonky2::field::extension::Extendable;
use anyhow::{Error, Result};
use rand::{Rng, thread_rng};
use generic_recursion::recursion::{BaseCircuitInfo, build_verifier_circuit_data, PreparedProof, RecursionCircuit, prepare_base_circuit_for_circuit_set};
use generic_recursion::{AggregationScheme, PreparedProofForAggregation};
use plonky2::plonk::plonk_common::reduce_with_powers_circuit;
use plonky2::field::types::Sample;
use plonky2_u32::gadgets::arithmetic_u32::{CircuitBuilderU32, U32Target};
use plonky2_u32::gadgets::range_check::range_check_u32_circuit;
use plonky2_u32::witness::WitnessU32;

 /*
    Implement a public input aggregation scheme for a shared state where the state is a set of
    `NUM_EL` targets
 */
// underlying type representing the state
struct VectorState<const NUM_EL: usize>([Target; NUM_EL]);

impl<const NUM_EL: usize> FromIterator<Target> for VectorState<NUM_EL> {
    fn from_iter<T: IntoIterator<Item=Target>>(iter: T) -> Self {
        let state = iter.into_iter().take(NUM_EL).collect::<Vec<_>>();
        VectorState(
            state.try_into().unwrap()
        )
    }
}

impl<const NUM_EL: usize> TryFrom<&[Target]> for VectorState<NUM_EL> {
    type Error = Error;

    fn try_from(targets: &[Target]) -> std::result::Result<Self, Self::Error> {
        if targets.len() != Self::num_targets() {
            Err(anyhow::Error::msg(format!("expected {} targets to build VectorState, found {}", Self::num_targets(), targets.len())))
        } else {
            let targets_array = targets.try_into()?;
            Ok(Self(targets_array))
        }
    }
}

impl<const NUM_EL: usize> State for VectorState<NUM_EL> {
    fn num_targets() -> usize {
        NUM_EL
    }

    fn to_vec(&self) -> Vec<Target> {
        self.0.to_vec()
    }
}

type VectorStatePublicInput<const NUM_EL: usize> = SharedStatePublicInput<VectorState<NUM_EL>>;

// Data structure for a base circuit employing a state as a public input given by `STATE_LEN`
// field elements
const STATE_LEN: usize = 4;
struct BaseCircuit<
F: RichField + Extendable<D>,
C: GenericConfig<D, F = F>,
const D: usize,
> {
    public_input: VectorStatePublicInput<STATE_LEN>,
    private_input: [Target; STATE_LEN],
    circuit_data: CircuitData<F,C,D>
}

impl<
F: RichField + Extendable<D>,
C: GenericConfig<D, F = F>,
const D: usize,
> BaseCircuit<F,C,D> {
    fn build_base_circuit(config: CircuitConfig) -> Self
        where
            C::Hasher: AlgebraicHasher<F>,
    {
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());

        let mut input_state_targets = builder.add_virtual_targets(STATE_LEN);

        let mut intermediate_state_targets = builder.add_virtual_targets(STATE_LEN);

        let private_input_targets = builder.add_virtual_targets(STATE_LEN);

        for (&input_t, &intermediate_t) in input_state_targets.iter().zip(intermediate_state_targets.iter()) {
            builder.generate_copy(input_t, intermediate_t);
            builder.connect(input_t, intermediate_t);
        }

        const NUM_ROUNDS: usize = 1 << 10;

        for i in 0..NUM_ROUNDS {
            intermediate_state_targets = intermediate_state_targets.iter().zip(private_input_targets.iter()).map(|(&intermediate_t, &input_t)| {
                if i % 2 == 0 {
                    builder.exp(intermediate_t, input_t, F::BITS)
                } else {
                    builder.mul(intermediate_t, input_t)
                }
            }).collect::<Vec<_>>();
            intermediate_state_targets = builder.hash_n_to_m_no_pad::<C::Hasher>(intermediate_state_targets, STATE_LEN);
        }

        input_state_targets.extend_from_slice(intermediate_state_targets.as_slice());

        let public_input_targets = VectorStatePublicInput::try_from_public_input_targets(input_state_targets.as_slice()).unwrap();

        public_input_targets.register_public_inputs(&mut builder);

        let data = builder.build::<C>();

        Self {
            public_input: public_input_targets,
            private_input: private_input_targets.try_into().unwrap(),
            circuit_data: data,
        }
    }

    fn generate_base_proof(&self, init_state: [F; STATE_LEN])
        -> Result<ProofWithPublicInputs<F,C,D>> {
        let mut pw = PartialWitness::<F>::new();

        for (&target, &value) in self.public_input.get_targets().iter().take(STATE_LEN).zip(init_state.iter()) {
            pw.set_target(target, value);
        }

        for target in self.private_input {
            pw.set_target(target, F::rand());
        }

        self.circuit_data.prove(pw)
    }
}

// Data structure for a second base circuit, which employs `STATE_LEN` `U32Target`s as the public
// input shared state
struct BaseCircuitU32 <
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    public_input: VectorStatePublicInput<STATE_LEN>,
    private_input: [U32Target; STATE_LEN],
    circuit_data: CircuitData<F,C,D>
}

impl<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> BaseCircuitU32<F,C,D> {

    // Utility function that computes a target out of the 32 least significant bits of the input
    // target
    fn truncate_target_to_u32(builder: &mut CircuitBuilder<F,D>, target: Target) -> Target {
        const B_BITS: usize = 2;
        const B: usize = 1 << B_BITS;
        let least_significant_limbs = &builder.split_le_base::<B>(target, (64+B_BITS-1)/B_BITS)[..(32+B_BITS-1)/B_BITS];
        let four = builder.constant(F::from_canonical_u64(B as u64));
        reduce_with_powers_circuit(builder, least_significant_limbs, four)
    }

    fn build_base_circuit(config: CircuitConfig) -> Self
        where
            C::Hasher: AlgebraicHasher<F>,
    {
        let mut builder = CircuitBuilder::<F, D>::new(config.clone());

        let mut input_state_targets = builder.add_virtual_targets(STATE_LEN);

        let private_input_targets = builder.add_virtual_u32_targets(STATE_LEN);

        range_check_u32_circuit(&mut builder, private_input_targets.clone());
        
        let mut intermediate_state_targets = input_state_targets.iter().map(|&input_t| {
            let target = Self::truncate_target_to_u32(&mut builder, input_t);
            range_check_u32_circuit(&mut builder, vec![U32Target(target)]);
            target
        }).collect::<Vec<_>>();

        const NUM_ROUNDS: usize = 1 << 9;

        for i in 0..NUM_ROUNDS {
            intermediate_state_targets = intermediate_state_targets.iter().zip(private_input_targets.iter()).map(|(&intermediate_t, &input_t)| {
                if i % 2 == 0 {
                    builder.add_u32(U32Target(intermediate_t), input_t).0
                } else {
                    builder.mul_u32(U32Target(intermediate_t), input_t).0
                }.0
            }).collect::<Vec<_>>();
            intermediate_state_targets =
                builder.hash_n_to_m_no_pad::<C::Hasher>(intermediate_state_targets, STATE_LEN).iter().map(|&target|
                Self::truncate_target_to_u32(&mut builder, target)
                ).collect();
        }

        input_state_targets.extend_from_slice(intermediate_state_targets.as_slice());

        let public_input_targets = VectorStatePublicInput::try_from_public_input_targets(input_state_targets.as_slice()).unwrap();

        public_input_targets.register_public_inputs(&mut builder);

        let data = builder.build::<C>();

        Self {
            public_input: public_input_targets,
            private_input: private_input_targets.try_into().unwrap(),
            circuit_data: data,
        }
    }

    fn generate_base_proof(&self, init_state: [F; STATE_LEN])
                           -> Result<ProofWithPublicInputs<F,C,D>> {
        let mut pw = PartialWitness::<F>::new();

        for (&target, &value) in self.public_input.get_targets().iter().take(STATE_LEN).zip(init_state.iter()) {
            pw.set_target(target, value);
        }

        for target in self.private_input {
            pw.set_u32_target(target, thread_rng().gen());
        }

        self.circuit_data.prove(pw)
    }
}

// Implement `BaseCircuitInfo` trait for `BaseCircuit` to allow aggregation of proofs of this
// circuit with the recursive aggregation circuit
impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
BaseCircuitInfo<F, C, D> for &BaseCircuit<F,C,D> {
    type PIScheme = VectorStatePublicInput<STATE_LEN>;

    fn get_verifier_circuit_data(&self) -> VerifierCircuitData<F, C, D> {
        build_verifier_circuit_data(&self.circuit_data)
    }
}
// Implement `BaseCircuitInfo` trait for `BaseCircuitU32` to allow aggregation of proofs of this
// circuit with the recursive aggregation circuit
impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
BaseCircuitInfo<F, C, D> for &BaseCircuitU32<F,C,D> {
    type PIScheme = VectorStatePublicInput<STATE_LEN>;

    fn get_verifier_circuit_data(&self) -> VerifierCircuitData<F, C, D> {
        build_verifier_circuit_data(&self.circuit_data)
    }
}

// multiple checks to ensure the validity of an aggregated proof for a
// `SharedStatePublicInput` aggregation scheme employing `ST` as state representation
fn check_aggregated_proof<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    const D: usize,
    ST: State,
>(
    aggregated_proof: &PreparedProofForAggregation<F, C, D>,
    aggregation_scheme: &AggregationScheme<F, C, D, SharedStatePublicInput<ST>>,
    init: Vec<F>,
    final_state: Vec<F>,
) where
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    aggregation_scheme
        .verify_aggregated_proof(aggregated_proof.clone())
        .unwrap();
    let aggregated_proof = aggregated_proof.get_proof();

    assert_eq!(aggregated_proof.public_inputs[..ST::num_targets()], init);
    assert_eq!(
        aggregated_proof.public_inputs
            [ST::num_targets()..SharedStatePublicInput::<ST>::num_public_inputs()],
        final_state
    );
}

#[test]
fn test_recursive_aggregation() {
    const D: usize = 2;
    type PC = PoseidonGoldilocksConfig;
    type F = <PC as GenericConfig<D>>::F;
    env_logger::init();

    let config = CircuitConfig::standard_recursion_config();

    let mut rng = thread_rng();

    let base_circuit = BaseCircuit::<F,PC,D>::build_base_circuit(config.clone());
    log::info!("base circuit size: {}", base_circuit.circuit_data.common.degree_bits());
    let base_circuit_u32 = BaseCircuitU32::<F,PC,D>::build_base_circuit(config.clone());
    log::info!("base circuit u32 size: {}", base_circuit.circuit_data.common.degree_bits());
    let base_vd = (&base_circuit).get_verifier_circuit_data();
    let base_u32_vd = (&base_circuit_u32).get_verifier_circuit_data();

    let init_state = (0..STATE_LEN).map(|_| F::rand()).collect::<Vec<_>>().try_into().unwrap();

    const NUM_PROOFS: usize = 8;
    let mut state = init_state;
    let base_proofs = (0..NUM_PROOFS).map(|i| {
        let (proof, vd) = if rng.gen() {
            (
                base_circuit.generate_base_proof(state).unwrap(),
                &base_vd
            )
        } else {
            (
                base_circuit_u32.generate_base_proof(state).unwrap(),
                &base_u32_vd,
            )
        };
        log::info!("generated {}-th base proof", i + 1);
        state = proof.public_inputs[STATE_LEN..].to_vec().try_into().unwrap();
        vd.verify(proof.clone()).unwrap();
        (proof, vd)
    }).collect::<Vec<_>>();

    let circuit_set
    = vec![prepare_base_circuit_for_circuit_set(&base_circuit),
           prepare_base_circuit_for_circuit_set(&base_circuit_u32)];

    let mut aggregation_scheme = AggregationScheme::build_circuit(
        circuit_set.into_iter()
    ).unwrap();

    for (proof, vd) in base_proofs {
        let prepared_proof = aggregation_scheme.prepare_proof_for_aggregation(proof, vd).unwrap();
        aggregation_scheme = aggregation_scheme.add_proofs_for_aggregation(once(prepared_proof));
    }

    let (aggregation_scheme, aggregated_proof) = aggregation_scheme.aggregate_proofs().unwrap();


    check_aggregated_proof::<_, _, D, VectorState<STATE_LEN>>(
        &aggregated_proof,
        &aggregation_scheme,
        init_state.to_vec(),
        state.to_vec(),
    );

}