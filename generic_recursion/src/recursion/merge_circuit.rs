use std::collections::HashMap;

use anyhow::{Error, Result};
use plonky2::gates::noop::NoopGate;
use plonky2::hash::hash_types::{HashOutTarget, MerkleCapTarget, RichField};
use plonky2::hash::merkle_proofs::MerkleProofTarget;
use plonky2::hash::merkle_tree::{MerkleCap, MerkleTree};
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{
    CircuitConfig, CircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, GenericHashOut, Hasher};
use plonky2::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use plonky2::field::extension::Extendable;
use plonky2_util::log2_ceil;

use crate::public_input_aggregation::{
    conditionally_aggregate_public_input, PublicInputAggregation,
};
use crate::recursion::common_data_for_recursion::build_data_for_recursive_aggregation;
use crate::recursion::util::{
    check_circuit_digest_target, merkle_cap_to_targets, num_targets_for_circuit_set,
};
use crate::recursion::wrap_circuit::WrapCircuit;
use crate::recursion::RECURSION_THRESHOLD;

// cap height for the Merkle-tree employed to represent the set of circuits that can be aggregated with
// `MergeCircuit`; it is now set to 0 for simplicity, which is equivalent to a traditional
// Merkle-tree with a single root.
//ToDo: evaluate if changing the value depending on the number of circuits in the set
const CIRCUIT_SET_CAP_HEIGHT: usize = 0;

// Set of targets employed to prove that the circuit employed to generate a proof being aggregated
// by `MergeCircuit` belongs to the set of circuits that can be aggregated by the `MergeCircuit`
// itself
struct CircuitSetMembershipTargets {
    merkle_proof_target: MerkleProofTarget,
    leaf_index_bits: Vec<BoolTarget>,
}

// The target employed to represent the set of circuits that can be aggregated by the `MergeCircuit`
pub(crate) struct CircuitSetTarget(MerkleCapTarget);

impl CircuitSetTarget {
    pub(crate) fn build_target<F: RichField + Extendable<D>, const D: usize>(
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self {
        Self(builder.add_virtual_cap(CIRCUIT_SET_CAP_HEIGHT))
    }

    pub(crate) fn to_targets(&self) -> Vec<Target> {
        merkle_cap_to_targets(&self.0)
    }

    // Enforce that `circuit_digest_target` is a leaf in the merkle-tree
    // with root `circuit_set_target`
    fn check_circuit_digest_membership<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        builder: &mut CircuitBuilder<F, D>,
        circuit_set_target: &Self,
        circuit_digest_target: &HashOutTarget,
        num_circuit_digests: usize,
    ) -> CircuitSetMembershipTargets
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let full_tree_height = log2_ceil(num_circuit_digests);
        assert!(full_tree_height >= CIRCUIT_SET_CAP_HEIGHT, "CIRCUIT_SET_CAP_HEIGHT={} is too high: it should be no greater than ceil(log2(num_leaves)) = {}", CIRCUIT_SET_CAP_HEIGHT, full_tree_height);
        let height = full_tree_height - CIRCUIT_SET_CAP_HEIGHT;
        let mpt = MerkleProofTarget {
            siblings: builder.add_virtual_hashes(height),
        };
        let leaf_index_bits = (0..height)
            .map(|_| builder.add_virtual_bool_target_safe())
            .collect::<Vec<_>>();

        builder.verify_merkle_proof_to_cap::<C::Hasher>(
            circuit_digest_target.elements.to_vec(),
            leaf_index_bits.as_slice(),
            &circuit_set_target.0,
            &mpt,
        );

        CircuitSetMembershipTargets {
            merkle_proof_target: mpt,
            leaf_index_bits,
        }
    }
}

// Data structure employed by the `MergeCircuit` to store and manage the set of circuits that can
// be aggregated by the `MergeCircuit`
struct CircuitSet<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    circuit_digests_to_leaf_indexes: HashMap<Vec<F>, usize>,
    mt: MerkleTree<F, C::Hasher>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> CircuitSet<F, C, D>
where
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    fn build_circuit_set(circuit_digests: Vec<<C::Hasher as Hasher<F>>::Hash>) -> Self {
        let (circuit_digests_to_leaf_indexes, mut leaves) : (HashMap<Vec<F>, usize>, Vec<_>) = circuit_digests
            .iter()
            .enumerate()
            .map(|(index, hash)| {
                let hash_to_fes = hash.to_vec();
                ((hash_to_fes, index), hash.to_vec())
            })
            .unzip();

        let num_leaves_padded: usize = 1 << log2_ceil(leaves.len());
        leaves.resize_with(num_leaves_padded, || vec![F::ZERO]);

        Self {
            circuit_digests_to_leaf_indexes,
            mt: MerkleTree::<F, C::Hasher>::new(leaves, CIRCUIT_SET_CAP_HEIGHT),
        }
    }

    fn leaf_index(&self, digest: &[F]) -> Option<usize> {
        self.circuit_digests_to_leaf_indexes.get(digest).cloned()
    }

    // set a `CircuitSetMembershipTargets` to prove membership of `circuit_digest` in the set of
    // circuits that can be aggregated by the `MergeCircuit`
    fn set_circuit_membership_target(
        &self,
        pw: &mut PartialWitness<F>,
        membership_target: &CircuitSetMembershipTargets,
        circuit_digest: <C::Hasher as Hasher<F>>::Hash,
    ) -> Result<()> {
        // compute merkle proof for `circuit_digest`
        let leaf_index = self
            .leaf_index(&circuit_digest.to_vec().as_slice())
            .ok_or(Error::msg("circuit digest not found"))?;

        let merkle_proof = self.mt.prove(leaf_index);

        // set leaf index bits targets with the little-endian bit decomposition of leaf_index
        for (i, bool_target) in membership_target.leaf_index_bits.iter().enumerate() {
            let mask = (1 << i) as usize;
            pw.set_bool_target(*bool_target, (leaf_index & mask) != 0);
        }
        // set merkle proof target
        assert_eq!(
            merkle_proof.len(),
            membership_target.merkle_proof_target.siblings.len()
        );
        for (&mp, &mpt) in merkle_proof
            .siblings
            .iter()
            .zip(membership_target.merkle_proof_target.siblings.iter())
        {
            pw.set_hash_target(mpt, mp);
        }

        Ok(())
    }
}

