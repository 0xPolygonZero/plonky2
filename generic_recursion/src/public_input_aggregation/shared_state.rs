//! `shared_state` module provides several implementations of the `PublicInputAggregation` trait.
//! All these implementations refer to a public input format where there is a public input state and
//! a public output state, which is computed from the input according to the logic of a circuit;
//! they mostly differ in how such state is defined.
//!

use plonky2::hash::hash_types::{HashOutTarget, MerkleCapTarget, RichField};
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::field::extension::Extendable;

use crate::public_input_aggregation::PublicInputAggregation;

/// `State` is a trait that represents a generic public state that is managed by a set of circuits.
/// A state corresponds to a collection of targets, which will correspond to public inputs of a proof
/// generated with one of the circuits belonging to the set of circuits managing such a state.
/// This trait is employed by the `SharedStatePublicInput` to implement a generic
/// `PublicInputAggregation` scheme for circuits that take as input a public state and compute an
/// output public state according to some logic.
pub trait State: for<'a> TryFrom<&'a[Target], Error = anyhow::Error> {
    /// Return the number of targets needed to represent the `State`
    fn num_targets() -> usize;
    /// Return the set of targets representing `State`
    fn to_vec(&self) -> Vec<Target>;
}

/// `SharedStatePublicInput` is an implementation of `PublicInputAggregation` scheme for circuits
/// that take as input a public state of type `ST` and compute an output public state (of the same
/// type) according to some logic.
pub struct SharedStatePublicInput<ST: State> {
    initial_state: ST,
    end_state: ST,
}

impl<ST: State> PublicInputAggregation for SharedStatePublicInput<ST> {
    type TargetList = Vec<Target>;

    fn num_public_inputs() -> usize {
        ST::num_targets() * 2
    }

    fn register_public_inputs<F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
    ) {
        self.initial_state
            .to_vec()
            .into_iter()
            .for_each(|target| builder.register_public_input(target));

        self.end_state
            .to_vec()
            .into_iter()
            .for_each(|target| builder.register_public_input(target));
    }

    fn dummy_circuit_inputs_logic<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F,D>,
    ) -> Self {
        let (initial_state_targets, end_state_targets) = (0..ST::num_targets())
            .map(|_| {
                let initial_state_target = builder.add_virtual_target();
                let end_state_target = builder.add_virtual_target();
                builder.connect(initial_state_target, end_state_target);
                builder.generate_copy(initial_state_target, end_state_target);
                (initial_state_target, end_state_target)
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();
        let initial_state = ST::try_from(initial_state_targets.as_slice()).unwrap();
        let end_state = ST::try_from(end_state_targets.as_slice()).unwrap();
        let public_inputs = Self {
            initial_state,
            end_state,
        };
        public_inputs.register_public_inputs(builder);
        // build the circuit
        public_inputs
    }

    fn can_aggregate_public_inputs_of_dummy_proofs() -> bool {
        true
    }

    fn set_dummy_circuit_inputs<
        F: RichField + Extendable<D>,
        const D: usize,
    >(
        input_values: &[F],
        input_targets: &Self,
        pw: &mut PartialWitness<F>,
    ) {
        assert_eq!(input_values.len(), Self::num_public_inputs());
        input_targets
            .initial_state
            .to_vec()
            .into_iter()
            .enumerate()
            .for_each(|(i, target)| {
                pw.set_target(
                    target,
                    input_values[ST::num_targets() + i].clone(),
                );
            })
    }

    fn get_targets(&self) -> Self::TargetList {
        self.initial_state
            .to_vec()
            .into_iter()
            .chain(self.end_state.to_vec().into_iter())
            .collect::<Vec<_>>()
    }

    fn try_from_public_input_targets(targets: &[Target]) -> anyhow::Result<Self> {
        if targets.len() != Self::num_public_inputs() {
            Err(anyhow::Error::msg(format!("expected {} targets to build SharedStatePublicInput, found {}", Self::num_public_inputs(), targets.len())))
        } else {
            Ok(
                Self {
                    initial_state: ST::try_from(&targets[..ST::num_targets()])?,
                    end_state: ST::try_from(&targets[ST::num_targets()..])?,
                }
            )
        }
    }

    fn aggregate_public_input<F: RichField + Extendable<D>, const D: usize>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        public_input: &Self,
    ) -> Self {
        self.end_state
            .to_vec()
            .into_iter()
            .zip(public_input.initial_state.to_vec().into_iter())
            .for_each(|(end_state_target, initial_state_target)| {
                builder.connect(end_state_target, initial_state_target)
            });
        Self {
            initial_state: ST::try_from(self.initial_state.to_vec().as_slice()).unwrap(),
            end_state: ST::try_from(public_input.end_state.to_vec().as_slice()).unwrap(),
        }
    }
}

