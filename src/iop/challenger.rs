use std::convert::TryInto;
use std::marker::PhantomData;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::{Extendable, FieldExtension};
use crate::field::field_types::RichField;
use crate::hash::hash_types::{HashOut, HashOutTarget, MerkleCapTarget};
use crate::hash::hashing::{PlonkyPermutation, SPONGE_RATE, SPONGE_WIDTH};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::Target;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
use crate::plonk::proof::{OpeningSet, OpeningSetTarget};

/// Observes prover messages, and generates challenges by hashing the transcript, a la Fiat-Shamir.
#[derive(Clone)]
pub struct Challenger<F: RichField, H: AlgebraicHasher<F>> {
    sponge_state: [F; SPONGE_WIDTH],
    input_buffer: Vec<F>,
    output_buffer: Vec<F>,
    _phantom: PhantomData<H>,
}

/// Observes prover messages, and generates verifier challenges based on the transcript.
///
/// The implementation is roughly based on a duplex sponge with a Rescue permutation. Note that in
/// each round, our sponge can absorb an arbitrary number of prover messages and generate an
/// arbitrary number of verifier challenges. This might appear to diverge from the duplex sponge
/// design, but it can be viewed as a duplex sponge whose inputs are sometimes zero (when we perform
/// multiple squeezes) and whose outputs are sometimes ignored (when we perform multiple
/// absorptions). Thus the security properties of a duplex sponge still apply to our design.
impl<F: RichField, H: AlgebraicHasher<F>> Challenger<F, H> {
    pub fn new() -> Challenger<F, H> {
        Challenger {
            sponge_state: [F::ZERO; SPONGE_WIDTH],
            input_buffer: Vec::new(),
            output_buffer: Vec::new(),
            _phantom: Default::default(),
        }
    }

    pub fn observe_element(&mut self, element: F) {
        // Any buffered outputs are now invalid, since they wouldn't reflect this input.
        self.output_buffer.clear();

        self.input_buffer.push(element);
    }

    pub fn observe_extension_element<const D: usize>(&mut self, element: &F::Extension)
    where
        F: Extendable<D>,
    {
        self.observe_elements(&element.to_basefield_array());
    }

    pub fn observe_elements(&mut self, elements: &[F]) {
        for &element in elements {
            self.observe_element(element);
        }
    }

    pub fn observe_extension_elements<const D: usize>(&mut self, elements: &[F::Extension])
    where
        F: Extendable<D>,
    {
        for element in elements {
            self.observe_extension_element(element);
        }
    }

    pub fn observe_opening_set<const D: usize>(&mut self, os: &OpeningSet<F, D>)
    where
        F: Extendable<D>,
    {
        let OpeningSet {
            constants,
            plonk_sigmas,
            wires,
            plonk_zs,
            plonk_zs_right,
            partial_products,
            quotient_polys,
        } = os;
        for v in &[
            constants,
            plonk_sigmas,
            wires,
            plonk_zs,
            plonk_zs_right,
            partial_products,
            quotient_polys,
        ] {
            self.observe_extension_elements(v);
        }
    }

    pub fn observe_hash<OH: Hasher<F>>(&mut self, hash: OH::Hash) {
        let felts: Vec<F> = hash.into();
        self.observe_elements(&felts)
    }

    pub fn observe_cap<OH: Hasher<F>>(&mut self, cap: &MerkleCap<F, OH>) {
        for &hash in &cap.0 {
            self.observe_hash::<OH>(hash);
        }
    }

    pub fn get_challenge<C: GenericConfig<D, F = F>, const D: usize>(&mut self) -> F {
        self.absorb_buffered_inputs::<C, D>();

        if self.output_buffer.is_empty() {
            // Evaluate the permutation to produce `r` new outputs.
            self.sponge_state =
                <<C as GenericConfig<D>>::InnerHasher as AlgebraicHasher<F>>::Permutation::permute(
                    self.sponge_state,
                );
            self.output_buffer = self.sponge_state[0..SPONGE_RATE].to_vec();
        }

        self.output_buffer
            .pop()
            .expect("Output buffer should be non-empty")
    }