// A short representation (e.g., a digest) of the set of circuits that can be aggregated by
// `MergeCircuit`; this should represent values assignable to a `CircuitSetTarget`
#[derive(Debug, Clone)]
pub(crate) struct CircuitSetDigest<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(MerkleCap<F, C::Hasher>);

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    CircuitSetDigest<F, C, D>
where
    C::Hasher: AlgebraicHasher<F>,
{
    pub(crate) fn set_circuit_set_target(
        &self,
        pw: &mut PartialWitness<F>,
        target: &CircuitSetTarget,
    ) {
        pw.set_cap_target(&target.0, &self.0);
    }

    pub(crate) fn flatten(&self) -> Vec<F> {
        self.0.flatten()
    }
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> Default
    for CircuitSetDigest<F, C, D>
where
    [(); C::Hasher::HASH_SIZE]:,
{
    fn default() -> Self {
        Self(MerkleCap(
            (0..(1 << CIRCUIT_SET_CAP_HEIGHT))
                .map(|_| {
                    <<C as GenericConfig<D>>::Hasher as Hasher<F>>::Hash::from_bytes(
                        &[0u8; <<C as GenericConfig<D>>::Hasher as Hasher<F>>::HASH_SIZE],
                    )
                })
                .collect::<Vec<_>>(),
        ))
    }
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    From<&CircuitSet<F, C, D>> for CircuitSetDigest<F, C, D>
{
    fn from(circuit_set: &CircuitSet<F, C, D>) -> Self {
        Self(circuit_set.mt.cap.clone())
    }
}

fn are_digest_equal<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    first_digest: &HashOutTarget,
    second_digest: &HashOutTarget,
) -> BoolTarget {
    let result = builder._true();
    first_digest
        .elements
        .iter()
        .zip(second_digest.elements.iter())
        .fold(result, |result, (first_el, second_el)| {
            let is_eq = builder.is_equal(*first_el, *second_el);
            builder.and(is_eq, result)
        })
}

struct MergeCircuitInputTargets<const D: usize> {
    proof_targets: Vec<ProofWithPublicInputsTarget<D>>,
    inner_vk_targets: Vec<VerifierCircuitTarget>,
    circuit_set_target: CircuitSetTarget,
    circuit_set_membership_targets: Vec<CircuitSetMembershipTargets>,
}
// `DummyCircuit` is the circuit employed to generate dummy proofs, that are useless proofs which are
// aggregated with real proofs by the `MergeCircuit` if there are less than `N` real proofs to be
// aggregated, where `N` is the number of proofs to be aggregated expected by the `MergeCircuit`.
struct DummyCircuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    PI: PublicInputAggregation,
> {
    circuit_data: CircuitData<F, C, D>,
    public_input_targets: PI,
    circuit_set_target: CircuitSetTarget,
    fake_proof_target: ProofWithPublicInputsTarget<D>,
    fake_vk_target: VerifierCircuitTarget,
    fake_proof: ProofWithPublicInputs<F, C, D>,
    fake_circuit: VerifierOnlyCircuitData<C, D>,
}

impl<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
        PI: PublicInputAggregation,
    > DummyCircuit<F, C, D, PI>
where
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    fn build_circuit(config: CircuitConfig) -> Self {
        let mut dummy_circuit_builder = CircuitBuilder::<F,D>::new(config.clone());
        let dummy_circuit_input_targets =
            PI::dummy_circuit_inputs_logic(&mut dummy_circuit_builder);

        // add public input for the circuit set digest to the dummy circuit, to make it compatible
        // with the public input format expected by the MergeCircuit
        let pi_target = CircuitSetTarget::build_target(&mut dummy_circuit_builder);
        dummy_circuit_builder.register_public_inputs(pi_target.to_targets().as_slice());

        // To ensure that the dummy circuit has the same set of gates expected by the `MergeCircuit`,
        // we add a recursive verifier for a useless circuit identified as the `fake_circuit`.
        // The `build_fake_proof` function instantiates the `fake_circuit` and generates the proof
        // for such circuit which is recursively verified by the dummy circuit
        let build_fake_proof = || {
            let mut builder = CircuitBuilder::new(config.clone());
            let mut pw = PartialWitness::new();
            let target = builder.add_virtual_target();
            builder.register_public_input(target);
            pw.set_target(target, F::rand());
            // in order for the recursive verifier wrapping the proof generated with this circuit to have
            // the same set of gates of circuits that can be aggregated by the `MergeCircuit`, we need
            // to have at least 2^6 gates
            for _ in 0..32 {
                builder.add_gate(NoopGate, vec![]);
            }

            let data = builder.build::<C>();
            let proof = data.prove(pw).unwrap();
            (data, proof)
        };
        let (fake_circuit, fake_proof) = build_fake_proof();

        let (pt, vt) = (
            dummy_circuit_builder.add_virtual_proof_with_pis::<C>(&fake_circuit.common),
            VerifierCircuitTarget {
                constants_sigmas_cap: dummy_circuit_builder
                    .add_virtual_cap(fake_circuit.common.config.fri_config.cap_height),
                circuit_digest: dummy_circuit_builder.add_virtual_hash(),
            },
        );

        dummy_circuit_builder.verify_proof::<C>(&pt, &vt, &fake_circuit.common);


        // pad dummy circuit with Noop gates to reach the RECURSION_THRESHOLD size
        while dummy_circuit_builder.num_gates() < (1 << (RECURSION_THRESHOLD - 1)) {
            dummy_circuit_builder.add_gate(NoopGate, vec![]);
        }

        let dummy_circuit_data = dummy_circuit_builder.build::<C>();

        DummyCircuit {
            circuit_data: dummy_circuit_data,
            public_input_targets: dummy_circuit_input_targets,
            circuit_set_target: pi_target,
            fake_proof_target: pt,
            fake_vk_target: vt,
            fake_proof,
            fake_circuit: fake_circuit.verifier_only,
        }
    }

    fn generate_dummy_proof(
        &self,
        input_values: &[F],
        circuit_set: &CircuitSet<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>> {
        let mut pw = PartialWitness::new();
        PI::set_dummy_circuit_inputs(input_values, &self.public_input_targets, &mut pw);
        CircuitSetDigest::from(circuit_set)
            .set_circuit_set_target(&mut pw, &self.circuit_set_target);
        pw.set_proof_with_pis_target(&self.fake_proof_target, &self.fake_proof);
        pw.set_cap_target(
            &self.fake_vk_target.constants_sigmas_cap,
            &self.fake_circuit.constants_sigmas_cap,
        );
        pw.set_hash_target(
            self.fake_vk_target.circuit_digest,
            self.fake_circuit.circuit_digest,
        );

        self.circuit_data.prove(pw)
    }
}

pub(crate) struct MergeCircuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    PI: PublicInputAggregation,
> {
    input_targets: MergeCircuitInputTargets<D>,
    circuit_set: CircuitSet<F, C, D>,
    circuit_data: CircuitData<F, C, D>,
    wrap_circuit: WrapCircuit<F, C, D>,
    dummy_circuit: DummyCircuit<F, C, D, PI>,
}

impl<F, C, const D: usize, PI: PublicInputAggregation> MergeCircuit<F, C, D, PI>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F> + 'static,
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    pub(crate) fn build_merge_circuit(
        config: CircuitConfig,
        num_proofs_to_merge: usize,
        mut circuit_digests: Vec<<C::Hasher as Hasher<F>>::Hash>,
    ) -> Result<Self> {
        if num_proofs_to_merge < 2 {
            return Err(Error::msg("meaningless to merge less than 2 proofs, provide a higher number of proofs to merge"))
        }
        let num_public_inputs = PI::num_public_inputs();

        let rec_data =
            build_data_for_recursive_aggregation::<F, C, D>(config.clone(), num_public_inputs);

        // build the dummy circuit for the PI scheme
        let dummy_circuit = DummyCircuit::<F,C,D,PI>::build_circuit(config.clone());
        assert_eq!(&rec_data, &dummy_circuit.circuit_data.common);

        circuit_digests.push(dummy_circuit.circuit_data.verifier_only.circuit_digest);

        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        let dummy_circuit_digest =
            builder.constant_hash(dummy_circuit.circuit_data.verifier_only.circuit_digest);
        let (proof_targets, inner_data): (Vec<_>, Vec<_>) = (0..num_proofs_to_merge)
            .map(|_| {
                (
                    builder.add_virtual_proof_with_pis::<C>(&rec_data),
                    VerifierCircuitTarget {
                        constants_sigmas_cap: builder
                            .add_virtual_cap(rec_data.config.fri_config.cap_height),
                        circuit_digest: builder.add_virtual_hash(),
                    },
                )
            })
            .unzip();

        // verify proofs and check circuit digests
        let circuit_set_target = CircuitSetTarget::build_target(&mut builder);
        let proof_membership_targets = proof_targets
            .iter()
            .zip(inner_data.iter())
            .map(|(pt, inner_vd)| {
                builder.verify_proof::<C>(pt, inner_vd, &rec_data);
                check_circuit_digest_target::<_, C, D>(&mut builder, inner_vd, RECURSION_THRESHOLD);
                CircuitSetTarget::check_circuit_digest_membership::<F, C, D>(
                    &mut builder,
                    &circuit_set_target,
                    &inner_vd.circuit_digest,
                    circuit_digests.len() + 1, // later on we will add the merge circuit digest to the set,
                                               // so the merkle-tree that will be built will have one additional leaf
                )
            })
            .collect::<Vec<_>>();

        // public input aggregation
        let circuit_set_targets = circuit_set_target.to_targets();
        debug_assert_eq!(
            circuit_set_targets.len(),
            num_targets_for_circuit_set::<F, D>(config.clone())
        );

        let public_inputs = proof_targets
            .iter()
            .map(|pt| {
                // check that the circuit set public input targets of each proof are equal to
                // `circuit_set_targets`
                for (cs_t, pi_t) in circuit_set_targets
                    .iter()
                    .zip(pt.public_inputs.iter().skip(num_public_inputs))
                {
                    builder.connect(*cs_t, *pi_t);
                }
                PI::try_from_public_input_targets(&pt.public_inputs[..num_public_inputs])
            })
            .collect::<Result<Vec<_>>>()?;
        if PI::can_aggregate_public_inputs_of_dummy_proofs() {
            PI::aggregate_public_inputs(&mut builder, public_inputs.into_iter());
        } else {
            // we always aggregate at least 2 real proofs
            let mut aggregation_input =
                public_inputs[0].aggregate_public_input(&mut builder, &public_inputs[1]);
            // Since we don't conditionally aggregate the public inputs of the first 2 proofs,
            // we need to enforce that the first two proofs aren't dummy ones: this is necessary to
            // avoid that a malicious prover may arbitrarily update the aggregated public input by
            // providing dummy proofs
            let is_first_circuit_dummy = are_digest_equal(
                &mut builder,
                &inner_data[0].circuit_digest,
                &dummy_circuit_digest,
            );
            builder.assert_zero(is_first_circuit_dummy.target);
            let is_second_circuit_dummy = are_digest_equal(
                &mut builder,
                &inner_data[1].circuit_digest,
                &dummy_circuit_digest,
            );
            builder.assert_zero(is_second_circuit_dummy.target);

            // conditionally aggregate all the remaining public inputs: `aggregation_input` is updated
            // only if the proof is not a dummy one, which is determined by comparing
            // `inner_vd.circuit_digest` with the circuit digest of the dummy circuit
            for (input, inner_vd) in public_inputs[2..].iter().zip(inner_data[2..].iter()) {
                let is_circuit_dummy = are_digest_equal(
                    &mut builder,
                    &inner_vd.circuit_digest,
                    &dummy_circuit_digest,
                );
                let selector = builder.not(is_circuit_dummy);
                aggregation_input = conditionally_aggregate_public_input::<F, C, D, PI>(
                    &mut builder,
                    &selector,
                    &aggregation_input,
                    input,
                )?;
            }
            aggregation_input.register_public_inputs(&mut builder);
        }
        builder.register_public_inputs(circuit_set_targets.as_slice());

        let data = builder.build::<C>();

        let wrap_circuit =
            WrapCircuit::build_wrap_circuit(&data.verifier_only, &data.common, &config);

        // add the circuit digest of the wrap circuit for the aggregated proof to the set of circuits
        // that can be merged with the `MergeCircuit`
        circuit_digests.push(
            wrap_circuit
                .final_proof_circuit_data()
                .verifier_only
                .circuit_digest,
        );

        Ok(
            Self {
                input_targets: MergeCircuitInputTargets {
                    proof_targets,
                    inner_vk_targets: inner_data,
                    circuit_set_target,
                    circuit_set_membership_targets: proof_membership_targets,
                },
                circuit_set: CircuitSet::build_circuit_set(circuit_digests),
                circuit_data: data,
                wrap_circuit,
                dummy_circuit,
            }
        )
    }
    /// Merge `input_proofs`, generated for circuits with verifier data found in `input_vds`, into a
    /// single proof, employing `self` circuit. `self` always merges N proofs, where N is the aggregation
    /// factor employed to build the circuit; if less than N proofs are provided as input,
    /// a dummy proof is generated to have N proofs to be merged
    pub(crate) fn merge_proofs<'a>(
        &'a self,
        input_proofs: &[ProofWithPublicInputs<F, C, D>],
        input_vds: impl Iterator<Item = &'a VerifierOnlyCircuitData<C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>> {
        let mut pw = PartialWitness::new();
        let mut input_proofs_iterator = self.input_targets.proof_targets.iter();
        for (proof, pt) in input_proofs.iter().zip(&mut input_proofs_iterator) {
            pw.set_proof_with_pis_target(pt, proof);
        }
        if input_proofs.len() < 2 {
            return Err(Error::msg("provide at least 2 proofs to be merged"))
        }
        let previous_proof = input_proofs.last().unwrap();
        let tmp_dummy_proof;
        let mut dummy_vds = vec![];
        let dummy_proof = if let Some(pt) = input_proofs_iterator.next() {
            tmp_dummy_proof = self
                .dummy_circuit
                .generate_dummy_proof(&previous_proof.public_inputs[..PI::num_public_inputs()], &self.circuit_set)?;
            pw.set_proof_with_pis_target(pt, &tmp_dummy_proof);
            dummy_vds.push(&self.dummy_circuit.circuit_data.verifier_only);
            &tmp_dummy_proof
        } else {
            previous_proof
        };

        for pt in input_proofs_iterator {
            pw.set_proof_with_pis_target(pt, dummy_proof);
            dummy_vds.push(&self.dummy_circuit.circuit_data.verifier_only);
        }

        for ((verifier_data, vt), membership_proof_target) in input_vds
            .chain(dummy_vds.into_iter())
            .zip(self.input_targets.inner_vk_targets.iter())
            .zip(self.input_targets.circuit_set_membership_targets.iter())
        {
            pw.set_cap_target(
                &vt.constants_sigmas_cap,
                &verifier_data.constants_sigmas_cap,
            );
            pw.set_hash_target(vt.circuit_digest, verifier_data.circuit_digest);

            self.circuit_set.set_circuit_membership_target(
                &mut pw,
                membership_proof_target,
                verifier_data.circuit_digest,
            )?;
        }

        CircuitSetDigest::from(&self.circuit_set)
            .set_circuit_set_target(&mut pw, &self.input_targets.circuit_set_target);

        let aggregated_proof = self.circuit_data.prove(pw)?;
        // wrap the aggregated proof to reduce its size to RECURSION_THRESHOLD
        self.wrap_circuit.wrap_proof(aggregated_proof)
    }

    pub(crate) fn get_circuit_set_digest(&self) -> CircuitSetDigest<F, C, D> {
        CircuitSetDigest::from(&self.circuit_set)
    }

    pub(crate) fn aggregated_proof_circuit_data(&self) -> &CircuitData<F, C, D> {
        self.wrap_circuit.final_proof_circuit_data()
    }
}

