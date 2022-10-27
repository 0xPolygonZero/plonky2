#![allow(clippy::int_plus_one)] // Makes more sense for some inequalities below.
use anyhow::{ensure, Result};
use itertools::Itertools;
use plonky2_field::extension::Extendable;

use crate::gates::noop::NoopGate;
use crate::hash::hash_types::{HashOut, HashOutTarget, MerkleCapTarget, RichField};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartialWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{
    CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
};
use crate::plonk::config::Hasher;
use crate::plonk::config::{AlgebraicHasher, GenericConfig};
use crate::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use crate::recursion::conditional_recursive_verifier::dummy_proof;

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

pub struct CyclicRecursionTarget<const D: usize> {
    pub proof: ProofWithPublicInputsTarget<D>,
    pub verifier_data: VerifierCircuitTarget,
    pub dummy_proof: ProofWithPublicInputsTarget<D>,
    pub dummy_verifier_data: VerifierCircuitTarget,
    pub base_case: BoolTarget,
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
                    elements: std::array::from_fn(|j| slice[len - 4 * (cap_len - i) + j]),
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
                    elements: std::array::from_fn(|j| slice[len - 4 * (cap_len - i) + j]),
                })
                .collect(),
        );
        let circuit_digest = HashOutTarget {
            elements: std::array::from_fn(|i| slice[len - 4 - 4 * cap_len + i]),
        };

        Ok(Self {
            circuit_digest,
            constants_sigmas_cap,
        })
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Cyclic recursion gadget.
    /// WARNING: Do not register any public input after calling this! TODO: relax this
    pub fn cyclic_recursion<C: GenericConfig<D, F = F>>(
        &mut self,
        // Flag set to true for the base case of the cycle where we verify a dummy proof to bootstrap the cycle. Set to false otherwise.
        base_case: BoolTarget,
        previous_virtual_public_inputs: &[Target],
        common_data: &mut CommonCircuitData<F, D>,
    ) -> Result<CyclicRecursionTarget<D>>
    where
        C::Hasher: AlgebraicHasher<F>,
        [(); C::Hasher::HASH_SIZE]:,
    {
        if self.verifier_data_public_input.is_none() {
            self.add_verifier_data_public_input();
        }
        let verifier_data = self.verifier_data_public_input.clone().unwrap();
        common_data.num_public_inputs = self.num_public_inputs();
        self.goal_common_data = Some(common_data.clone());

        let dummy_verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: self.add_virtual_cap(self.config.fri_config.cap_height),
            circuit_digest: self.add_virtual_hash(),
        };

        let proof = self.add_virtual_proof_with_pis::<C>(common_data);
        let dummy_proof = self.add_virtual_proof_with_pis::<C>(common_data);

        let pis = VerifierCircuitTarget::from_slice::<F, C, D>(&proof.public_inputs, common_data)?;
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

        for (x, y) in previous_virtual_public_inputs
            .iter()
            .zip(&proof.public_inputs)
        {
            self.connect(*x, *y);
        }

        // Verify the dummy proof if `base_case` is set to true, otherwise verify the "real" proof.
        self.conditionally_verify_proof::<C>(
            base_case,
            &dummy_proof,
            &dummy_verifier_data,
            &proof,
            &verifier_data,
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
            proof,
            verifier_data: verifier_data.clone(),
            dummy_proof,
            dummy_verifier_data,
            base_case,
        })
    }
}

/// Set the targets in a `CyclicRecursionTarget` to their corresponding values in a `CyclicRecursionData`.
pub fn set_cyclic_recursion_data_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    pw: &mut PartialWitness<F>,
    cyclic_recursion_data_target: &CyclicRecursionTarget<D>,
    cyclic_recursion_data: &CyclicRecursionData<F, C, D>,
    // Public inputs to set in the base case to seed some initial data.
    public_inputs: &[F],
) -> Result<()>
where
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    if let Some(proof) = cyclic_recursion_data.proof {
        pw.set_bool_target(cyclic_recursion_data_target.base_case, false);
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
        let (dummy_proof, dummy_data) = dummy_proof::<F, C, D>(cyclic_recursion_data.common_data)?;
        pw.set_bool_target(cyclic_recursion_data_target.base_case, true);
        let mut proof = dummy_proof.clone();
        proof.public_inputs[0..public_inputs.len()].copy_from_slice(public_inputs);
        let pis_len = proof.public_inputs.len();
        // The circuit checks that the verifier data is the same throughout the cycle, so
        // we set the verifier data to the "real" verifier data even though it's unused in the base case.
        let num_cap = cyclic_recursion_data
            .common_data
            .config
            .fri_config
            .num_cap_elements();
        let s = pis_len - 4 - 4 * num_cap;
        proof.public_inputs[s..s + 4]
            .copy_from_slice(&cyclic_recursion_data.verifier_data.circuit_digest.elements);
        for i in 0..num_cap {
            proof.public_inputs[s + 4 * (1 + i)..s + 4 * (2 + i)].copy_from_slice(
                &cyclic_recursion_data.verifier_data.constants_sigmas_cap.0[i].elements,
            );
        }

        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.proof, &proof);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.verifier_data,
            cyclic_recursion_data.verifier_data,
        );
        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.dummy_proof, &dummy_proof);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.dummy_verifier_data,
            &dummy_data,
        );
    }

    Ok(())
}

