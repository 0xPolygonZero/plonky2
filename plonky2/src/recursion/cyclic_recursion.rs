#![allow(clippy::int_plus_one)] // Makes more sense for some inequalities below.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use anyhow::{ensure, Result};

use crate::field::extension::Extendable;
use crate::hash::hash_types::{HashOut, HashOutTarget, MerkleCapTarget, RichField};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{
    CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
};
use crate::plonk::config::{AlgebraicHasher, GenericConfig};
use crate::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use crate::util::serialization::{Buffer, IoResult, Read, Write};

impl<C: GenericConfig<D>, const D: usize> VerifierOnlyCircuitData<C, D> {
    fn from_slice(slice: &[C::F], common_data: &CommonCircuitData<C::F, D>) -> Result<Self>
    where
        C::Hasher: AlgebraicHasher<C::F>,
    {
        // The structure of the public inputs is `[..., circuit_digest, constants_sigmas_cap]`.
        let cap_len = common_data.config.fri_config.num_cap_elements();
        let len = slice.len();
        ensure!(len >= 4 + 4 * cap_len, "Not enough public inputs");
        let constants_sigmas_cap = MerkleCap(
            (0..cap_len)
                .map(|i| HashOut {
                    elements: core::array::from_fn(|j| slice[len - 4 * (cap_len - i) + j]),
                })
                .collect(),
        );
        let circuit_digest =
            HashOut::from_partial(&slice[len - 4 - 4 * cap_len..len - 4 * cap_len]);

        Ok(Self {
            circuit_digest,
            constants_sigmas_cap,
        })
    }
}

impl VerifierCircuitTarget {
    pub fn to_bytes(&self) -> IoResult<Vec<u8>> {
        let mut buffer = Vec::new();
        buffer.write_target_merkle_cap(&self.constants_sigmas_cap)?;
        buffer.write_target_hash(&self.circuit_digest)?;
        Ok(buffer)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> IoResult<Self> {
        let mut buffer = Buffer::new(&bytes);
        let constants_sigmas_cap = buffer.read_target_merkle_cap()?;
        let circuit_digest = buffer.read_target_hash()?;
        Ok(Self {
            constants_sigmas_cap,
            circuit_digest,
        })
    }

    fn from_slice<F: RichField + Extendable<D>, const D: usize>(
        slice: &[Target],
        common_data: &CommonCircuitData<F, D>,
    ) -> Result<Self> {
        let cap_len = common_data.config.fri_config.num_cap_elements();
        let len = slice.len();
        ensure!(len >= 4 + 4 * cap_len, "Not enough public inputs");
        let constants_sigmas_cap = MerkleCapTarget(
            (0..cap_len)
                .map(|i| HashOutTarget {
                    elements: core::array::from_fn(|j| slice[len - 4 * (cap_len - i) + j]),
                })
                .collect(),
        );
        let circuit_digest = HashOutTarget {
            elements: core::array::from_fn(|i| slice[len - 4 - 4 * cap_len + i]),
        };

        Ok(Self {
            circuit_digest,
            constants_sigmas_cap,
        })
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// If `condition` is true, recursively verify a proof for the same circuit as the one we're
    /// currently building. Otherwise, verify `other_proof_with_pis`.
    ///
    /// For a typical IVC use case, `condition` will be false for the very first proof in a chain,
    /// i.e. the base case.
    ///
    /// Note that this does not enforce that the inner circuit uses the correct verification key.
    /// This is not possible to check in this recursive circuit, since we do not know the
    /// verification key until after we build it. Verifiers must separately call
    /// `check_cyclic_proof_verifier_data`, in addition to verifying a recursive proof, to check
    /// that the verification key matches.
    ///
    /// WARNING: Do not register any public input after calling this! TODO: relax this
    pub fn conditionally_verify_cyclic_proof<C: GenericConfig<D, F = F>>(
        &mut self,
        condition: BoolTarget,
        cyclic_proof_with_pis: &ProofWithPublicInputsTarget<D>,
        other_proof_with_pis: &ProofWithPublicInputsTarget<D>,
        other_verifier_data: &VerifierCircuitTarget,
        common_data: &CommonCircuitData<F, D>,
    ) -> Result<()>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let verifier_data = self
            .verifier_data_public_input
            .clone()
            .expect("Must call add_verifier_data_public_inputs before cyclic recursion");

        if let Some(existing_common_data) = self.goal_common_data.as_ref() {
            assert_eq!(existing_common_data, common_data);
        } else {
            self.goal_common_data = Some(common_data.clone());
        }

        let inner_cyclic_pis = VerifierCircuitTarget::from_slice::<F, D>(
            &cyclic_proof_with_pis.public_inputs,
            common_data,
        )?;
        // Connect previous verifier data to current one. This guarantees that every proof in the cycle uses the same verifier data.
        self.connect_hashes(
            inner_cyclic_pis.circuit_digest,
            verifier_data.circuit_digest,
        );
        self.connect_merkle_caps(
            &inner_cyclic_pis.constants_sigmas_cap,
            &verifier_data.constants_sigmas_cap,
        );

        // Verify the cyclic proof if `condition` is set to true, otherwise verify the other proof.
        self.conditionally_verify_proof::<C>(
            condition,
            cyclic_proof_with_pis,
            &verifier_data,
            other_proof_with_pis,
            other_verifier_data,
            common_data,
        );

        // Make sure we have every gate to match `common_data`.
        for g in &common_data.gates {
            self.add_gate_to_gate_set(g.clone());
        }

        Ok(())
    }

    pub fn conditionally_verify_cyclic_proof_or_dummy<C: GenericConfig<D, F = F> + 'static>(
        &mut self,
        condition: BoolTarget,
        cyclic_proof_with_pis: &ProofWithPublicInputsTarget<D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> Result<()>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let (dummy_proof_with_pis_target, dummy_verifier_data_target) =
            self.dummy_proof_and_vk::<C>(common_data)?;
        self.conditionally_verify_cyclic_proof::<C>(
            condition,
            cyclic_proof_with_pis,
            &dummy_proof_with_pis_target,
            &dummy_verifier_data_target,
            common_data,
        )?;
        Ok(())
    }
}

