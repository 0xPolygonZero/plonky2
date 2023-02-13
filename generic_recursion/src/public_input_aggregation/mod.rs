//! `public_input_aggregation` module provides the utilities to allow users to specify the format
//! of the public inputs of their base circuits.
//! In particular, the module provides the `PublicInputAggregation` trait, which defines the
//! information about the public input format needed to aggregate proofs whose public inputs have
//! such format, and several implementations of such trait, corresponding to different public
//! input formats.
//!

use plonky2::hash::hash_types::RichField;
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::GenericConfig;
use plonky2::field::extension::Extendable;
use anyhow::Result;
use itertools::Itertools;

pub mod shared_state;

// aggregate `public_input` to `aggregated_input` if and only if `condition == true`
pub(crate) fn conditionally_aggregate_public_input<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    PI: PublicInputAggregation,
>(
    builder: &mut CircuitBuilder<F, D>,
    condition: &BoolTarget,
    aggregated_input: &PI,
    public_input: &PI,
) -> Result<PI> {
    let updated_input = aggregated_input.aggregate_public_input(builder, &public_input);
    PI::try_from_public_input_targets(
        aggregated_input
            .get_targets()
            .into_iter()
            .zip(updated_input.get_targets().into_iter())
            .map(|(agg_target, upd_target)| builder.select(*condition, upd_target, agg_target))
            .collect::<Vec<_>>()
            .as_slice(),
    )
}

/// `check_consistency_of_dummy_public_inputs_aggregation` is meant to be employed to test, for an
/// implementation `PI` of the trait `PublicInputAggregation`, the consistency between the aggregation
/// logic of public inputs and the implementations of the methods of the trait dealing with dummy
/// proofs (i.e., `dummy_circuit_inputs_logic`, `set_dummy_circuit_inputs`,
/// `can_aggregate_public_inputs_of_dummy_proofs`): indeed, the default implementations found in the
/// `PublicInputAggregation` trait for such methods will not work with any public input aggregation
/// strategy; for instance, though public inputs of dummy proofs are never aggregated when such
/// default implementations are employed, if the aggregation strategy requires some constraints on
/// the public inputs of proofs to be aggregated, then the default implementations will not be able
/// to generate public inputs fulfilling such constraints, hence leading to a failure in proof
/// aggregation.
/// This function is meant to help implementors of `PI` spotting such incompatibility: if the function
/// yields an error or return `false`, then the implementations of the methods of `PI` dealing with
/// dummy proofs might need to be fixed in order to ensure compatibility with the public input
/// aggregation logic
pub fn check_consistency_of_dummy_public_inputs_aggregation<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    PI: PublicInputAggregation,
>() -> Result<bool>
{
    // Step 1: generate a non-dummy proof
    let config = CircuitConfig::standard_recursion_config();

    let mut builder = CircuitBuilder::<F, D>::new(config.clone());
    let mut pw = PartialWitness::new();
    let pi_targets = (0..PI::num_public_inputs())
        .map(|_| {
            let pi_t = builder.add_virtual_target();
            pw.set_target(pi_t, F::rand());
            pi_t
        })
        .collect::<Vec<_>>();
    let public_inputs = PI::try_from_public_input_targets(pi_targets.as_slice())?;
    public_inputs.register_public_inputs(&mut builder);

    let real_circuit_data = builder.build::<C>();

    let real_proof = real_circuit_data.prove(pw)?;
    let real_public_inputs = real_proof.public_inputs.clone();

    // Step 2: build dummy circuit according to the logic in PI trait
    let mut builder = CircuitBuilder::<F,D>::new(config.clone());
    let public_inputs = PI::dummy_circuit_inputs_logic(&mut builder);

    let dummy_circuit_data = builder.build::<C>();

    // generate 2 dummy proofs
    let mut to_be_aggregated_proofs = vec![real_proof];
    for _ in 0..2 {
        let mut pw = PartialWitness::new();
        PI::set_dummy_circuit_inputs(
            to_be_aggregated_proofs.last().unwrap().public_inputs.as_slice(),
            &public_inputs,
            &mut pw,
        );
        to_be_aggregated_proofs.push(dummy_circuit_data.prove(pw)?)
    }

    // Build a circuit that aggregates the public inputs of the non-dummy proofs with the ones of
    // the 2 dummy proofs
    let mut aggregate_circuit_builder = CircuitBuilder::<F, D>::new(config.clone());
    let mut aggregate_circuit_pw = PartialWitness::new();
    let public_input_targets = to_be_aggregated_proofs
        .iter()
        .map(|proof| {
            let targets = aggregate_circuit_builder.add_virtual_targets(PI::num_public_inputs());
            for (input, target) in proof.public_inputs.iter().zip(targets.iter()) {
                aggregate_circuit_pw.set_target(*target, *input);
            }
            targets
        })
        .collect::<Vec<_>>();

    let mut aggregation_input = PI::try_from_public_input_targets(public_input_targets[0].as_slice())?;
    let selector = aggregate_circuit_builder._false();
    aggregation_input =
        public_input_targets[1..]
            .iter()
            .fold(Ok(aggregation_input), |agg_input, targets| {
                let input = PI::try_from_public_input_targets(targets.as_slice())?;
                if PI::can_aggregate_public_inputs_of_dummy_proofs() {
                    Ok(agg_input?.aggregate_public_input(&mut aggregate_circuit_builder, &input))
                } else {
                    conditionally_aggregate_public_input::<F, C, D, PI>(
                        &mut aggregate_circuit_builder,
                        &selector,
                        &agg_input?,
                        &input,
                    )
                }
            })?;
    aggregation_input.register_public_inputs(&mut aggregate_circuit_builder);

    let aggregation_circuit_data = aggregate_circuit_builder.build::<C>();

    let proof = aggregation_circuit_data.prove(aggregate_circuit_pw)?;

    // check that the public inputs of the dummy proofs were not aggregated, which means that the
    // public inputs of the proof generated with the aggregation circuit are equal to the public
    // inputs of the non-dummy proof
    let are_pi_equals = proof.public_inputs == real_public_inputs;

    aggregation_circuit_data.verify(proof)?;

    Ok(are_pi_equals)
}