    pub fn get_n_challenges<C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
        n: usize,
    ) -> Vec<F> {
        (0..n).map(|_| self.get_challenge::<C, D>()).collect()
    }

    pub fn get_hash<C: GenericConfig<D, F = F>, const D: usize>(&mut self) -> HashOut<F> {
        HashOut {
            elements: [
                self.get_challenge::<C, D>(),
                self.get_challenge::<C, D>(),
                self.get_challenge::<C, D>(),
                self.get_challenge::<C, D>(),
            ],
        }
    }

    pub fn get_extension_challenge<C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
    ) -> F::Extension
    where
        F: Extendable<D>,
    {
        let mut arr = [F::ZERO; D];
        arr.copy_from_slice(&self.get_n_challenges::<C, D>(D));
        F::Extension::from_basefield_array(arr)
    }

    pub fn get_n_extension_challenges<C: GenericConfig<D, F = F>, const D: usize>(
        &mut self,
        n: usize,
    ) -> Vec<F::Extension>
    where
        F: Extendable<D>,
    {
        (0..n)
            .map(|_| self.get_extension_challenge::<C, D>())
            .collect()
    }

    /// Absorb any buffered inputs. After calling this, the input buffer will be empty.
    fn absorb_buffered_inputs<C: GenericConfig<D, F = F>, const D: usize>(&mut self) {
        if self.input_buffer.is_empty() {
            return;
        }

        for input_chunk in self.input_buffer.chunks(SPONGE_RATE) {
            // Overwrite the first r elements with the inputs. This differs from a standard sponge,
            // where we would xor or add in the inputs. This is a well-known variant, though,
            // sometimes called "overwrite mode".
            for (i, &input) in input_chunk.iter().enumerate() {
                self.sponge_state[i] = input;
            }

            // Apply the permutation.
            self.sponge_state =
                <<C as GenericConfig<D>>::InnerHasher as AlgebraicHasher<F>>::Permutation::permute(
                    self.sponge_state,
                );
        }

        self.output_buffer = self.sponge_state[0..SPONGE_RATE].to_vec();

        self.input_buffer.clear();
    }
}

impl<F: RichField, H: AlgebraicHasher<F>> Default for Challenger<F, H> {
    fn default() -> Self {
        Self::new()
    }
}

/// A recursive version of `Challenger`.
pub struct RecursiveChallenger<F: Extendable<D>, H: AlgebraicHasher<F>, const D: usize> {
    sponge_state: [Target; SPONGE_WIDTH],
    input_buffer: Vec<Target>,
    output_buffer: Vec<Target>,
}

