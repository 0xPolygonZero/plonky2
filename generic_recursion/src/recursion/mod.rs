//!
//! `recursion` module defines the interfaces to recursively aggregate an unlimited
//! number of proofs in a single aggregated proof, which can be verified with the same verifier
//! data independently from the number of proofs being aggregated.
//! The module provides also an actual implementation of such interfaces that can be used to
//! recursively aggregate proofs.
//!

use anyhow::Result;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_data::{CircuitData, VerifierCircuitData};
use plonky2::plonk::config::GenericConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use plonky2::field::extension::Extendable;

use crate::public_input_aggregation::PublicInputAggregation;
use crate::recursion::util::VerifierOnlyCircuitDataWrapper;

mod common_data_for_recursion;
mod merge_circuit;
pub mod recursive_circuit;
mod util;
mod wrap_circuit;

pub(crate) const RECURSION_THRESHOLD: usize = 12;

/// Construct an instance of `VerifierCircuitData` from a `CircuitData`: it replaces the function
/// provided by plonky2 which takes ownership of the circuit data
pub fn build_verifier_circuit_data<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    circuit_data: &CircuitData<F, C, D>,
) -> VerifierCircuitData<F, C, D> {
    let vd = VerifierOnlyCircuitDataWrapper::from(&circuit_data.verifier_only);
    VerifierCircuitData {
        verifier_only: vd.0,
        common: circuit_data.common.clone(),
    }
}

/// `BaseCircuitInfo` trait should be implemented for each base circuit whose proofs needs to be
/// aggregated with the mod circuit: it exposes to the recursion circuit the information
/// about a base circuit that are necessary for mod
pub trait BaseCircuitInfo<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
{
    /// Specify the type of public input for the circuit
    type PIScheme: PublicInputAggregation;

    fn get_verifier_circuit_data(&self) -> VerifierCircuitData<F, C, D>;
}

/// `PreparedProof` trait provides the operations available on a proof which is ready to be
/// aggregated with other prepared proofs
pub trait PreparedProof<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>:
    Clone
{
    fn get_proof(&self) -> &ProofWithPublicInputs<F, C, D>;
}

/// `RecursionCircuit` specifies the interface to recursively aggregate a set of proofs belonging
/// to a set of circuits in a single aggregated proof
pub trait RecursionCircuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    PI: PublicInputAggregation,
>: Sized
{
    /// Define the type of proofs which are ready to be aggregated
    type PreparedProof: PreparedProof<F, C, D>;

    fn build_circuit<'a>(
        circuit_set: impl Iterator<Item = Box<dyn BaseCircuitInfo<F, C, D, PIScheme = PI> + 'a>>,
    ) -> Result<Self>;

    /// variant of build_circuit function which allows to specify the numbers of
    /// proofs to be aggregated in a single recursive layer
    fn build_circuit_with_custom_aggregation_factor<'a>(
        circuit_set: impl Iterator<Item = Box<dyn BaseCircuitInfo<F, C, D, PIScheme = PI> + 'a>>,
        aggregation_factor: usize,
    ) -> Result<Self>;

    /// make a base proof ready to be aggregated, converting into a `Self::PreparedProof`
    fn prepare_proof_for_aggregation(
        &self,
        proof: ProofWithPublicInputs<F, C, D>,
        circuit_data: &VerifierCircuitData<F, C, D>,
    ) -> Result<Self::PreparedProof>;

    /// add a prepared proof to the set of proofs to be aggregated
    fn add_proofs_for_aggregation(
        self,
        prepared_proofs: impl IntoIterator<Item = Self::PreparedProof>,
    ) -> Self;

    /// compute an aggregated proof for the set of proofs to be aggregated
    fn aggregate_proofs(self) -> Result<(Self, Self::PreparedProof)> {
        self.aggregate_proofs_with([].into_iter())
    }

    /// compute an aggregated proof for the set of proofs to be aggregated and
    /// the prepared_proofs provided as input
    fn aggregate_proofs_with(
        self,
        prepared_proofs: impl IntoIterator<Item = Self::PreparedProof>,
    ) -> Result<(Self, Self::PreparedProof)>;

    /// verify an aggregated proof
    fn verify_aggregated_proof(&self, prepared_proof: Self::PreparedProof) -> Result<()>;
}