/// `PublicInputAggregation` can be implemented for a type of public inputs employed in a circuit to
/// specify how to aggregate public inputs of different proofs all having this type of public inputs.
/// This trait should be implemented for data-structures that collect all the public input targets
/// employed for the given type of public inputs, providing a set of operations, to be performed
/// inside a circuit, that allow to aggregate multiple instances of such data-structure to a single
/// one, which will represent the public inputs of an aggregated proof.
pub trait PublicInputAggregation: Sized {
    type TargetList: IntoIterator<Item = Target>;
    /// Return to the caller the number of public inputs for this scheme
    fn num_public_inputs() -> usize;

    /// Method to register the targets found in `self` as public inputs
    /// of a proof
    fn register_public_inputs<F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
    );

    /// Method to add the logic related to the public input scheme for a dummy circuit with builder
    /// `builder`.
    /// The dummy circuit enforces a trivial statement while being
    /// compliant with the public input scheme specified by `Self`.
    fn dummy_circuit_inputs_logic<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F,D>,
    ) -> Self {
        let targets = (0..Self::num_public_inputs())
            .map(|_| builder.add_virtual_target())
            .collect::<Vec<_>>();
        let public_inputs = Self::try_from_public_input_targets(targets.as_slice()).unwrap();
        public_inputs.register_public_inputs(builder);
        // build the circuit
        public_inputs
    }

    /// Method employed by the builder of the merge circuit to determine
    /// if the public inputs of dummy proofs can be safely aggregated with
    /// public inputs of non-dummy proofs
    fn can_aggregate_public_inputs_of_dummy_proofs() -> bool {
        false
    }

    /// Method to set the input values of the dummy circuit employing a set of
    /// input values; it expects `input_values` to have the same size of the number of targets
    /// employed in `input_targets`, that is `Self::num_public_inputs()`
    fn set_dummy_circuit_inputs<
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        input_values: &[F],
        input_targets: &Self,
        pw: &mut PartialWitness<F>,
    ) {
        let input_target_list = input_targets.get_targets();
        for (target, input) in input_target_list
            .into_iter()
            .zip_eq(input_values.iter())
        {
            pw.set_target(target, input.clone());
        }
    }

    /// Retrieve the list of public inputs targets in the same order they are
    /// appended in a `ProofWithPublicInputs` data structure
    fn get_targets(&self) -> Self::TargetList;

    /// Specify how to map the public input targets found in
    /// `ProofWithPublicInputsTarget` to the targets in `Self`.
    /// The function expects exactly `Self::num_public_inputs()` targets as input, it should return
    /// an error otherwise
    fn try_from_public_input_targets(targets: &[Target]) -> Result<Self>;

    /// Specify how to aggregate a public input to another public input; this
    /// forces user to specify a public input aggregation scheme which is
    /// incremental
    fn aggregate_public_input<F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        public_input: &Self,
    ) -> Self;

    /// Aggregates the public inputs of a set of proofs in the public inputs of
    /// the recursive proof; can be overridden by the user in case it is more
    /// efficient (in terms of number of gates) to aggregate in a single step all the public inputs
    /// instead of aggregating one public input at a time
    fn aggregate_public_inputs<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
        mut public_inputs: impl Iterator<Item = Self>,
    ) {
        // check that there are at least 2 public inputs to be aggregated
        let first_public_input = public_inputs.next();
        let second_public_input = public_inputs.next();
        assert!(first_public_input.is_some());
        assert!(second_public_input.is_some());
        let mut aggregate_input = first_public_input
            .unwrap()
            .aggregate_public_input(builder, &second_public_input.unwrap());
        for input in public_inputs {
            aggregate_input = aggregate_input.aggregate_public_input(builder, &input);
        }
        aggregate_input.register_public_inputs(builder);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use plonky2::hash::hash_types::{HashOutTarget, RichField};
    use plonky2::field::extension::Extendable;
    use plonky2::iop::target::Target;
    use anyhow::Result;
    use plonky2::hash::poseidon::PoseidonHash;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use crate::public_input_aggregation::check_consistency_of_dummy_public_inputs_aggregation;

    use super::PublicInputAggregation;

    /** "Toy" public input aggregation scheme to be employed to test the conditional aggregation of
     public inputs for dummy proofs, relying on the default implementations of the
     `PublicInputAggregation` trait. Specifically, this public input scheme accumulates public
     inputs and public outputs of base proofs by hashing them, that is the input/output accumulators
     of an aggregated proof will be the hash of the inputs/outputs of the base proofs.
     To have a consistent public input interface among base proofs and aggregated proofs, the public input
     format requires that the input and the output public inputs of a base proof are hash
     themselves, that is they correspond to the hash of the corresponding input/output values,
     which are instead provided as witnesses.
    */
    pub(crate) struct PublicInputAccumulator {
        input_accumulator: HashOutTarget,
        output_accumulator: HashOutTarget,
    }

    impl PublicInputAccumulator {
        pub(crate) fn new(input_accumulator: HashOutTarget, output_accumulator: HashOutTarget)
            -> Self {
            Self {
                input_accumulator,
                output_accumulator,
            }
        }
    }

    impl PublicInputAggregation for PublicInputAccumulator {
        type TargetList = Vec<Target>;

        fn num_public_inputs() -> usize {
            let hash_target = HashOutTarget::from_partial(vec![].as_slice(), Target::wire(0, 0));
            2 * hash_target.elements.len()
        }

        fn register_public_inputs<F: RichField + Extendable<D>, const D: usize>
        (
            &self,
            builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        ) {
            builder.register_public_inputs(&self.input_accumulator.elements);
            builder.register_public_inputs(&self.output_accumulator.elements);
        }

        fn get_targets(&self) -> Self::TargetList {
            self.input_accumulator.elements.into_iter()
                .chain(self.output_accumulator.elements.into_iter())
                .collect()
        }

        fn try_from_public_input_targets(targets: &[Target]) -> Result<Self> {
            if targets.len() != Self::num_public_inputs() {
                Err(anyhow::Error::msg(format!("expected {} targets to build PublicInputAccumulator, found {}", Self::num_public_inputs(), targets.len())))
            } else {
                let num_hash_elements = Self::num_public_inputs() / 2;
                let input_accumulator = HashOutTarget::try_from(&targets[..num_hash_elements]).unwrap();
                let output_accumulator = HashOutTarget::try_from(&targets[num_hash_elements..]).unwrap();
                Ok(
                    Self {
                        input_accumulator,
                        output_accumulator,
                    }
                )
            }
        }

        fn aggregate_public_input<F: RichField + Extendable<D>, const D: usize>(
            &self,
            builder: &mut CircuitBuilder<F, D>,
            public_input: &Self,
        ) -> Self {
            let mut input_accumulator = self.input_accumulator.elements.to_vec();
            input_accumulator.extend_from_slice(
                &public_input.input_accumulator.elements
            );
            let aggregated_input = builder.hash_n_to_hash_no_pad::<PoseidonHash>(input_accumulator);
            let mut output_accumulator = self.output_accumulator.elements.to_vec();
            output_accumulator.extend_from_slice(
                &public_input.output_accumulator.elements
            );
            let aggregated_output = builder.hash_n_to_hash_no_pad::<PoseidonHash>(output_accumulator);
            Self {
                input_accumulator: aggregated_input,
                output_accumulator: aggregated_output,
            }
        }
    }


    #[test]
    fn test_consistency_public_input_accumulator() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        assert!(check_consistency_of_dummy_public_inputs_aggregation::<
            F,
            C,
            D,
            PublicInputAccumulator,
        >()
            .unwrap());
    }
}