/// Data structure representing a simple state which is made only by a single target
#[repr(transparent)]
pub struct SimpleState(Target);

impl FromIterator<Target> for SimpleState {
    fn from_iter<T: IntoIterator<Item = Target>>(iter: T) -> Self {
        let target = iter.into_iter().next();
        assert!(target.is_some());
        SimpleState(target.unwrap())
    }
}

impl TryFrom<&[Target]> for SimpleState {
    type Error = anyhow::Error;

    fn try_from(value: &[Target]) -> Result<Self, Self::Error> {
        if value.len() != 1 {
            Err(anyhow::Error::msg(format!("expected 1 target to build SimpleState, found {}", value.len())))
        } else {
            Ok(Self(*value.first().unwrap()))
        }
    }
}

impl State for SimpleState {

    fn num_targets() -> usize {
        1
    }

    fn to_vec(&self) -> Vec<Target> {
        vec!(self.0)
    }
}

/// Type alias for the `SharedStatePublicInput` scheme employing `SimpleState` as state
/// representation
pub type SimpleStatePublicInput = SharedStatePublicInput<SimpleState>;

/// Data structure representing a state given by a Merkle-tree; the state is given by the Merkle-cap
/// of such a Merkle-tree
pub struct MerkleRootState<const CAP_HEIGHT: usize>(MerkleCapTarget);

impl<const CAP_HEIGHT: usize> TryFrom<&[Target]> for MerkleRootState<CAP_HEIGHT> {
    type Error = anyhow::Error;

    fn try_from(targets: &[Target]) -> Result<Self, Self::Error> {
        if targets.len() != Self::num_targets() {
            Err(anyhow::Error::msg(format!("expected {} targets to build MerkleRootState, found {}", Self::num_targets(), targets.len())))
        } else {
            Ok(
                Self(
                    MerkleCapTarget(
                        targets
                            .chunks(4)
                            .map(|chunk| HashOutTarget::from_vec(chunk.to_vec()))
                            .collect(),
                    )
                )
            )
        }
    }
}

impl<const CAP_HEIGHT: usize> State for MerkleRootState<CAP_HEIGHT> {
    fn num_targets() -> usize {
        // A `MerkleCapTarget` is given by 2^CAP_HEIGHT hash targets,
        // and each hash target is made of 4 targets, so the overall number of targets for a
        // `MerkleCapTarget` are 4*2^CAP_HEIGHT
        4 * (1 << CAP_HEIGHT)
    }

    fn to_vec(&self) -> Vec<Target> {
        self.0
             .0
            .iter()
            .flat_map(|hash| hash.elements)
            .collect::<Vec<_>>()
    }
}

/// Type alias for the `SharedStatePublicInput` scheme employing `MerkleRootState` as state
/// representation
pub type MerkleRootPublicInput<const CAP_HEIGHT: usize> =
    SharedStatePublicInput<MerkleRootState<CAP_HEIGHT>>;

#[cfg(test)]
mod tests {
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::public_input_aggregation::check_consistency_of_dummy_public_inputs_aggregation;
    use crate::public_input_aggregation::shared_state::{
        MerkleRootPublicInput, SimpleStatePublicInput,
    };

    #[test]
    fn test_consistency() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        assert!(check_consistency_of_dummy_public_inputs_aggregation::<
            F,
            C,
            D,
            SimpleStatePublicInput,
        >()
        .unwrap());

        assert!(check_consistency_of_dummy_public_inputs_aggregation::<
            F,
            C,
            D,
            MerkleRootPublicInput<0>,
        >()
        .unwrap());
    }
}
