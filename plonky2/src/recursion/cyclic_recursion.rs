#![allow(clippy::int_plus_one)] // Makes more sense for some inequalities below.

use alloc::vec;

use anyhow::{ensure, Result};
use hashbrown::HashMap;
use itertools::Itertools;

use crate::field::extension::Extendable;
use crate::gates::noop::NoopGate;
use crate::hash::hash_types::{HashOut, HashOutTarget, MerkleCapTarget, RichField};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartialWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{
    CircuitData, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
};
use crate::plonk::config::{AlgebraicHasher, GenericConfig};
use crate::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use crate::recursion::dummy_circuit::{dummy_circuit, dummy_proof};

pub struct CyclicRecursionData<
    'a,
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    proof: &'a Option<ProofWithPublicInputs<F, C, D>>,
    verifier_data: &'a VerifierOnlyCircuitData<C, D>,
    common_data: &'a CommonCircuitData<F, D>,
}

pub struct CyclicRecursionTarget<F, C, const D: usize>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    pub(crate) proof: ProofWithPublicInputsTarget<D>,
    pub(crate) verifier_data: VerifierCircuitTarget,
    pub(crate) dummy_proof: ProofWithPublicInputsTarget<D>,
    pub(crate) dummy_verifier_data: VerifierCircuitTarget,
    pub(crate) condition: BoolTarget,
    pub(crate) dummy_circuit: CircuitData<F, C, D>,
}

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
    fn from_slice<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
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
    /// currently building.
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
    pub fn cyclic_recursion<C: GenericConfig<D, F = F>>(
        &mut self,
        condition: BoolTarget,
        proof_with_pis: &ProofWithPublicInputsTarget<D>,
        common_data: &CommonCircuitData<F, D>,
    ) -> Result<CyclicRecursionTarget<F, C, D>>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let verifier_data = self
            .verifier_data_public_input
            .clone()
            .expect("Must call add_verifier_data_public_inputs before cyclic recursion");
        self.goal_common_data = Some(common_data.clone());

        let dummy_verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: self.add_virtual_cap(self.config.fri_config.cap_height),
            circuit_digest: self.add_virtual_hash(),
        };

        let dummy_proof = self.add_virtual_proof_with_pis::<C>(common_data);

        let pis = VerifierCircuitTarget::from_slice::<F, C, D>(
            &proof_with_pis.public_inputs,
            common_data,
        )?;
        // Connect previous verifier data to current one. This guarantees that every proof in the cycle uses the same verifier data.
        self.connect_hashes(pis.circuit_digest, verifier_data.circuit_digest);
        for (h0, h1) in pis
            .constants_sigmas_cap
            .0
            .iter()
            .zip_eq(&verifier_data.constants_sigmas_cap.0)
        {
            self.connect_hashes(*h0, *h1);
        }

        // Verify the real proof if `condition` is set to true, otherwise verify the dummy proof.
        self.conditionally_verify_proof::<C>(
            condition,
            proof_with_pis,
            &verifier_data,
            &dummy_proof,
            &dummy_verifier_data,
            common_data,
        );

        // Make sure we have enough gates to match `common_data`.
        while self.num_gates() < (common_data.degree() / 2) {
            self.add_gate(NoopGate, vec![]);
        }
        // Make sure we have every gate to match `common_data`.
        for g in &common_data.gates {
            self.add_gate_to_gate_set(g.clone());
        }

        Ok(CyclicRecursionTarget {
            proof: proof_with_pis.clone(),
            verifier_data,
            dummy_proof,
            dummy_verifier_data,
            condition,
            dummy_circuit: dummy_circuit(common_data),
        })
    }
}