#[cfg(test)]
mod test {
    use std::cmp::min;

    use plonky2::plonk::config::PoseidonGoldilocksConfig;
    use plonky2::field::types::Sample;
    use rstest::{fixture, rstest};
    use serial_test::serial;
    use plonky2::hash::hashing::hash_n_to_hash_no_pad;

    use super::*;
    use crate::public_input_aggregation::shared_state::{
        MerkleRootPublicInput, SimpleStatePublicInput,
    };
    use crate::public_input_aggregation::tests::PublicInputAccumulator;
    use crate::recursion::test_circuits::{check_panic, logger, ExpBaseCircuit, MulBaseCircuit};
    use crate::recursion::wrap_circuit::test::mutable_final_proof_circuit_data;
    use crate::recursion::wrap_circuit::WrapCircuitForBaseProofs;



    impl<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + 'static,
        const D: usize,
        PI: PublicInputAggregation,
    > MergeCircuit<F,C,D,PI>
        where
            C::Hasher: AlgebraicHasher<F>,
            [(); C::Hasher::HASH_SIZE]:,
    {
        fn aggregate_proofs(
            &self,
            proofs: &[ProofWithPublicInputs<F, C, D>],
            verifier_data: &[&VerifierOnlyCircuitData<C, D>],
            aggregation_factor: usize,
        ) -> Result<ProofWithPublicInputs<F, C, D>>
            where
                C::Hasher: AlgebraicHasher<F>,
                [(); C::Hasher::HASH_SIZE]:,
        {
            let num_proofs = proofs.len();
            assert_eq!(verifier_data.len(), num_proofs);
            let num_proofs_to_be_aggregated = min(num_proofs, aggregation_factor);
            let mut aggregated_proof = self.merge_proofs(
                &proofs[..num_proofs_to_be_aggregated],
                verifier_data[..num_proofs_to_be_aggregated]
                    .into_iter()
                    .cloned(),
            )?;
            println!("first {} proofs aggregated", num_proofs_to_be_aggregated);

            let merge_circuit_data = self.aggregated_proof_circuit_data();

            let merge_next_chunk = |chunk_index, accum, proof_chunk, vd_chunk| {
                let mut proofs = vec![accum];
                proofs.extend_from_slice(proof_chunk);
                let mut merge_vd = vec![&merge_circuit_data.verifier_only];
                merge_vd.extend_from_slice(vd_chunk);
                assert!(proofs.len() <= aggregation_factor);
                assert!(merge_vd.len() <= aggregation_factor);
                let proof = self
                    .merge_proofs(&proofs, merge_vd.into_iter())
                    .unwrap();
                println!("aggregation of {}-th chunk done", chunk_index + 1);
                proof
            };

            aggregated_proof = proofs[num_proofs_to_be_aggregated..]
                .chunks(aggregation_factor - 1)
                .zip(verifier_data[num_proofs_to_be_aggregated..].chunks(aggregation_factor - 1))
                .enumerate()
                .fold(aggregated_proof, |accum, (i, (proof_chunk, vd_chunk))| {
                    self.check_proof(&accum).unwrap();
                    merge_next_chunk(i, accum, proof_chunk, vd_chunk)
                });

            self.check_proof(&aggregated_proof)?;

            Ok(aggregated_proof)

        }

        // multiple checks to ensure the validity of an aggregated proof
        fn check_proof(
            &self,
            proof: &ProofWithPublicInputs<F, C, D>,
        ) -> Result<()>
            where
                [(); C::Hasher::HASH_SIZE]:,
        {
            self.aggregated_proof_circuit_data().verify(proof.clone())?;

            let num_public_inputs = proof.public_inputs.len();
            assert_eq!(
                PI::num_public_inputs() + num_targets_for_circuit_set::<F, D>(self.circuit_data.common.config.clone()),
                num_public_inputs
            );

            // negative test: change each of the public inputs
            for i in 0..num_public_inputs {
                let mut proof = proof.clone();
                proof.public_inputs[i] = F::rand();
                self.aggregated_proof_circuit_data().verify(proof).unwrap_err();
            }

            assert_eq!(RECURSION_THRESHOLD, self.aggregated_proof_circuit_data().common.degree_bits());

            Ok(())
        }
    }