/// This method should be called on each base circuit to be included in the sets of circuits that is
/// provided as input to the `build_circuit` method of the `RecursionCircuit` trait.
/// In particular, this method allows to convert the base circuit to a data structure that fulfills
/// the trait bounds expected for circuits in the input set by the `build_circuit` method
pub fn prepare_base_circuit_for_circuit_set<'a,
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    PI: PublicInputAggregation,
>(
    circuit: impl BaseCircuitInfo<F,C,D,PIScheme=PI>+'a,
) -> Box<dyn BaseCircuitInfo<F, C, D, PIScheme = PI> + 'a> {
    Box::new(circuit)
}

#[cfg(test)]
mod test_circuits {
    use anyhow::Result;
    use plonky2::gates::arithmetic_base::ArithmeticGate;
    use plonky2::hash::hash_types::{MerkleCapTarget, RichField};
    use plonky2::hash::hashing::hash_n_to_m_no_pad;
    use plonky2::hash::merkle_proofs::MerkleProofTarget;
    use plonky2::hash::merkle_tree::MerkleTree;
    use plonky2::iop::target::{BoolTarget, Target};
    use plonky2::iop::witness::{PartialWitness, WitnessWrite};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::{CircuitConfig, CircuitData, VerifierCircuitData};
    use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
    use plonky2::plonk::proof::ProofWithPublicInputs;
    use plonky2::field::extension::Extendable;
    use plonky2_util::log2_ceil;
    use rstest::fixture;

    use crate::public_input_aggregation::shared_state::{
        MerkleRootPublicInput, SimpleStatePublicInput,
    };
    use crate::recursion::{build_verifier_circuit_data, BaseCircuitInfo};

