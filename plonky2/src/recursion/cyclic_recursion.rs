use anyhow::{ensure, Result};
use itertools::Itertools;
use plonky2_field::extension::Extendable;
use plonky2_field::types::Field;

use crate::gates::noop::NoopGate;
use crate::hash::hash_types::{HashOut, HashOutTarget, MerkleCapTarget, RichField};
use crate::hash::merkle_tree::MerkleCap;
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartialWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::{
    CircuitData, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
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
    common_data: &'a CommonCircuitData<F, C, D>,
}

pub struct CyclicRecursionTarget<const D: usize> {
    pub proof: ProofWithPublicInputsTarget<D>,
    pub verifier_data: VerifierCircuitTarget,
    pub dummy_proof: ProofWithPublicInputsTarget<D>,
    pub dummy_verifier_data: VerifierCircuitTarget,
    pub base_case: BoolTarget,
}

pub struct CyclicPublicInputs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    pub circuit_digest: HashOut<F>,
    pub constants_sigmas_cap: MerkleCap<F, C::Hasher>,
    pub base_case: bool,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    CyclicPublicInputs<F, C, D>
{
    fn from_slice(slice: &[F], common_data: &CommonCircuitData<F, C, D>) -> Result<Self>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        // The structure of the public inputs is `[...,circuit_digest, constants_sigmas_cap, base_case]`.
        let cap_len = common_data.config.fri_config.num_cap_elements();
        let len = slice.len();
        ensure!(len >= 4 + 4 * cap_len + 1, "Not enough public inputs");
        let base_case = slice[len - 1];
        ensure!(
            base_case.is_one() || base_case.is_zero(),
            "Base case flag {:?} is not binary",
            base_case
        );
        let constants_sigmas_cap = MerkleCap(
            (0..cap_len)
                .map(|i| HashOut {
                    elements: std::array::from_fn(|j| slice[len - 1 - 4 * (cap_len - i) + j]),
                })
                .collect(),
        );
        let circuit_digest =
            HashOut::from_partial(&slice[len - 5 - 4 * cap_len..len - 1 - 4 * cap_len]);

        Ok(Self {
            circuit_digest,
            constants_sigmas_cap,
            base_case: base_case.is_one(),
        })
    }
}

pub struct CyclicPublicInputsTarget {
    pub circuit_digest: HashOutTarget,
    pub constants_sigmas_cap: MerkleCapTarget,
    pub base_case: Target,
}