/// Set the targets in a `CyclicRecursionTarget` to their corresponding values in a `CyclicRecursionData`.
/// The `public_inputs` parameter let the caller specify certain public inputs (identified by their
/// indices) which should be given specific values. The rest will default to zero.
pub fn set_cyclic_recursion_data_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    pw: &mut PartialWitness<F>,
    cyclic_recursion_data_target: &CyclicRecursionTarget<F, C, D>,
    cyclic_recursion_data: &CyclicRecursionData<F, C, D>,
    // Public inputs to set in the base case to seed some initial data.
    mut public_inputs: HashMap<usize, F>,
) -> Result<()>
where
    C::Hasher: AlgebraicHasher<F>,
{
    if let Some(proof) = cyclic_recursion_data.proof {
        pw.set_bool_target(cyclic_recursion_data_target.condition, true);
        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.proof, proof);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.verifier_data,
            cyclic_recursion_data.verifier_data,
        );
        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.dummy_proof, proof);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.dummy_verifier_data,
            cyclic_recursion_data.verifier_data,
        );
    } else {
        pw.set_bool_target(cyclic_recursion_data_target.condition, false);

        let pis_len = cyclic_recursion_data_target
            .dummy_circuit
            .common
            .num_public_inputs;
        let cap_elements = cyclic_recursion_data
            .common_data
            .config
            .fri_config
            .num_cap_elements();
        let start_vk_pis = pis_len - 4 - 4 * cap_elements;

        // The circuit checks that the verifier data is the same throughout the cycle, so
        // we set the verifier data to the "real" verifier data even though it's unused in the base case.
        let verifier_data = &cyclic_recursion_data.verifier_data;
        public_inputs.extend((start_vk_pis..).zip(verifier_data.circuit_digest.elements));

        for i in 0..cap_elements {
            let start = start_vk_pis + 4 + 4 * i;
            public_inputs.extend((start..).zip(verifier_data.constants_sigmas_cap.0[i].elements));
        }

        let proof = dummy_proof(&cyclic_recursion_data_target.dummy_circuit, public_inputs)?;
        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.proof, &proof);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.verifier_data,
            cyclic_recursion_data.verifier_data,
        );

        let dummy_p = dummy_proof(&cyclic_recursion_data_target.dummy_circuit, HashMap::new())?;
        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.dummy_proof, &dummy_p);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.dummy_verifier_data,
            &cyclic_recursion_data_target.dummy_circuit.verifier_only,
        );
    }

    Ok(())
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
    use anyhow::Result;
    use hashbrown::HashMap;

    use crate::field::extension::Extendable;
    use crate::field::types::{Field, PrimeField64};
    use crate::gates::noop::NoopGate;
    use crate::hash::hash_types::{HashOutTarget, RichField};
    use crate::hash::hashing::hash_n_to_hash_no_pad;
    use crate::hash::poseidon::{PoseidonHash, PoseidonPermutation};
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData, VerifierCircuitTarget};
    use crate::plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig};
    use crate::recursion::cyclic_recursion::{
        check_cyclic_proof_verifier_data, set_cyclic_recursion_data_target, CyclicRecursionData,
    };

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
        let proof = builder.add_virtual_proof_with_pis::<C>(&data.common);
        let verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: builder.add_virtual_cap(data.common.config.fri_config.cap_height),
            circuit_digest: builder.add_virtual_hash(),
        };
        builder.verify_proof::<C>(&proof, &verifier_data, &data.common);
        let data = builder.build::<C>();

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let proof = builder.add_virtual_proof_with_pis::<C>(&data.common);
        let verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: builder.add_virtual_cap(data.common.config.fri_config.cap_height),
            circuit_digest: builder.add_virtual_hash(),
        };
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
        let initial_hash = builder.add_virtual_hash();
        builder.register_public_inputs(&initial_hash.elements);
        let current_hash_in = builder.add_virtual_hash();
        let current_hash_out =
            builder.hash_n_to_hash_no_pad::<PoseidonHash>(current_hash_in.elements.to_vec());
        builder.register_public_inputs(&current_hash_out.elements);
        let counter = builder.add_virtual_public_input();

        let mut common_data = common_data_for_recursion::<F, C, D>();
        builder.add_verifier_data_public_inputs();
        common_data.num_public_inputs = builder.num_public_inputs();

        let condition = builder.add_virtual_bool_target_safe();

        // Unpack inner proof's public inputs.
        let inner_proof_with_pis = builder.add_virtual_proof_with_pis::<C>(&common_data);
        let inner_pis = &inner_proof_with_pis.public_inputs;
        let inner_initial_hash = HashOutTarget::try_from(&inner_pis[0..4]).unwrap();
        let inner_latest_hash = HashOutTarget::try_from(&inner_pis[4..8]).unwrap();
        let inner_counter = inner_pis[8];

        // Connect our initial hash to that of our inner proof. (If there is no inner proof, the
        // initial hash will be unconstrained, which is intentional.)
        builder.connect_hashes(initial_hash, inner_initial_hash);

        // The input hash is the previous hash output if we have an inner proof, or the initial hash
        // if this is the base case.
        let actual_hash_in = builder.select_hash(condition, inner_latest_hash, initial_hash);
        builder.connect_hashes(current_hash_in, actual_hash_in);

        // Our chain length will be inner_counter + 1 if we have an inner proof, or 1 if not.
        let new_counter = builder.mul_add(condition.target, inner_counter, one);
        builder.connect(counter, new_counter);

        let cyclic_data_target =
            builder.cyclic_recursion::<C>(condition, &inner_proof_with_pis, &common_data)?;

        let cyclic_circuit_data = builder.build::<C>();

        let mut pw = PartialWitness::new();
        let cyclic_recursion_data = CyclicRecursionData {
            proof: &None, // Base case: We don't have a proof to put here yet.
            verifier_data: &cyclic_circuit_data.verifier_only,
            common_data: &cyclic_circuit_data.common,
        };
        let initial_hash = [F::ZERO, F::ONE, F::TWO, F::from_canonical_usize(3)];
        let initial_hash_pis = initial_hash.into_iter().enumerate().collect();
        set_cyclic_recursion_data_target(
            &mut pw,
            &cyclic_data_target,
            &cyclic_recursion_data,
            initial_hash_pis,
        )?;
        let proof = cyclic_circuit_data.prove(pw)?;
        check_cyclic_proof_verifier_data(
            &proof,
            cyclic_recursion_data.verifier_data,
            cyclic_recursion_data.common_data,
        )?;
        cyclic_circuit_data.verify(proof.clone())?;

        // 1st recursive layer.
        let mut pw = PartialWitness::new();
        let cyclic_recursion_data = CyclicRecursionData {
            proof: &Some(proof), // Input previous proof.
            verifier_data: &cyclic_circuit_data.verifier_only,
            common_data: &cyclic_circuit_data.common,
        };
        set_cyclic_recursion_data_target(
            &mut pw,
            &cyclic_data_target,
            &cyclic_recursion_data,
            HashMap::new(),
        )?;
        let proof = cyclic_circuit_data.prove(pw)?;
        check_cyclic_proof_verifier_data(
            &proof,
            cyclic_recursion_data.verifier_data,
            cyclic_recursion_data.common_data,
        )?;
        cyclic_circuit_data.verify(proof.clone())?;

        // 2nd recursive layer.
        let mut pw = PartialWitness::new();
        let cyclic_recursion_data = CyclicRecursionData {
            proof: &Some(proof), // Input previous proof.
            verifier_data: &cyclic_circuit_data.verifier_only,
            common_data: &cyclic_circuit_data.common,
        };
        set_cyclic_recursion_data_target(
            &mut pw,
            &cyclic_data_target,
            &cyclic_recursion_data,
            HashMap::new(),
        )?;
        let proof = cyclic_circuit_data.prove(pw)?;
        check_cyclic_proof_verifier_data(
            &proof,
            cyclic_recursion_data.verifier_data,
            cyclic_recursion_data.common_data,
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
            current = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(&current).elements;
        }
        current
    }
}