    // check that the closure $f actually panics, printing $msg as error message if the function
    // did not panic; this macro is employed in tests in place of #[should_panic] to ensure that a
    // panic occurred in the expected function rather than in other parts of the test
    macro_rules! check_panic {
        ($f: expr, $msg: expr) => {{
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe($f));
            assert!(result.is_err(), $msg);
        }};
    }

    pub(crate) use check_panic;

    #[fixture]
    #[once]
    pub(crate) fn logger() {
        env_logger::init()
    }

    /// Data structure with all input/output targets and the `CircuitData` for a circuit proven
    /// in base proofs. The circuit is designed to be representative of a common base circuit
    /// operating on a common public state employing also some private data.
    /// The computation performed on the state was chosen to employ commonly used gates, such as
    /// arithmetic and hash ones
    pub(crate) struct MulBaseCircuit<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    > {
        private_input: Target,
        public_input: Target,
        public_output: Target,
        circuit_data: CircuitData<F, C, D>,
        num_powers: usize,
    }

    impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
        MulBaseCircuit<F, C, D>
    where
        C::Hasher: AlgebraicHasher<F>,
        [(); C::Hasher::HASH_SIZE]:,
    {
        pub(crate) fn build_base_circuit(
            config: &CircuitConfig,
            degree: usize,
            swap: bool,
        ) -> Self {
            let num_gates: usize = 1usize << degree;
            let num_ops = ArithmeticGate::new_from_config(&config).num_ops;
            // the number of gates in the circuit depending on `powers` is
            // `N = powers + 1 + ceil((2 + powers)/num_ops) + 3`.
            // Thus, to obtain a circuit with N <= num_gates gates, we set `powers` as follows
            let powers = ((num_gates - 5) * num_ops - 2) / (num_ops + 1);

            let mut builder = CircuitBuilder::<F, D>::new(config.clone());

            let init_t = builder.add_virtual_target();
            builder.register_public_input(init_t);
            let mut res_t = builder.add_virtual_target();
            builder.generate_copy(init_t, res_t);
            let zero = builder.constant(F::ZERO);
            let to_be_hashed_t = builder.add_virtual_target();
            for _ in 0..powers {
                if swap {
                    res_t = builder.hash_n_to_m_no_pad::<C::Hasher>(
                        vec![res_t, to_be_hashed_t, zero, zero],
                        1,
                    )[0];
                    res_t = builder.mul(res_t, init_t);
                } else {
                    res_t = builder.mul(res_t, init_t);
                    res_t = builder.hash_n_to_m_no_pad::<C::Hasher>(
                        vec![res_t, to_be_hashed_t, zero, zero],
                        1,
                    )[0];
                }
            }

            let pow_t = builder.add_virtual_target();
            builder.register_public_input(pow_t);
            builder.is_equal(res_t, pow_t);

            let data = builder.build::<C>();

            Self {
                private_input: to_be_hashed_t,
                public_input: init_t,
                public_output: pow_t,
                num_powers: powers,
                circuit_data: data,
            }
        }

        pub(crate) fn generate_base_proof(
            &self,
            init: F,
        ) -> Result<ProofWithPublicInputs<F, C, D>> {
            let mut pw = PartialWitness::<F>::new();

            pw.set_target(self.public_input, init);
            let to_be_hashed = F::rand();
            pw.set_target(self.private_input, to_be_hashed);
            let res = (0..self.num_powers).fold(init,
                |acc, _| {
                    let int = acc.mul(init);
                    hash_n_to_m_no_pad::<_, <C::Hasher as Hasher<F>>::Permutation>(
                        &[int, to_be_hashed, F::ZERO, F::ZERO],
                        1,
                    )[0]
                }
            );

            pw.set_target(self.public_output, res);

            let proof = self.circuit_data.prove(pw)?;

            self.circuit_data.verify(proof.clone())?;

            assert_eq!(proof.public_inputs[1], res);

            Ok(proof)
        }

        pub(crate) fn get_circuit_data(&self) -> &CircuitData<F, C, D> {
            &self.circuit_data
        }
    }

    /// Data structure with all input/output targets and the `CircuitData` for another base
    /// circuit proven in base proofs. The set of public input/output of the circuit is the same of
    /// `MulBaseCircuit`, as the base proofs of such circuits will be merged together, but the
    /// circuit employs a different set of gates to operate on the public state
    pub(crate) struct ExpBaseCircuit<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    > {
        private_input: Target,
        public_input: Target,
        public_output: Target,
        circuit_data: CircuitData<F, C, D>,
        num_powers: usize,
    }

    impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
        ExpBaseCircuit<F, C, D>
    where
        C::Hasher: AlgebraicHasher<F>,
        [(); C::Hasher::HASH_SIZE]:,
    {
        pub(crate) fn build_base_circuit(config: &CircuitConfig, degree: usize) -> Self {
            // Estimation of the number of exponentiations to be performed in order to ensure that
            // the number of gates of the circuit will be at most 2^degree, also taking into account
            // that the `build` method of `CircuitBuilder` adds further gates besides the ones
            // instantiated in this function
            let num_powers = (1 << (degree - 1)) - 4;
            let mut builder = CircuitBuilder::<F, D>::new(config.clone());
            let mut res_t = builder.add_virtual_target();
            builder.register_public_input(res_t);
            let init_t = res_t.clone();
            let exp = builder.add_virtual_target();
            for _ in 0..num_powers {
                res_t = builder.exp(res_t, exp, F::BITS);
            }

            let pow_t = builder.add_virtual_target();
            builder.register_public_input(pow_t);

            builder.is_equal(res_t, pow_t);

            let data = builder.build::<C>();

            Self {
                private_input: exp,
                public_input: init_t,
                public_output: pow_t,
                circuit_data: data,
                num_powers,
            }
        }

        pub(crate) fn generate_base_proof(
            &self,
            init: F,
        ) -> Result<ProofWithPublicInputs<F, C, D>> {
            let mut pw = PartialWitness::<F>::new();

            pw.set_target(self.public_input, init);
            let exp = F::rand();
            pw.set_target(self.private_input, exp);
            let mut res = init;
            for _ in 0..self.num_powers {
                res = res.exp_u64(exp.to_canonical_u64());
            }
            pw.set_target(self.public_output, res);

            let proof = self.circuit_data.prove(pw)?;

            self.circuit_data.verify(proof.clone())?;

            assert_eq!(proof.public_inputs[1], res);

            Ok(proof)
        }

        pub(crate) fn get_circuit_data(&self) -> &CircuitData<F, C, D> {
            &self.circuit_data
        }
    }

    /// Data structure with all input/output targets and the `CircuitData` for a base circuit
    /// where the input/output state is represented by a Merkle-root. This circuit is employed to
    /// test the `MerkleRootPublicInput` aggregation scheme. The circuit takes as input a state
    /// representing a Merkle-tree, a leaf of such Merkle-tree and a value `op` in the range [0,3].
    /// The circuit updates the provided leaf of the Merkle-tree according to the value of `op` as
    /// follows:
    /// - If `op==0` -> `new_leaf = 2*leaf`
    /// - If `op==1` -> `new_leaf = leaf^2`
    /// - If `op==2` -> `new_leaf = leaf^leaf`
    /// - If `op==3` -> `new_leaf = leaf`
    /// The circuit computes as an output the new state given by the root of the updated Merkle-tree
    pub(crate) struct MerkleRootStateBaseCircuit<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
        const CAP_HEIGHT: usize,
    > {
        initial_root: MerkleCapTarget,
        initial_mpt: MerkleProofTarget,
        leaf_index_bits_initial_mpt: Vec<BoolTarget>,
        leaf_target: Target,
        op_target: Target,
        final_root: MerkleCapTarget,
        final_mpt: MerkleProofTarget,
        circuit_data: CircuitData<F, C, D>,
    }

    impl<
            F: RichField + Extendable<D>,
            C: GenericConfig<D, F = F>,
            const D: usize,
            const CAP_HEIGHT: usize,
        > MerkleRootStateBaseCircuit<F, C, D, CAP_HEIGHT>
    where
        C::Hasher: AlgebraicHasher<F>,
        [(); C::Hasher::HASH_SIZE]:,
    {
        pub(crate) fn build_circuit(config: &CircuitConfig, num_leaves: usize) -> Self {
            let mut builder = CircuitBuilder::<F, D>::new(config.clone());
            let leaf_target = builder.add_virtual_target();
            let op_target = builder.add_virtual_target();
            let initial_root_target = builder.add_virtual_cap(CAP_HEIGHT);
            let full_tree_height = log2_ceil(num_leaves);
            assert!(full_tree_height >= CAP_HEIGHT, "CAP_HEIGHT={} for MerkleRootStateBaseCircuit is \
            too high: it should be no greater than ceil(log2(num_leaves)) = {}", CAP_HEIGHT, full_tree_height);
            let height = full_tree_height - CAP_HEIGHT;
            let mpt = MerkleProofTarget {
                siblings: builder.add_virtual_hashes(height),
            };
            let leaf_index_bits = (0..height)
                .map(|_| builder.add_virtual_bool_target_safe())
                .collect::<Vec<_>>();

            let hash_leaf = builder.hash_or_noop::<C::Hasher>(vec![leaf_target]);

            builder.verify_merkle_proof_to_cap::<C::Hasher>(
                hash_leaf.elements.to_vec(),
                leaf_index_bits.as_slice(),
                &initial_root_target,
                &mpt,
            );

            let new_leaf_target_doubled = builder.add(leaf_target, leaf_target);
            let new_leaf_target_squared = builder.mul(leaf_target, leaf_target);
            let new_leaf_target_powered = builder.exp(leaf_target, leaf_target, F::BITS);
            let new_leaf_target = builder.random_access(
                op_target,
                vec![
                    new_leaf_target_doubled,
                    new_leaf_target_squared,
                    new_leaf_target_powered,
                    leaf_target,
                ],
            );

            builder.range_check(op_target, 2);

            let final_mpt = MerkleProofTarget {
                siblings: builder.add_virtual_hashes(height),
            };

            let new_hash_leaf = builder.hash_or_noop::<C::Hasher>(vec![new_leaf_target]);

            let final_root_target = builder.add_virtual_cap(CAP_HEIGHT);

            builder.verify_merkle_proof_to_cap::<C::Hasher>(
                new_hash_leaf.elements.to_vec(),
                leaf_index_bits.as_slice(),
                &final_root_target,
                &final_mpt,
            );

            let merkle_cap_to_targets = |target: MerkleCapTarget| {
                target
                    .0
                    .iter()
                    .flat_map(|hash| hash.elements.to_vec())
                    .collect::<Vec<_>>()
            };

            builder.register_public_inputs(
                merkle_cap_to_targets(initial_root_target.clone()).as_slice(),
            );
            builder.register_public_inputs(
                merkle_cap_to_targets(final_root_target.clone()).as_slice(),
            );

            let data = builder.build::<C>();

            Self {
                initial_root: initial_root_target,
                initial_mpt: mpt,
                leaf_index_bits_initial_mpt: leaf_index_bits,
                leaf_target,
                op_target,
                final_root: final_root_target,
                final_mpt,
                circuit_data: data,
            }
        }

        pub(crate) fn generate_base_proof(
            &self,
            mt: &mut MerkleTree<F, C::Hasher>,
            leaf_index: usize,
            op: u8,
        ) -> Result<ProofWithPublicInputs<F, C, D>> {
            let mut pw = PartialWitness::new();

            pw.set_cap_target(&self.initial_root, &mt.cap);

            let leaf = *mt.leaves[leaf_index].first().unwrap();
            pw.set_target(self.leaf_target, leaf);

            pw.set_target(self.op_target, F::from_canonical_u64(op as u64));

            let merkle_proof = mt.prove(leaf_index);

            for (i, bool_target) in self.leaf_index_bits_initial_mpt.iter().enumerate() {
                let mask = (1 << i) as usize;
                pw.set_bool_target(*bool_target, (leaf_index & mask) != 0);
            }
            // set merkle proof target
            assert_eq!(merkle_proof.len(), self.initial_mpt.siblings.len());
            for (&mp, &mpt) in merkle_proof
                .siblings
                .iter()
                .zip(self.initial_mpt.siblings.iter())
            {
                pw.set_hash_target(mpt, mp);
            }

            let new_leaf = match op {
                0 => leaf.add(leaf),
                1 => leaf.mul(leaf),
                2 => leaf.exp_u64(leaf.to_canonical_u64()),
                3 => leaf,
                _ => unreachable!(),
            };

            mt.leaves[leaf_index] = vec![new_leaf];

            *mt = MerkleTree::new(mt.leaves.clone(), CAP_HEIGHT);

            let merkle_proof = mt.prove(leaf_index);
            // set merkle proof target
            assert_eq!(merkle_proof.len(), self.final_mpt.siblings.len());
            for (&mp, &mpt) in merkle_proof
                .siblings
                .iter()
                .zip(self.final_mpt.siblings.iter())
            {
                pw.set_hash_target(mpt, mp);
            }

            pw.set_cap_target(&self.final_root, &mt.cap);

            self.circuit_data.prove(pw)
        }
    }

    impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
        BaseCircuitInfo<F, C, D> for &MulBaseCircuit<F, C, D>
    {
        type PIScheme = SimpleStatePublicInput;

        fn get_verifier_circuit_data(&self) -> VerifierCircuitData<F, C, D> {
            build_verifier_circuit_data(&self.circuit_data)
        }
    }

    impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
        BaseCircuitInfo<F, C, D> for &ExpBaseCircuit<F, C, D>
    {
        type PIScheme = SimpleStatePublicInput;

        fn get_verifier_circuit_data(&self) -> VerifierCircuitData<F, C, D> {
            build_verifier_circuit_data(&self.circuit_data)
        }
    }

    impl<
            F: RichField + Extendable<D>,
            C: GenericConfig<D, F = F>,
            const D: usize,
            const CAP_HEIGHT: usize,
        > BaseCircuitInfo<F, C, D> for &MerkleRootStateBaseCircuit<F, C, D, CAP_HEIGHT>
    {
        type PIScheme = MerkleRootPublicInput<CAP_HEIGHT>;

        fn get_verifier_circuit_data(&self) -> VerifierCircuitData<F, C, D> {
            build_verifier_circuit_data(&self.circuit_data)
        }
    }
}