/// Additional checks to be performed on a cyclic recursive proof in addition to verifying the proof.
/// Checks that the purported verifier data in the public inputs match the real verifier data.
pub fn check_cyclic_proof_verifier_data<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    proof: &ProofWithPublicInputs<F, C, D>,
    verifier_data: &VerifierOnlyCircuitData<C, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()>
where
    C::Hasher: AlgebraicHasher<F>,
{
    let pis = VerifierOnlyCircuitData::<C, D>::from_slice(&proof.public_inputs, common_data)?;
    ensure!(verifier_data.constants_sigmas_cap == pis.constants_sigmas_cap);
    ensure!(verifier_data.circuit_digest == pis.circuit_digest);

    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    use anyhow::Result;

    use crate::field::extension::Extendable;
    use crate::field::types::{Field, PrimeField64};
    use crate::gates::noop::NoopGate;
    use crate::hash::hash_types::{HashOutTarget, RichField};
    use crate::hash::hashing::hash_n_to_hash_no_pad;
    use crate::hash::poseidon::{PoseidonHash, PoseidonPermutation};
    use crate::iop::witness::{PartialWitness, WitnessWrite};
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData};
    use crate::plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig};
    use crate::recursion::cyclic_recursion::check_cyclic_proof_verifier_data;
    use crate::recursion::dummy_circuit::cyclic_base_proof;

    // Generates `CommonCircuitData` usable for recursion.
    fn common_data_for_recursion<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >() -> CommonCircuitData<F, D>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let config = CircuitConfig::standard_recursion_config();
        let builder = CircuitBuilder::<F, D>::new(config);
        let data = builder.build::<C>();
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let proof = builder.add_virtual_proof_with_pis(&data.common);
        let verifier_data =
            builder.add_virtual_verifier_data(data.common.config.fri_config.cap_height);
        builder.verify_proof::<C>(&proof, &verifier_data, &data.common);
        let data = builder.build::<C>();

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let proof = builder.add_virtual_proof_with_pis(&data.common);
        let verifier_data =
            builder.add_virtual_verifier_data(data.common.config.fri_config.cap_height);
        builder.verify_proof::<C>(&proof, &verifier_data, &data.common);
        while builder.num_gates() < 1 << 12 {
            builder.add_gate(NoopGate, vec![]);
        }
        builder.build::<C>().common
    }

    /// Uses cyclic recursion to build a hash chain.
    /// The circuit has the following public input structure:
    /// - Initial hash (4)
    /// - Output for the tip of the hash chain (4)
    /// - Chain length, i.e. the number of times the hash has been applied (1)
    /// - VK for cyclic recursion (?)
    #[test]
    fn test_cyclic_recursion() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let one = builder.one();

        // Circuit that computes a repeated hash.
        let initial_hash_target = builder.add_virtual_hash();
        builder.register_public_inputs(&initial_hash_target.elements);
        let current_hash_in = builder.add_virtual_hash();
        let current_hash_out =
            builder.hash_n_to_hash_no_pad::<PoseidonHash>(current_hash_in.elements.to_vec());
        builder.register_public_inputs(&current_hash_out.elements);
        let counter = builder.add_virtual_public_input();

        let mut common_data = common_data_for_recursion::<F, C, D>();
        let verifier_data_target = builder.add_verifier_data_public_inputs();
        common_data.num_public_inputs = builder.num_public_inputs();

        let condition = builder.add_virtual_bool_target_safe();

        // Unpack inner proof's public inputs.
        let inner_cyclic_proof_with_pis = builder.add_virtual_proof_with_pis(&common_data);
        let inner_cyclic_pis = &inner_cyclic_proof_with_pis.public_inputs;
        let inner_cyclic_initial_hash = HashOutTarget::try_from(&inner_cyclic_pis[0..4]).unwrap();
        let inner_cyclic_latest_hash = HashOutTarget::try_from(&inner_cyclic_pis[4..8]).unwrap();
        let inner_cyclic_counter = inner_cyclic_pis[8];

        // Connect our initial hash to that of our inner proof. (If there is no inner proof, the
        // initial hash will be unconstrained, which is intentional.)
        builder.connect_hashes(initial_hash_target, inner_cyclic_initial_hash);

        // The input hash is the previous hash output if we have an inner proof, or the initial hash
        // if this is the base case.
        let actual_hash_in =
            builder.select_hash(condition, inner_cyclic_latest_hash, initial_hash_target);
        builder.connect_hashes(current_hash_in, actual_hash_in);

        // Our chain length will be inner_counter + 1 if we have an inner proof, or 1 if not.
        let new_counter = builder.mul_add(condition.target, inner_cyclic_counter, one);
        builder.connect(counter, new_counter);

        builder.conditionally_verify_cyclic_proof_or_dummy::<C>(
            condition,
            &inner_cyclic_proof_with_pis,
            &common_data,
        )?;

        let cyclic_circuit_data = builder.build::<C>();

        let mut pw = PartialWitness::new();
        let initial_hash = [F::ZERO, F::ONE, F::TWO, F::from_canonical_usize(3)];
        let initial_hash_pis = initial_hash.into_iter().enumerate().collect();
        pw.set_bool_target(condition, false)?;
        pw.set_proof_with_pis_target::<C, D>(
            &inner_cyclic_proof_with_pis,
            &cyclic_base_proof(
                &common_data,
                &cyclic_circuit_data.verifier_only,
                initial_hash_pis,
            ),
        )?;
        pw.set_verifier_data_target(&verifier_data_target, &cyclic_circuit_data.verifier_only)?;
        let proof = cyclic_circuit_data.prove(pw)?;
        check_cyclic_proof_verifier_data(
            &proof,
            &cyclic_circuit_data.verifier_only,
            &cyclic_circuit_data.common,
        )?;
        cyclic_circuit_data.verify(proof.clone())?;

        // 1st recursive layer.
        let mut pw = PartialWitness::new();
        pw.set_bool_target(condition, true)?;
        pw.set_proof_with_pis_target(&inner_cyclic_proof_with_pis, &proof)?;
        pw.set_verifier_data_target(&verifier_data_target, &cyclic_circuit_data.verifier_only)?;
        let proof = cyclic_circuit_data.prove(pw)?;
        check_cyclic_proof_verifier_data(
            &proof,
            &cyclic_circuit_data.verifier_only,
            &cyclic_circuit_data.common,
        )?;
        cyclic_circuit_data.verify(proof.clone())?;

        // 2nd recursive layer.
        let mut pw = PartialWitness::new();
        pw.set_bool_target(condition, true)?;
        pw.set_proof_with_pis_target(&inner_cyclic_proof_with_pis, &proof)?;
        pw.set_verifier_data_target(&verifier_data_target, &cyclic_circuit_data.verifier_only)?;
        let proof = cyclic_circuit_data.prove(pw)?;
        check_cyclic_proof_verifier_data(
            &proof,
            &cyclic_circuit_data.verifier_only,
            &cyclic_circuit_data.common,
        )?;

        // Verify that the proof correctly computes a repeated hash.
        let initial_hash = &proof.public_inputs[..4];
        let hash = &proof.public_inputs[4..8];
        let counter = proof.public_inputs[8];
        let expected_hash: [F; 4] = iterate_poseidon(
            initial_hash.try_into().unwrap(),
            counter.to_canonical_u64() as usize,
        );
        assert_eq!(hash, expected_hash);

        cyclic_circuit_data.verify(proof)
    }

    fn iterate_poseidon<F: RichField>(initial_state: [F; 4], n: usize) -> [F; 4] {
        let mut current = initial_state;
        for _ in 0..n {
            current = hash_n_to_hash_no_pad::<F, PoseidonPermutation<F>>(&current).elements;
        }
        current
    }
}
