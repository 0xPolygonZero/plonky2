use anyhow::Result;
use plonky2_field::extension::Extendable;

use crate::gates::noop::NoopGate;
use crate::hash::hash_types::RichField;
use crate::iop::target::BoolTarget;
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
        self.register_public_inputs(&verifier_data.circuit_digest.elements);
        for i in 0..self.config.fri_config.num_cap_elements() {
            self.register_public_inputs(&verifier_data.constants_sigmas_cap.0[i].elements);
        }
        let dummy_verifier_data = VerifierCircuitTarget {
            constants_sigmas_cap: self.add_virtual_cap(self.config.fri_config.cap_height),
            circuit_digest: self.add_virtual_hash(),
        };
        let base_case = self.add_virtual_bool_target();
        self.register_public_input(base_case.target);

        common_data.num_public_inputs = self.num_public_inputs();
        common_data.degree_bits = common_data.degree_bits.max(13);
        common_data.fri_params.degree_bits = common_data.fri_params.degree_bits.max(13);

        let proof = self.add_virtual_proof_with_pis(&common_data);
        let dummy_proof = self.add_virtual_proof_with_pis(&common_data);

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
    F: RichField + Extendable<D>,
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
        let (dummy_proof, dummy_data) = dummy_proof(cyclic_recursion_data.common_data)?;
        pw.set_bool_target(cyclic_recursion_data_target.base_case, true);
        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.proof, &dummy_proof);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.verifier_data,
            &dummy_data.verifier_only,
        );
        pw.set_proof_with_pis_target(&cyclic_recursion_data_target.dummy_proof, &dummy_proof);
        pw.set_verifier_data_target(
            &cyclic_recursion_data_target.dummy_verifier_data,
            &dummy_data.verifier_only,
        );
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
        set_cyclic_recursion_data_target, CyclicRecursionData,
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
        let mut pw = PartialWitness::<F>::new();
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
        let proof = cyclic_circuit_data.prove(pw)?;
        cyclic_circuit_data.verify(proof);

        Ok(())
    }
}