impl CyclicPublicInputsTarget {
    fn from_slice<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
        slice: &[Target],
        common_data: &CommonCircuitData<F, C, D>,
    ) -> Result<Self> {
        let cap_len = common_data.config.fri_config.num_cap_elements();
        let len = slice.len();
        ensure!(len >= 4 + 4 * cap_len + 1, "Not enough public inputs");
        let base_case = slice[len - 1];
        let constants_sigmas_cap = MerkleCapTarget(
            (0..cap_len)
                .map(|i| HashOutTarget {
                    elements: std::array::from_fn(|j| slice[len - 1 - 4 * (cap_len - i) + j]),
                })
                .collect(),
        );
        let circuit_digest = HashOutTarget {
            elements: std::array::from_fn(|i| slice[len - 5 - 4 * cap_len + i]),
        };

        Ok(Self {
            circuit_digest,
            constants_sigmas_cap,
            base_case,
        })
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn cyclic_recursion<C: GenericConfig<D, F = F>>(
        mut self,
        mut common_data: CommonCircuitData<F, C, D>,
    ) -> Result<(CircuitData<F, C, D>, CyclicRecursionTarget<D>)>
    where
        C::Hasher: AlgebraicHasher<F>,
        [(); C::Hasher::HASH_SIZE]:,
    {
        let verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: self.add_virtual_cap(self.config.fri_config.cap_height),
            circuit_digest: self.add_virtual_hash(),
        };
        // The verifier data are public inputs.
        self.register_public_inputs(&verifier_data.circuit_digest.elements);
        for i in 0..self.config.fri_config.num_cap_elements() {
            self.register_public_inputs(&verifier_data.constants_sigmas_cap.0[i].elements);
        }

        let dummy_verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: self.add_virtual_cap(self.config.fri_config.cap_height),
            circuit_digest: self.add_virtual_hash(),
        };
        // Unsafe is ok since `base_case` is a public input and its booleaness should be checked in the verifier.
        let base_case = self.add_virtual_bool_target_unsafe();
        self.register_public_input(base_case.target);

        common_data.num_public_inputs = self.num_public_inputs();
        // The `conditionally_verify_proof` gadget below takes 2^12 gates, so `degree_bits` cannot be smaller than 13.
        common_data.degree_bits = common_data.degree_bits.max(13);
        common_data.fri_params.degree_bits = common_data.fri_params.degree_bits.max(13);

        let proof = self.add_virtual_proof_with_pis(&common_data);
        let dummy_proof = self.add_virtual_proof_with_pis(&common_data);

        let pis = CyclicPublicInputsTarget::from_slice(&proof.public_inputs, &common_data)?;
        self.connect_hashes(pis.circuit_digest, verifier_data.circuit_digest);
        for (h0, h1) in pis
            .constants_sigmas_cap
            .0
            .iter()
            .zip_eq(&verifier_data.constants_sigmas_cap.0)
        {
            self.connect_hashes(*h0, *h1);
        }

        self.conditionally_verify_proof(
            base_case,
            &dummy_proof,
            &dummy_verifier_data,
            &proof,
            &verifier_data,
            &common_data,
        );

        while self.num_gates() < 1 << (common_data.degree_bits - 1) {
            self.add_gate(NoopGate, vec![]);
        }
        for g in &common_data.gates {
            self.add_gate_to_gate_set(g.clone());
        }

        let data = self.build::<C>();
        assert_eq!(&data.common, &common_data);

        Ok((
            data,
            CyclicRecursionTarget {
                proof,
                verifier_data,
                dummy_proof,
                dummy_verifier_data,
                base_case,
            },
        ))
    }
}

/// Set the targets in a `ProofTarget` to their corresponding values in a `Proof`.
pub fn set_cyclic_recursion_data_target<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    pw: &mut PartialWitness<F>,
    cyclic_recursion_data_target: &CyclicRecursionTarget<D>,
    cyclic_recursion_data: &CyclicRecursionData<F, C, D>,
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
        dbg!("hi");
        let (dummy_proof, dummy_data) = dummy_proof(cyclic_recursion_data.common_data)?;
        pw.set_bool_target(cyclic_recursion_data_target.base_case, true);
        let mut dummy_proof_real_vd = dummy_proof.clone();
        let pis_len = dummy_proof_real_vd.public_inputs.len();
        let num_cap = cyclic_recursion_data
            .common_data
            .config
            .fri_config
            .num_cap_elements();
        let s = pis_len - 5 - 4 * num_cap;
        dummy_proof_real_vd.public_inputs[s..s + 4]
            .copy_from_slice(&cyclic_recursion_data.verifier_data.circuit_digest.elements);
        for i in 0..num_cap {
            dummy_proof_real_vd.public_inputs[s + 4 * (1 + i)..s + 4 * (2 + i)].copy_from_slice(
                &cyclic_recursion_data.verifier_data.constants_sigmas_cap.0[i].elements,
            );
        }
        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.proof, &dummy_proof_real_vd);
        dbg!(cyclic_recursion_data.verifier_data.circuit_digest);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.verifier_data,
            cyclic_recursion_data.verifier_data,
        );
        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.dummy_proof, &dummy_proof);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.dummy_verifier_data,
            &dummy_data.verifier_only,
        );
    }

    Ok(())
}