impl<F: Extendable<D>, H: AlgebraicHasher<F>, const D: usize> RecursiveChallenger<F, H, D> {
    pub(crate) fn new(builder: &mut CircuitBuilder<F, D>) -> Self {
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

    pub fn observe_opening_set(&mut self, os: &OpeningSetTarget<D>) {
        let OpeningSetTarget {
            constants,
            plonk_sigmas,
            wires,
            plonk_zs,
            plonk_zs_right,
            partial_products,
            quotient_polys,
        } = os;
        for v in &[
            constants,
            plonk_sigmas,
            wires,
            plonk_zs,
            plonk_zs_right,
            partial_products,
            quotient_polys,
        ] {
            self.observe_extension_elements(v);
        }
    }

    pub fn observe_hash(&mut self, hash: &HashOutTarget) {
        self.observe_elements(&hash.elements)
    }

    pub fn observe_cap(&mut self, cap: &MerkleCapTarget) {
        for hash in &cap.0 {
            self.observe_hash(hash)
        }
    }

    pub fn observe_extension_element(&mut self, element: ExtensionTarget<D>) {
        self.observe_elements(&element.0);
    }

    pub fn observe_extension_elements(&mut self, elements: &[ExtensionTarget<D>]) {
        for &element in elements {
            self.observe_extension_element(element);
        }
    }

    pub(crate) fn get_challenge(&mut self, builder: &mut CircuitBuilder<F, D>) -> Target {
        self.absorb_buffered_inputs(builder);

        if self.output_buffer.is_empty() {
            // Evaluate the permutation to produce `r` new outputs.
            self.sponge_state = builder.permute::<H>(self.sponge_state);
            self.output_buffer = self.sponge_state[0..SPONGE_RATE].to_vec();
        }

        self.output_buffer
            .pop()
            .expect("Output buffer should be non-empty")
    }

    pub(crate) fn get_n_challenges(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
        n: usize,
    ) -> Vec<Target> {
        (0..n).map(|_| self.get_challenge(builder)).collect()
    }

    pub fn get_hash(&mut self, builder: &mut CircuitBuilder<F, D>) -> HashOutTarget {
        HashOutTarget {
            elements: [
                self.get_challenge(builder),
                self.get_challenge(builder),
                self.get_challenge(builder),
                self.get_challenge(builder),
            ],
        }
    }

    pub fn get_extension_challenge(
        &mut self,
        builder: &mut CircuitBuilder<F, D>,
    ) -> ExtensionTarget<D> {
        self.get_n_challenges(builder, D).try_into().unwrap()
    }

    /// Absorb any buffered inputs. After calling this, the input buffer will be empty.
    fn absorb_buffered_inputs(&mut self, builder: &mut CircuitBuilder<F, D>) {
        if self.input_buffer.is_empty() {
            return;
        }

        for input_chunk in self.input_buffer.chunks(SPONGE_RATE) {
            // Overwrite the first r elements with the inputs. This differs from a standard sponge,
            // where we would xor or add in the inputs. This is a well-known variant, though,
            // sometimes called "overwrite mode".
            for (i, &input) in input_chunk.iter().enumerate() {
                self.sponge_state[i] = input;
            }

            // Apply the permutation.
            self.sponge_state = builder.permute::<H>(self.sponge_state);
        }

        self.output_buffer = self.sponge_state[0..SPONGE_RATE].to_vec();

        self.input_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::iop::challenger::{Challenger, RecursiveChallenger};
    use crate::iop::generator::generate_partial_witness;
    use crate::iop::target::Target;
    use crate::iop::witness::{PartialWitness, Witness};
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    #[test]
    fn no_duplicate_challenges() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut challenger = Challenger::<F, <C as GenericConfig<D>>::InnerHasher>::new();
        let mut challenges = Vec::new();

        for i in 1..10 {
            challenges.extend(challenger.get_n_challenges::<C, D>(i));
            challenger.observe_element(F::rand());
        }

        let dedup_challenges = {
            let mut dedup = challenges.clone();
            dedup.dedup();
            dedup
        };
        assert_eq!(dedup_challenges, challenges);
    }

    /// Tests for consistency between `Challenger` and `RecursiveChallenger`.
    #[test]
    fn test_consistency() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        // These are mostly arbitrary, but we want to test some rounds with enough inputs/outputs to
        // trigger multiple absorptions/squeezes.
        let num_inputs_per_round = vec![2, 5, 3];
        let num_outputs_per_round = vec![1, 2, 4];

        // Generate random input messages.
        let inputs_per_round: Vec<Vec<F>> = num_inputs_per_round
            .iter()
            .map(|&n| F::rand_vec(n))
            .collect();

        let mut challenger = Challenger::<F, <C as GenericConfig<D>>::InnerHasher>::new();
        let mut outputs_per_round: Vec<Vec<F>> = Vec::new();
        for (r, inputs) in inputs_per_round.iter().enumerate() {
            challenger.observe_elements(inputs);
            outputs_per_round.push(challenger.get_n_challenges::<C, D>(num_outputs_per_round[r]));
        }

        let config = CircuitConfig::large_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let mut recursive_challenger =
            RecursiveChallenger::<F, <C as GenericConfig<D>>::InnerHasher, D>::new(&mut builder);
        let mut recursive_outputs_per_round: Vec<Vec<Target>> = Vec::new();
        for (r, inputs) in inputs_per_round.iter().enumerate() {
            recursive_challenger.observe_elements(&builder.constants(inputs));
            recursive_outputs_per_round.push(
                recursive_challenger.get_n_challenges(&mut builder, num_outputs_per_round[r]),
            );
        }
        let circuit = builder.build::<C>();
        let inputs = PartialWitness::new();
        let witness = generate_partial_witness(inputs, &circuit.prover_only, &circuit.common);
        let recursive_output_values_per_round: Vec<Vec<F>> = recursive_outputs_per_round
            .iter()
            .map(|outputs| witness.get_targets(outputs))
            .collect();

        assert_eq!(outputs_per_round, recursive_output_values_per_round);
    }
}