    struct Circuits<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + 'static,
        const D: usize,
        PI: PublicInputAggregation,
    > {
        mul_base_circuit: MulBaseCircuit<F, C, D>,
        exp_base_circuit: ExpBaseCircuit<F, C, D>,
        mul_wrap_circuit: WrapCircuitForBaseProofs<F, C, D>,
        exp_wrap_circuit: WrapCircuitForBaseProofs<F, C, D>,
        merge_circuit: MergeCircuit<F, C, D, PI>,
    }

    impl<
            F: RichField + Extendable<D>,
            C: GenericConfig<D, F = F> + 'static,
            const D: usize,
            PI: PublicInputAggregation,
        > Circuits<F, C, D, PI>
    where
        C::Hasher: AlgebraicHasher<F>,
        [(); C::Hasher::HASH_SIZE]:,
    {
        fn build_circuits(
            config: CircuitConfig,
            aggregation_factor: usize,
        ) -> Circuits<F, C, D, PI> {
            let exp_base_circuit =
                ExpBaseCircuit::<F, C, D>::build_base_circuit(&config, RECURSION_THRESHOLD);
            let mul_base_circuit = MulBaseCircuit::<F, C, D>::build_base_circuit(
                &config,
                RECURSION_THRESHOLD,
                false,
            );
            println!("base circuit built");
            let exp_wrap_circuit = WrapCircuitForBaseProofs::build_wrap_circuit(
                &exp_base_circuit.get_circuit_data().verifier_only,
                &exp_base_circuit.get_circuit_data().common,
                &config,
            );
            let mul_wrap_circuit = WrapCircuitForBaseProofs::build_wrap_circuit(
                &mul_base_circuit.get_circuit_data().verifier_only,
                &mul_base_circuit.get_circuit_data().common,
                &config,
            );
            println!("start build aggregation circuit");
            let base_circuit_digests = vec![
                mul_wrap_circuit
                    .final_proof_circuit_data()
                    .verifier_only
                    .circuit_digest,
                exp_wrap_circuit
                    .final_proof_circuit_data()
                    .verifier_only
                    .circuit_digest,
            ];
            let merge_circuit = MergeCircuit::<F, C, D, PI>::build_merge_circuit(
                config.clone(),
                aggregation_factor,
                base_circuit_digests,
            ).unwrap();
            println!(
                "aggregation circuit size: {}",
                merge_circuit.circuit_data.common.degree_bits()
            );

            Circuits {
                mul_base_circuit,
                exp_base_circuit,
                mul_wrap_circuit,
                exp_wrap_circuit,
                merge_circuit,
            }
        }

        fn generate_base_proofs(
            &self,
            num_proofs: usize,
            init: F,
            test_case: Option<&WrongPublicInputTestCases>,
        ) -> (
            Vec<ProofWithPublicInputs<F, C, D>>,
            Vec<&VerifierOnlyCircuitData<C, D>>,
            F,
        ) {
            // employ a random circuit set digest to be employed as a public input when
            // wrapping base proofs in case of `CraftedCircuitSetPublicInput` test case
            let circuit_set_digest =
                if let Some(&WrongPublicInputTestCases::CraftedCircuitSetPublicInput) = test_case {
                    CircuitSetDigest::default()
                } else {
                    CircuitSetDigest::from(&self.merge_circuit.circuit_set)
                };

            let mut state = init;
            // generate base proofs interleaving the 2 base circuits, and shrink each generated proof
            // up to RECURSION_THRESHOLD size in order to later aggregate all of them
            let (base_proofs, verifier_data) = (0..num_proofs)
                .map(|i| {
                    let (proof, vd) = if i % 2 == 0 {
                        let base_proof = self.mul_base_circuit.generate_base_proof(state).unwrap();
                        (
                            self.mul_wrap_circuit
                                .wrap_proof(base_proof, circuit_set_digest.clone())
                                .unwrap(),
                            &self
                                .mul_wrap_circuit
                                .final_proof_circuit_data()
                                .verifier_only,
                        )
                    } else {
                        // use a wrong initial state for the base proof in case of `WrongPublicInputInBaseProof` negative test
                        let input_state =
                            if let Some(&WrongPublicInputTestCases::WrongPublicInputInBaseProof) =
                                test_case
                            {
                                F::rand()
                            } else {
                                state
                            };
                        let base_proof = self
                            .exp_base_circuit
                            .generate_base_proof(input_state)
                            .unwrap();
                        (
                            self.exp_wrap_circuit
                                .wrap_proof(base_proof, circuit_set_digest.clone())
                                .unwrap(),
                            &self
                                .exp_wrap_circuit
                                .final_proof_circuit_data()
                                .verifier_only,
                        )
                    };
                    println!("generated {}-th base proof", i + 1);
                    state = proof.public_inputs[1];
                    (proof, vd)
                })
                .unzip::<_, _, Vec<_>, Vec<_>>();

            (base_proofs, verifier_data, state)
        }

        fn aggregate_proofs(
            &self,
            proofs: &[ProofWithPublicInputs<F, C, D>],
            verifier_data: &[&VerifierOnlyCircuitData<C, D>],
            aggregation_factor: usize,
        ) -> Result<ProofWithPublicInputs<F, C, D>>
        where
            C::Hasher: AlgebraicHasher<F>,
            [(); C::Hasher::HASH_SIZE]:,
        {
            self.merge_circuit.aggregate_proofs(proofs,verifier_data,aggregation_factor)
        }
    }