pub fn check_cyclic_proof_verifier_data<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    proof: &ProofWithPublicInputs<F, C, D>,
    verifier_data: &VerifierOnlyCircuitData<C, D>,
    common_data: &CommonCircuitData<F, C, D>,
) -> Result<()>
where
    C::Hasher: AlgebraicHasher<F>,
{
    let pis = CyclicPublicInputs::from_slice(&proof.public_inputs, common_data)?;
    dbg!(pis.circuit_digest);
    dbg!(verifier_data.circuit_digest);
    if !pis.base_case {
        ensure!(verifier_data.constants_sigmas_cap == pis.constants_sigmas_cap);
        ensure!(verifier_data.circuit_digest == pis.circuit_digest);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::extension::Extendable;

    use crate::field::types::Field;
    use crate::hash::hash_types::RichField;
    use crate::hash::poseidon::PoseidonHash;
    use crate::iop::witness::{PartialWitness, Witness};
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::{CircuitConfig, CommonCircuitData, VerifierCircuitTarget};
    use crate::plonk::config::{AlgebraicHasher, GenericConfig, Hasher, PoseidonGoldilocksConfig};
    use crate::recursion::cyclic_recursion::{
        check_cyclic_proof_verifier_data, set_cyclic_recursion_data_target, CyclicRecursionData,
    };

    fn common_data_for_recursion<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >() -> CommonCircuitData<F, C, D>
    where
        C::Hasher: AlgebraicHasher<F>,
        [(); C::Hasher::HASH_SIZE]:,
    {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let data = builder.build::<C>();
        let config = CircuitConfig::standard_recursion_config();
        let mut pw = PartialWitness::<F>::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let proof = builder.add_virtual_proof_with_pis(&data.common);
        let verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: builder.add_virtual_cap(data.common.config.fri_config.cap_height),
            circuit_digest: builder.add_virtual_hash(),
        };
        builder.verify_proof(proof, &verifier_data, &data.common);
        let data = builder.build::<C>();

        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::<F>::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        let proof = builder.add_virtual_proof_with_pis(&data.common);
        let verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: builder.add_virtual_cap(data.common.config.fri_config.cap_height),
            circuit_digest: builder.add_virtual_hash(),
        };
        builder.verify_proof(proof, &verifier_data, &data.common);
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

        // Build realistic circuit
        let t = builder.add_virtual_target();
        pw.set_target(t, F::rand());
        let t_inv = builder.inverse(t);
        let h = builder.hash_n_to_hash_no_pad::<PoseidonHash>(vec![t_inv]);
        builder.register_public_inputs(&h.elements);

        let common_data = common_data_for_recursion::<F, C, D>();

        let (cyclic_circuit_data, cyclic_data_target) = builder.cyclic_recursion(common_data)?;
        let cyclic_recursion_data = CyclicRecursionData {
            proof: &None,
            verifier_data: &cyclic_circuit_data.verifier_only,
            common_data: &cyclic_circuit_data.common,
        };
        set_cyclic_recursion_data_target(&mut pw, &cyclic_data_target, &cyclic_recursion_data)?;
        dbg!("yo");
        let proof = cyclic_circuit_data.prove(pw)?;
        check_cyclic_proof_verifier_data(
            &proof,
            &cyclic_recursion_data.verifier_data,
            cyclic_recursion_data.common_data,
        )?;
        cyclic_circuit_data.verify(proof.clone())?;

        let mut pw = PartialWitness::new();
        pw.set_target(t, F::rand());
        let cyclic_recursion_data = CyclicRecursionData {
            proof: &Some(proof),
            verifier_data: &cyclic_circuit_data.verifier_only,
            common_data: &cyclic_circuit_data.common,
        };
        set_cyclic_recursion_data_target(&mut pw, &cyclic_data_target, &cyclic_recursion_data)?;
        dbg!("yo");
        let proof = cyclic_circuit_data.prove(pw)?;
        check_cyclic_proof_verifier_data(
            &proof,
            &cyclic_recursion_data.verifier_data,
            cyclic_recursion_data.common_data,
        )?;
        cyclic_circuit_data.verify(proof.clone())?;

        let mut pw = PartialWitness::new();
        pw.set_target(t, F::rand());
        let cyclic_recursion_data = CyclicRecursionData {
            proof: &Some(proof),
            verifier_data: &cyclic_circuit_data.verifier_only,
            common_data: &cyclic_circuit_data.common,
        };
        set_cyclic_recursion_data_target(&mut pw, &cyclic_data_target, &cyclic_recursion_data)?;
        let proof = cyclic_circuit_data.prove(pw)?;
        check_cyclic_proof_verifier_data(
            &proof,
            &cyclic_recursion_data.verifier_data,
            cyclic_recursion_data.common_data,
        )?;
        cyclic_circuit_data.verify(proof.clone())?;

        Ok(())
    }
}