/// Additional checks to be performed on a cyclic recursive proof in addition to verifying the proof.
/// Checks that the `base_case` flag is boolean and that the purported verifier data in the public inputs
/// match the real verifier data.
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
    use plonky2_field::extension::Extendable;
    use plonky2_field::types::PrimeField64;

    use crate::field::types::Field;
    use crate::gates::noop::NoopGate;
    use crate::hash::hash_types::RichField;
    use crate::hash::hashing::hash_n_to_hash_no_pad;
    use crate::hash::poseidon::{PoseidonHash, PoseidonPermutation};
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData, VerifierCircuitTarget};
    use crate::plonk::config::{AlgebraicHasher, GenericConfig, Hasher, PoseidonGoldilocksConfig};
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
        [(); C::Hasher::HASH_SIZE]:,
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
        builder.verify_proof::<C>(proof, &verifier_data, &data.common);
        let data = builder.build::<C>();

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let proof = builder.add_virtual_proof_with_pis::<C>(&data.common);
        let verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: builder.add_virtual_cap(data.common.config.fri_config.cap_height),
            circuit_digest: builder.add_virtual_hash(),
        };
        builder.verify_proof::<C>(proof, &verifier_data, &data.common);
        while builder.num_gates() < 1 << 12 {
            builder.add_gate(NoopGate, vec![]);
        }
        builder.build::<C>().common
    }

    #[test]
    fn test_cyclic_recursion() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();
        let mut pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        // Circuit that computes a repeated hash.
        let initial_hash = builder.add_virtual_hash();
        builder.register_public_inputs(&initial_hash.elements);
        // Hash from the previous proof.
        let old_hash = builder.add_virtual_hash();
        // The input hash is either the previous hash or the initial hash depending on whether
        // the last proof was a base case.
        let input_hash = builder.add_virtual_hash();
        let h = builder.hash_n_to_hash_no_pad::<PoseidonHash>(input_hash.elements.to_vec());
        builder.register_public_inputs(&h.elements);
        // Previous counter.
        let old_counter = builder.add_virtual_target();
        let one = builder.one();
        let new_counter = builder.add_virtual_public_input();
        let old_pis = [
            initial_hash.elements.as_slice(),
            old_hash.elements.as_slice(),
            [old_counter].as_slice(),
        ]
        .concat();

        let mut common_data = common_data_for_recursion::<F, C, D>();

        let base_case = builder.add_virtual_bool_target_safe();
        // Add cyclic recursion gadget.
        let cyclic_data_target =
            builder.cyclic_recursion::<C>(base_case, &old_pis, &mut common_data)?;
        let input_hash_bis =
            builder.select_hash(cyclic_data_target.base_case, initial_hash, old_hash);
        builder.connect_hashes(input_hash, input_hash_bis);
        let not_base_case = builder.sub(one, cyclic_data_target.base_case.target);
        // New counter is the previous counter +1 if the previous proof wasn't a base case.
        let new_counter_bis = builder.add(old_counter, not_base_case);
        builder.connect(new_counter, new_counter_bis);

        let cyclic_circuit_data = builder.build::<C>();

        let cyclic_recursion_data = CyclicRecursionData {
            proof: &None, // Base case: We don't have a proof to put here yet.
            verifier_data: &cyclic_circuit_data.verifier_only,
            common_data: &cyclic_circuit_data.common,
        };
        let initial_hash = [F::ZERO, F::ONE, F::TWO, F::from_canonical_usize(3)];
        set_cyclic_recursion_data_target(
            &mut pw,
            &cyclic_data_target,
            &cyclic_recursion_data,
            &initial_hash,
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
            &[],
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
            &[],
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
        let mut h: [F; 4] = initial_hash.try_into().unwrap();
        assert_eq!(
            hash,
            std::iter::repeat_with(|| {
                h = hash_n_to_hash_no_pad::<F, PoseidonPermutation>(&h).elements;
                h
            })
            .nth(counter.to_canonical_u64() as usize)
            .unwrap()
        );

        cyclic_circuit_data.verify(proof)
    }
}