    const D: usize = 2;
    type PC = PoseidonGoldilocksConfig;
    type F = <PC as GenericConfig<D>>::F;
    const AGGREGATION_FACTOR: usize = 4;

    #[fixture]
    #[once]
    fn circuits(_logger: ()) -> Circuits<F, PC, D, SimpleStatePublicInput> {
        Circuits::<F, PC, D, SimpleStatePublicInput>::build_circuits(
            CircuitConfig::standard_recursion_config(),
            AGGREGATION_FACTOR,
        )
    }

    #[rstest]
    #[case::no_pad(10)]
    #[case::one_proof_pad(9)]
    #[case::two_proofs_pad(5)]
    #[case::min_proofs(2)]
    #[should_panic(expected = "provide at least 2 proofs to be merged")]
    #[case::too_few_proofs_to_merge(1)]
    #[serial]
    fn test_merge_circuit(
        #[case] num_proofs: usize,
        circuits: &Circuits<F, PC, D, SimpleStatePublicInput>,
    ) {
        let circuit_set_digest = CircuitSetDigest::from(&circuits.merge_circuit.circuit_set);

        let init = F::rand();
        let (base_proofs, verifier_data, state) =
            circuits.generate_base_proofs(num_proofs, init, None);

        let final_output = state;

        let aggregated_proof = circuits
            .aggregate_proofs(
                base_proofs.as_slice(),
                verifier_data.as_slice(),
                AGGREGATION_FACTOR,
            )
            .unwrap();

        assert_eq!(aggregated_proof.public_inputs[0], init);
        assert_eq!(aggregated_proof.public_inputs[1], final_output);
        assert_eq!(
            aggregated_proof.public_inputs[2..].to_vec(),
            circuit_set_digest.flatten(),
        );
    }

    // set of test cases for negative tests that tamper with the public inputs of aggregated proofs
    enum WrongPublicInputTestCases {
        WrongPublicInputInBaseProof, // generate base proofs with inconsistent inputs, i.e., such
        // that the public output of a proof does not correspond to the public input of the
        // subsequent proof, hereby breaking the constraint for public input aggregation imposed
        // by the public input interface specified by `SimpleStatePublicInput`
        CraftedInputState, // replace public input of first base proof with an arbitrary
        // value, attempting to change the initial state
        CraftedOutputState, // replace public output of last base proof with an arbitrary
        // value, attempting to change the final state
        CraftedCircuitSetPublicInput, // generate wrapped proofs employing a wrong value for the
                                      // public input representing the set of circuits that can be aggregated
    }

    #[rstest]
    #[case::wrong_public_input(WrongPublicInputTestCases::WrongPublicInputInBaseProof)]
    #[case::crafted_final_state(WrongPublicInputTestCases::CraftedOutputState)]
    #[case::crafted_initial_state(WrongPublicInputTestCases::CraftedInputState)]
    #[case::crafted_circuit_set(WrongPublicInputTestCases::CraftedCircuitSetPublicInput)]
    #[serial]
    fn test_wrong_public_inputs(
        #[case] test_case: WrongPublicInputTestCases,
        circuits: &Circuits<F, PC, D, SimpleStatePublicInput>,
    ) {
        const NUM_PROOFS: usize = 5;

        let init = F::rand();
        let (mut base_proofs, verifier_data, _state) =
            circuits.generate_base_proofs(NUM_PROOFS, init, Some(&test_case));

        match test_case {
            WrongPublicInputTestCases::CraftedInputState => {
                base_proofs.first_mut().unwrap().public_inputs[0] = F::rand()
            }
            WrongPublicInputTestCases::CraftedOutputState => {
                base_proofs.last_mut().unwrap().public_inputs[1] = F::rand()
            }
            _ => (),
        };

        check_panic!(
            || circuits
                .aggregate_proofs(
                    base_proofs.as_slice(),
                    verifier_data.as_slice(),
                    AGGREGATION_FACTOR,
                )
                .unwrap(),
            "proof aggregation failed with wrong public inputs"
        );
    }

    // set of test cases for negative tests that tries to include a proof generated with a circuit that
    // does not belong to the set of circuits allowed by the merge circuit
    enum WrongCircuitSetTestCases {
        WrongWrapCircuit, // try to wrap base proof generated with the wrong circuit with a wrap
        // circuit belonging to the correct set of circuits
        WrapCircuitNotInSet, // employ a proper wrap circuit for the base proof generated with
        // the wrong circuit, providing as input to the merge circuit the actual verifier data
        // of such wrap circuit
        WrongVerifierData, // associate to the proof generate with the wrong circuit a wrong
        // `VerifierOnlyCircuitData`, which belongs to the correct set of circuits
        WrongSet, // associate to the proof generate with the wrong circuit a valid circuit
        // membership statement but for a fake set which includes a circuit not belonging to
        // the correct set of circuits
        WrongCircuitDigest, // associate to the proof generated with the wrong circuit a crafted
        // `VerifierOnlyData` with a `circuit_digest` belonging to the correct set
        WrongCircuitMembership, //  associate to the proof generated with the wrong circuit a fake
                                // circuit membership statement for the correct set of circuits
    }

