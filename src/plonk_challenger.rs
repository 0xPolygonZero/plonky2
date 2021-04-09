use crate::circuit_builder::CircuitBuilder;
use crate::field::field::Field;
use crate::hash::{permute, SPONGE_WIDTH, SPONGE_RATE};
use crate::target::Target;
use crate::proof::{Hash, HashTarget};

/// Observes prover messages, and generates challenges by hashing the transcript.
#[derive(Clone)]
pub struct Challenger<F: Field> {
    sponge_state: [F; SPONGE_WIDTH],
    input_buffer: Vec<F>,
    output_buffer: Vec<F>,
}

/// Observes prover messages, and generates verifier challenges based on the transcript.
///
/// The implementation is roughly based on a duplex sponge with a Rescue permutation. Note that in
/// each round, our sponge can absorb an arbitrary number of prover messages and generate an
/// arbitrary number of verifier challenges. This might appear to diverge from the duplex sponge
/// design, but it can be viewed as a duplex sponge whose inputs are sometimes zero (when we perform
/// multiple squeezes) and whose outputs are sometimes ignored (when we perform multiple
/// absorptions). Thus the security properties of a duplex sponge still apply to our design.
impl<F: Field> Challenger<F> {
    pub fn new() -> Challenger<F> {
        Challenger {
            sponge_state: [F::ZERO; SPONGE_WIDTH],
            input_buffer: Vec::new(),
            output_buffer: Vec::new(),
        }
    }

    pub fn observe_element(&mut self, element: F) {
        // Any buffered outputs are now invalid, since they wouldn't reflect this input.
        self.output_buffer.clear();

        self.input_buffer.push(element);
    }

    pub fn observe_elements(&mut self, elements: &[F]) {
        for &element in elements {
            self.observe_element(element);
        }
    }

    pub fn observe_hash(&mut self, hash: &Hash<F>) {
        self.observe_elements(&hash.elements)
    }

    pub fn get_challenge(&mut self) -> F {
        self.absorb_buffered_inputs();

        if self.output_buffer.is_empty() {
            // Evaluate the permutation to produce `r` new outputs.
            self.sponge_state = permute(self.sponge_state);
            self.output_buffer = self.sponge_state[0..SPONGE_RATE].to_vec();
        }

        self.output_buffer
            .pop()
            .expect("Output buffer should be non-empty")
    }

    pub fn get_2_challenges(&mut self) -> (F, F) {
        (self.get_challenge(), self.get_challenge())
    }

    pub fn get_3_challenges(&mut self) -> (F, F, F) {
        (
            self.get_challenge(),
            self.get_challenge(),
            self.get_challenge(),
        )
    }

    pub fn get_n_challenges(&mut self, n: usize) -> Vec<F> {
        (0..n).map(|_| self.get_challenge()).collect()
    }

    /// Absorb any buffered inputs. After calling this, the input buffer will be empty.
    fn absorb_buffered_inputs(&mut self) {
        for input_chunk in self.input_buffer.chunks(SPONGE_RATE) {
            // Add the inputs to our sponge state.
            for (i, &input) in input_chunk.iter().enumerate() {
                self.sponge_state[i] = self.sponge_state[i] + input;
            }

            // Apply the permutation.
            self.sponge_state = permute(self.sponge_state);
        }

        self.output_buffer = self.sponge_state[0..SPONGE_RATE].to_vec();

        self.input_buffer.clear();
    }
}

/// A recursive version of `Challenger`.
pub(crate) struct RecursiveChallenger {
    sponge_state: [Target; SPONGE_WIDTH],
    input_buffer: Vec<Target>,
    output_buffer: Vec<Target>,
}

impl RecursiveChallenger {
    pub(crate) fn new<F: Field>(builder: &mut CircuitBuilder<F>) -> Self {
        let zero = builder.zero();
        RecursiveChallenger {
            sponge_state: [zero; SPONGE_WIDTH],
            input_buffer: Vec::new(),
            output_buffer: Vec::new(),
        }
    }

    pub(crate) fn observe_element(&mut self, target: Target) {
        // Any buffered outputs are now invalid, since they wouldn't reflect this input.
        self.output_buffer.clear();

        self.input_buffer.push(target);
    }

    pub(crate) fn observe_elements(&mut self, targets: &[Target]) {
        for &target in targets {
            self.observe_element(target);
        }
    }

    pub fn observe_hash(&mut self, hash: &HashTarget) {
        self.observe_elements(&hash.elements)
    }