    #[rstest]
    #[case::wrong_wrap_circuit(WrongCircuitSetTestCases::WrongWrapCircuit)]
    #[should_panic(expected = "circuit digest not found")]
    #[case::wrap_circuit_not_in_set(WrongCircuitSetTestCases::WrapCircuitNotInSet)]
    #[case::wrong_circuit_digest(WrongCircuitSetTestCases::WrongCircuitDigest)]
    #[case::wrong_verifier_data(WrongCircuitSetTestCases::WrongVerifierData)]
    #[serial]
    fn test_proof_with_wrong_circuit(
        #[case] test_case: WrongCircuitSetTestCases,
        circuits: &Circuits<F, PC, D, SimpleStatePublicInput>,
    ) {
        const NUM_PROOFS: usize = 4;
        let config = &circuits
            .mul_wrap_circuit
            .final_proof_circuit_data()
            .common
            .config;

        let init = F::rand();
        let (mut base_proofs, mut verifier_data, state) =
            circuits.generate_base_proofs(NUM_PROOFS, init, None);

        let mul_wrong_circuit = MulBaseCircuit::<F, PC, D>::build_base_circuit(
            config,
            circuits
                .mul_base_circuit
                .get_circuit_data()
                .common
                .degree_bits(),
            true,
        );
        let mut mul_wrap_circuit_tmp;
        let mul_wrap_circuit = match &test_case {
            &WrongCircuitSetTestCases::WrongWrapCircuit =>
            // employ a wrap circuit in the correct set to wrap the proof, even if this wrap
            // circuit should expect proofs generated with a different base circuit
            {
                &circuits.mul_wrap_circuit
            }
            &WrongCircuitSetTestCases::WrongCircuitDigest => {
                // build the proper wrap circuit but change the circuit digest to the one of a
                // circuit in the set
                mul_wrap_circuit_tmp = WrapCircuitForBaseProofs::build_wrap_circuit(
                    &mul_wrong_circuit.get_circuit_data().verifier_only,
                    &mul_wrong_circuit.get_circuit_data().common,
                    config,
                );
                let circuit_data = mutable_final_proof_circuit_data(&mut mul_wrap_circuit_tmp);
                // change the circuit digest both for prover and verifier data to ensure that the wrapped
                // proof is generated employing a circuit digest belonging to the correct set of circuits
                circuit_data.verifier_only.circuit_digest = circuits
                    .mul_wrap_circuit
                    .final_proof_circuit_data()
                    .verifier_only
                    .circuit_digest;
                circuit_data.prover_only.circuit_digest = circuits
                    .mul_wrap_circuit
                    .final_proof_circuit_data()
                    .prover_only
                    .circuit_digest;
                &mul_wrap_circuit_tmp
            }
            _ => {
                // in all other test cases we wrap the proof generated with the wrong circuit with
                // the proper wrap circuit
                mul_wrap_circuit_tmp = WrapCircuitForBaseProofs::build_wrap_circuit(
                    &mul_wrong_circuit.get_circuit_data().verifier_only,
                    &mul_wrong_circuit.get_circuit_data().common,
                    config,
                );
                &mul_wrap_circuit_tmp
            }
        };

        let base_proof = mul_wrong_circuit.generate_base_proof(state).unwrap();
        let wrap_proof = {
            let wrap_proof = || {
                mul_wrap_circuit.wrap_proof(
                    base_proof,
                    CircuitSetDigest::from(&circuits.merge_circuit.circuit_set),
                )
            };
            match &test_case {
                &WrongCircuitSetTestCases::WrongWrapCircuit => {
                    check_panic!(wrap_proof, "wrap proof did not fail");
                    return;
                }
                _ => wrap_proof(),
            }
        }
        .unwrap();
        base_proofs.push(wrap_proof);

        let mul_wrap_circuit_vd = match &test_case {
            &WrongCircuitSetTestCases::WrongVerifierData =>
            // associate to the proof generated with the wrong circuit a `VerifierOnlyData`
            // that belongs to the set of circuits expected by the merge circuit
            {
                &circuits
                    .mul_wrap_circuit
                    .final_proof_circuit_data()
                    .verifier_only
            }
            _ => &mul_wrap_circuit.final_proof_circuit_data().verifier_only,
        };
        verifier_data.push(mul_wrap_circuit_vd);

        let aggregate_proofs_fn = || {
            circuits.aggregate_proofs(
                base_proofs.as_slice(),
                verifier_data.as_slice(),
                AGGREGATION_FACTOR,
            )
        };

        {
            match &test_case {
                &WrongCircuitSetTestCases::WrapCircuitNotInSet => aggregate_proofs_fn(),
                _ => {
                    check_panic!(aggregate_proofs_fn, "proof aggregation did not fail");
                    return;
                }
            }
        }
        .unwrap();
    }

    #[rstest]
    #[serial]
    fn test_add_dummy_proofs(circuits: &Circuits<F, PC, D, SimpleStatePublicInput>) {
        const NUM_PROOFS: usize = 4;
        let circuit_set_digest = CircuitSetDigest::from(&circuits.merge_circuit.circuit_set);

        let init = F::rand();
        let (mut base_proofs, mut verifier_data, state) =
            circuits.generate_base_proofs(NUM_PROOFS, init, None);

        // add a useless dummy proof for aggregation
        let dummy_proof = circuits
            .merge_circuit
            .dummy_circuit
            .generate_dummy_proof(
                &base_proofs.last().unwrap().public_inputs[..SimpleStatePublicInput::num_public_inputs()],
                &circuits.merge_circuit.circuit_set,
            )
            .unwrap();
        let final_output = dummy_proof.public_inputs[1];

        base_proofs.push(dummy_proof);
        verifier_data.push(
            &circuits
                .merge_circuit
                .dummy_circuit
                .circuit_data
                .verifier_only,
        );

        let aggregated_proof = circuits
            .aggregate_proofs(
                base_proofs.as_slice(),
                verifier_data.as_slice(),
                AGGREGATION_FACTOR,
            )
            .unwrap();

        // check that the dummy proof has not changed the state
        assert_eq!(aggregated_proof.public_inputs[1], state);

        assert_eq!(aggregated_proof.public_inputs[0], init);
        assert_eq!(aggregated_proof.public_inputs[1], final_output);
        assert_eq!(
            aggregated_proof.public_inputs[2..].to_vec(),
            circuit_set_digest.flatten()
        );
    }