    pub(crate) fn get_challenge<F: Field>(
        &mut self,
        builder: &mut CircuitBuilder<F>,
    ) -> Target {
        self.absorb_buffered_inputs(builder);

        if self.output_buffer.is_empty() {
            // Evaluate the permutation to produce `r` new outputs.
            self.sponge_state = builder.permute(self.sponge_state);
            self.output_buffer = self.sponge_state[0..SPONGE_RATE].to_vec();
        }

        self.output_buffer
            .pop()
            .expect("Output buffer should be non-empty")
    }

    pub(crate) fn get_2_challenges<F: Field>(
        &mut self,
        builder: &mut CircuitBuilder<F>,
    ) -> (Target, Target) {
        (self.get_challenge(builder), self.get_challenge(builder))
    }

    pub(crate) fn get_3_challenges<F: Field>(
        &mut self,
        builder: &mut CircuitBuilder<F>,
    ) -> (Target, Target, Target) {
        (
            self.get_challenge(builder),
            self.get_challenge(builder),
            self.get_challenge(builder),
        )
    }

    pub(crate) fn get_n_challenges<F: Field>(
        &mut self,
        builder: &mut CircuitBuilder<F>,
        n: usize,
    ) -> Vec<Target> {
        (0..n).map(|_| self.get_challenge(builder)).collect()
    }

    /// Absorb any buffered inputs. After calling this, the input buffer will be empty.
    fn absorb_buffered_inputs<F: Field>(
        &mut self,
        builder: &mut CircuitBuilder<F>,
    ) {
        for input_chunk in self.input_buffer.chunks(SPONGE_RATE) {
            // Add the inputs to our sponge state.
            for (i, &input) in input_chunk.iter().enumerate() {
                // TODO: These adds are wasteful. Maybe GMiMCGate should have separates wires to be added in.
                self.sponge_state[i] = builder.add(self.sponge_state[i], input);
            }

            // Apply the permutation.
            self.sponge_state = builder.permute(self.sponge_state);
        }

        self.output_buffer = self.sponge_state[0..SPONGE_RATE].to_vec();

        self.input_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::circuit_data::CircuitConfig;
    use crate::field::crandall_field::CrandallField;
    use crate::generator::generate_partial_witness;
    use crate::plonk_challenger::{Challenger, RecursiveChallenger};
    use crate::target::Target;
    use crate::circuit_builder::CircuitBuilder;
    use crate::witness::PartialWitness;
    use crate::field::field::Field;

    /// Tests for consistency between `Challenger` and `RecursiveChallenger`.
    #[test]
    fn test_consistency() {
        type F = CrandallField;

        // These are mostly arbitrary, but we want to test some rounds with enough inputs/outputs to
        // trigger multiple absorptions/squeezes.
        let num_inputs_per_round = vec![2, 5, 3];
        let num_outputs_per_round = vec![1, 2, 4];

        // Generate random input messages.
        let inputs_per_round: Vec<Vec<F>> = num_inputs_per_round
            .iter()
            .map(|&n| (0..n).map(|_| F::rand()).collect::<Vec<_>>())
            .collect();

        let mut challenger = Challenger::new();
        let mut outputs_per_round: Vec<Vec<F>> = Vec::new();
        for (r, inputs) in inputs_per_round.iter().enumerate() {
            challenger.observe_elements(inputs);
            outputs_per_round.push(challenger.get_n_challenges(num_outputs_per_round[r]));
        }

        let config = CircuitConfig {
            num_wires: 114,
            num_routed_wires: 27,
            ..CircuitConfig::default()
        };
        let mut builder = CircuitBuilder::<F>::new(config);
        let mut recursive_challenger = RecursiveChallenger::new(&mut builder);
        let mut recursive_outputs_per_round: Vec<Vec<Target>> =
            Vec::new();
        for (r, inputs) in inputs_per_round.iter().enumerate() {
            recursive_challenger.observe_elements(&builder.constants(inputs));
            recursive_outputs_per_round.push(
                recursive_challenger.get_n_challenges(&mut builder, num_outputs_per_round[r]),
            );
        }
        let circuit = builder.build();
        let mut witness = PartialWitness::new();
        generate_partial_witness(&mut witness, &circuit.prover_only.generators);
        let recursive_output_values_per_round: Vec<Vec<F>> = recursive_outputs_per_round
            .iter()
            .map(|outputs| witness.get_targets(outputs))
            .collect();

        assert_eq!(outputs_per_round, recursive_output_values_per_round);
    }
}