    #[rstest]
    #[case::wrong_set(WrongCircuitSetTestCases::WrongSet)]
    #[case::wrong_merkle_proof(WrongCircuitSetTestCases::WrongCircuitMembership)]
    #[serial]
    fn test_circuit_set_membership(
        #[case] test_case: WrongCircuitSetTestCases,
        circuits: &Circuits<F, PC, D, SimpleStatePublicInput>,
    ) {
        const NUM_PROOFS: usize = 3;
        let config = &circuits
            .mul_wrap_circuit
            .final_proof_circuit_data()
            .common
            .config;
        let circuit_set_digest = CircuitSetDigest::from(&circuits.merge_circuit.circuit_set);

        let init = F::rand();
        let (mut base_proofs, mut verifier_data, state) =
            circuits.generate_base_proofs(NUM_PROOFS, init, None);

        let mul_wrong_circuit = MulBaseCircuit::<F, PC, D>::build_base_circuit(
            config,
            circuits
                .mul_base_circuit
                .get_circuit_data()
                .common
                .degree_bits(),
            true,
        );
        let mul_wrap_circuit = WrapCircuitForBaseProofs::build_wrap_circuit(
            &mul_wrong_circuit.get_circuit_data().verifier_only,
            &mul_wrong_circuit.get_circuit_data().common,
            config,
        );

        let base_proof = mul_wrong_circuit.generate_base_proof(state).unwrap();
        let wrap_proof = mul_wrap_circuit
            .wrap_proof(base_proof, circuit_set_digest)
            .unwrap();
        base_proofs.push(wrap_proof);
        verifier_data.push(&mul_wrap_circuit.final_proof_circuit_data().verifier_only);

        // here we employ a modified version of `merge_proofs` function that allows to wrongly set
        // specific targets of the `MergeCircuit` to test that specific constraints are actually
        // enforced
        let mut pw = PartialWitness::new();
        for (pt, proof) in circuits
            .merge_circuit
            .input_targets
            .proof_targets
            .iter()
            .zip(base_proofs.iter())
        {
            pw.set_proof_with_pis_target(pt, proof);
        }

        for ((vt, vd), mpt) in circuits
            .merge_circuit
            .input_targets
            .inner_vk_targets
            .iter()
            .zip(verifier_data.into_iter())
            .zip(
                circuits
                    .merge_circuit
                    .input_targets
                    .circuit_set_membership_targets
                    .iter(),
            )
        {
            // `wrong_proof` is true iff the verifier data in this iteration corresponds to the one
            // of the circuit which is not included in the correct set of circuits
            let wrong_proof = vd.circuit_digest
                == mul_wrap_circuit
                    .final_proof_circuit_data()
                    .verifier_only
                    .circuit_digest;

            pw.set_cap_target(&vt.constants_sigmas_cap, &vd.constants_sigmas_cap);
            pw.set_hash_target(vt.circuit_digest, vd.circuit_digest);

            if wrong_proof {
                match &test_case {
                    &WrongCircuitSetTestCases::WrongSet => {
                        // build a fake set which includes the wrong circuit
                        let circuit_set = CircuitSet::<F, PC, D>::build_circuit_set(vec![
                            mul_wrap_circuit
                                .final_proof_circuit_data()
                                .verifier_only
                                .circuit_digest,
                            circuits
                                .mul_wrap_circuit
                                .final_proof_circuit_data()
                                .verifier_only
                                .circuit_digest,
                            circuits
                                .exp_wrap_circuit
                                .final_proof_circuit_data()
                                .verifier_only
                                .circuit_digest,
                        ]);
                        // employ this set to set values for circuit membership targets
                        circuit_set
                            .set_circuit_membership_target(&mut pw, mpt, vd.circuit_digest)
                            .unwrap()
                    }
                    &WrongCircuitSetTestCases::WrongCircuitMembership => circuits
                        .merge_circuit
                        .circuit_set
                        .set_circuit_membership_target(
                            &mut pw,
                            mpt,
                            // employ to compute assignments to the circuit set membership target a
                            // circuit digest which is in `merge_circuit.circuit_set` but
                            // it does not correspond to the one of the circuit employed to generate
                            // the proof being recursively verified in the
                            circuits
                                .mul_wrap_circuit
                                .final_proof_circuit_data()
                                .verifier_only
                                .circuit_digest,
                        )
                        .unwrap(),
                    _ => panic!("unexpected test case"),
                }
            } else {
                circuits
                    .merge_circuit
                    .circuit_set
                    .set_circuit_membership_target(&mut pw, mpt, vd.circuit_digest)
                    .unwrap();
            }
        }

        CircuitSetDigest::from(&circuits.merge_circuit.circuit_set).set_circuit_set_target(
            &mut pw,
            &circuits.merge_circuit.input_targets.circuit_set_target,
        );

        check_panic!(
            || circuits.merge_circuit.circuit_data.prove(pw),
            "proof aggregation with wrong circuit set did not fail"
        );
    }

    #[rstest]
    #[serial]
    fn test_wrong_public_input_aggregation_scheme(_logger: ()) {
        let config = CircuitConfig::standard_recursion_config();
        // build circuits specifying an inconsistent public input aggregation scheme to be
        // employed by the `MergeCircuit` to aggregate public inputs
        let circuits = Circuits::<F, PC, D, MerkleRootPublicInput<0>>::build_circuits(
            config.clone(),
            AGGREGATION_FACTOR,
        );
        let num_proofs = 5;

        let init = F::rand();
        let (base_proofs, verifier_data, _) = circuits.generate_base_proofs(num_proofs, init, None);

        check_panic!(
            || circuits
                .aggregate_proofs(
                    base_proofs.as_slice(),
                    verifier_data.as_slice(),
                    AGGREGATION_FACTOR,
                )
                .unwrap(),
            "proof aggregation with wrong public input scheme did not panic"
        )
    }

    // Simple circuit to test the `PublicInputAccumulator` public input scheme, which employs the
    // conditional aggregation of public inputs for dummy proofs
    struct PublicInputAccumulatorBaseCircuit<F: RichField + Extendable<D>,
        C: GenericConfig<D, F=F>,
        const D: usize> {
        private_input_values: [Target; 2],
        circuit_data: CircuitData<F,C,D>
    }

    impl<F: RichField + Extendable<D>,
        C: GenericConfig<D, F=F>,
        const D: usize> PublicInputAccumulatorBaseCircuit<F,C,D>
        where
            C::Hasher: AlgebraicHasher<F>,
    {

        fn build_base_circuit(config: CircuitConfig) -> Self {
            let mut builder = CircuitBuilder::<F,D>::new(config);

            let private_input_values: [Target; 2] = builder.add_virtual_targets(2).try_into().unwrap();
            let input_hash = builder.hash_n_to_hash_no_pad::<C::Hasher>(private_input_values.to_vec());

            let output_value = (0..1024).fold(private_input_values, |values, i| {
                let next = builder.mul_const_add(F::from_canonical_u64(i),values[0], values[1]);
                [values[1], next]
            })[1];

            let output_hash = builder.hash_n_to_hash_no_pad::<C::Hasher>(vec![output_value]);

            let public_inputs = PublicInputAccumulator::new(input_hash, output_hash);
            public_inputs.register_public_inputs(&mut builder);

            let data = builder.build::<C>();

            Self {
                private_input_values,
                circuit_data: data,
            }
        }

        fn generate_base_proof(&self, init_values: [F; 2]) -> Result<ProofWithPublicInputs<F,C,D>> {
            let mut pw = PartialWitness::<F>::new();

            pw.set_target_arr(self.private_input_values, init_values);

            self.circuit_data.prove(pw)
        }
    }

    struct CircuitsForPublicInputAccumulatorTests<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + 'static,
        const D: usize,
    > {
        base_circuit: PublicInputAccumulatorBaseCircuit<F, C, D>,
        wrap_circuit: WrapCircuitForBaseProofs<F, C, D>,
        merge_circuit: MergeCircuit<F, C, D, PublicInputAccumulator>,
    }

    impl<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + 'static,
        const D: usize,
    > CircuitsForPublicInputAccumulatorTests<F,C,D>
    where
        C::Hasher: AlgebraicHasher<F>,
        [(); C::Hasher::HASH_SIZE]:,
    {
        fn build_circuits(config: CircuitConfig) -> Self {
            let base_circuit = PublicInputAccumulatorBaseCircuit::<F,C,D>::build_base_circuit(config.clone());

            let wrap_circuit = WrapCircuitForBaseProofs::build_wrap_circuit(
                &base_circuit.circuit_data.verifier_only,
                &base_circuit.circuit_data.common,
                &config,
            );


            let merge_circuit = MergeCircuit::<F,C,D,PublicInputAccumulator>::build_merge_circuit(
                config,
                AGGREGATION_FACTOR,
                vec![wrap_circuit.final_proof_circuit_data().verifier_only.circuit_digest],
            ).unwrap();

            assert_eq!(wrap_circuit.final_proof_circuit_data().common, merge_circuit.aggregated_proof_circuit_data().common);

            Self {
                base_circuit,
                wrap_circuit,
                merge_circuit
            }
        }

        fn generate_base_proofs(&self, num_proofs: usize) -> Result<Vec<ProofWithPublicInputs<F,C,D>>> {
            (0..num_proofs).map(
                |_| {
                    let input_values = [F::rand(), F::rand()];
                    let proof = self.base_circuit.generate_base_proof(input_values)?;
                    self.wrap_circuit.wrap_proof(proof, CircuitSetDigest::from(&self.merge_circuit.circuit_set))
                }
            ).collect::<Result<Vec<_>>>()
        }
    }

    #[fixture]
    #[once]
    fn circuits_for_public_input_accumulator(_logger: ())
        -> CircuitsForPublicInputAccumulatorTests<F,PC,D> {
        CircuitsForPublicInputAccumulatorTests::<F,PC,D>::build_circuits(
            CircuitConfig::standard_recursion_config()
        )
    }

    #[rstest]
    #[serial]
    fn test_merge_circuit_with_conditional_input_aggregation(
        circuits_for_public_input_accumulator: &CircuitsForPublicInputAccumulatorTests<F,PC,D>,
    ) {
        let num_proofs = 8;
        let base_proofs = circuits_for_public_input_accumulator.generate_base_proofs(num_proofs).unwrap();

        let aggregated_proof = circuits_for_public_input_accumulator.merge_circuit.aggregate_proofs(
            &base_proofs[..num_proofs-1],
            vec![&circuits_for_public_input_accumulator.wrap_circuit.final_proof_circuit_data().verifier_only; num_proofs-1].as_slice(),
            AGGREGATION_FACTOR
        ).unwrap();

        // compute the final input/output accumulators expected by merging `aggregated_proof` with
        // the last base proofs, which have not been merged with the other proofs yet.
        // According to the `PublicInputAccumulator` aggregation strategy, the aggregated input
        // accumulator is computed as H(input1||input2), and similarly for the output accumulator
        let num_public_inputs = PublicInputAccumulator::num_public_inputs();
        let mut input_accumulator = aggregated_proof.public_inputs[..num_public_inputs/2].to_vec();
        let mut output_accumulator = aggregated_proof.public_inputs[num_public_inputs/2..num_public_inputs].to_vec();

        input_accumulator.extend_from_slice(&base_proofs[num_proofs-1].public_inputs[..num_public_inputs/2]);
        output_accumulator.extend_from_slice(&base_proofs[num_proofs-1].public_inputs[num_public_inputs/2..num_public_inputs]);

        let final_public_input = hash_n_to_hash_no_pad::<
            _,
            <<PC as GenericConfig<D>>::Hasher as Hasher<F>>::Permutation,
        >(&input_accumulator);
        let final_public_output = hash_n_to_hash_no_pad::<
            _,
            <<PC as GenericConfig<D>>::Hasher as Hasher<F>>::Permutation,
        >(&output_accumulator);

        // explicitly add a dummy proof to be aggregated in order to check that it does not affect
        // the final accumulators of the aggregated proof
        let dummy_proof = circuits_for_public_input_accumulator
            .merge_circuit
            .dummy_circuit
            .generate_dummy_proof(
                &base_proofs.last().unwrap().public_inputs[..PublicInputAccumulator::num_public_inputs()],
                &circuits_for_public_input_accumulator.merge_circuit.circuit_set,
            )
            .unwrap();

        let final_aggregated_proof = circuits_for_public_input_accumulator.merge_circuit.aggregate_proofs(
            vec![aggregated_proof, base_proofs[num_proofs-1].clone(), dummy_proof].as_slice(),
            vec![
                &circuits_for_public_input_accumulator.merge_circuit.aggregated_proof_circuit_data().verifier_only,
                &circuits_for_public_input_accumulator.wrap_circuit.final_proof_circuit_data().verifier_only,
                &circuits_for_public_input_accumulator.merge_circuit.dummy_circuit.circuit_data.verifier_only,
            ].as_slice(),
            AGGREGATION_FACTOR
        ).unwrap();


        assert_eq!(
            final_public_input.to_vec().as_slice(),
            &final_aggregated_proof.public_inputs[..num_public_inputs/2],
        );

        assert_eq!(
            final_public_output.to_vec().as_slice(),
            &final_aggregated_proof.public_inputs[num_public_inputs/2..num_public_inputs],
        );
    }

    #[rstest]
    #[serial]
    fn test_aggregation_of_dummy_proofs_only(
        circuits_for_public_input_accumulator: &CircuitsForPublicInputAccumulatorTests<F,PC,D>,
    ) {
        let dummy_proofs = (0..2).map(|_|
              {
                  // generate random values as public inputs
                  let input_values = (0..PublicInputAccumulator::num_public_inputs()).map(|_|
                    F::rand()
                  ).collect::<Vec<_>>();
                  circuits_for_public_input_accumulator.merge_circuit.dummy_circuit.generate_dummy_proof(
                      input_values.as_slice(),
                      &circuits_for_public_input_accumulator.merge_circuit.circuit_set,
                    )
              }
        ).collect::<Result<Vec<_>>>().unwrap();


        check_panic!(
            ||
            circuits_for_public_input_accumulator.merge_circuit.aggregate_proofs(dummy_proofs.as_slice(), vec![
                &circuits_for_public_input_accumulator.merge_circuit.dummy_circuit.circuit_data.verifier_only; 2
                ].as_slice(),
            AGGREGATION_FACTOR,
            )
            .unwrap(),
            "proof aggregation with only dummy proofs did not panic"
        );
    }


    #[rstest]
    #[serial]
    fn test_aggregation_of_real_proof_with_dummy_proof(
        circuits_for_public_input_accumulator: &CircuitsForPublicInputAccumulatorTests<F,PC,D>,
    ) {
        let real_proof = {
            let proof = circuits_for_public_input_accumulator.base_circuit.generate_base_proof([F::rand(), F::rand()]).unwrap();
            circuits_for_public_input_accumulator.wrap_circuit.wrap_proof(proof, CircuitSetDigest::from(&circuits_for_public_input_accumulator.merge_circuit.circuit_set)
            ).unwrap()
        };
        let dummy_proof = circuits_for_public_input_accumulator.merge_circuit.dummy_circuit.generate_dummy_proof(
            &real_proof.public_inputs[..PublicInputAccumulator::num_public_inputs()],
            &circuits_for_public_input_accumulator.merge_circuit.circuit_set,
        ).unwrap();

        check_panic!(
            ||
            circuits_for_public_input_accumulator.merge_circuit.aggregate_proofs(
                vec![real_proof, dummy_proof].as_slice(),
                vec![
                    &circuits_for_public_input_accumulator.merge_circuit.wrap_circuit.final_proof_circuit_data().verifier_only,
                    &circuits_for_public_input_accumulator.merge_circuit.dummy_circuit.circuit_data.verifier_only,
                ].as_slice(),
                AGGREGATION_FACTOR,
            ).unwrap(),
            "proof aggregation of real and dummy proof did not panic"
        );
    }

}
