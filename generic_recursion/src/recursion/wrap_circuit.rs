use anyhow::Result;
use plonky2::hash::hash_types::{MerkleCapTarget, RichField};
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, VerifierCircuitTarget, VerifierOnlyCircuitData,
};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
use plonky2::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use plonky2::field::extension::Extendable;

use crate::recursion::merge_circuit::{CircuitSetDigest, CircuitSetTarget};
use crate::recursion::util::check_circuit_digest_target;
use crate::recursion::RECURSION_THRESHOLD;

// Data structure with all input/output targets and the `CircuitData` for each circuit employed
// to recursively wrap a proof up to the recursion threshold. The data structure contains a set
// of targets and a `CircuitData` for each wrap step. This data structure is employed as a building
// block to construct 2 different types of wrap circuits:
// - The wrap circuit for base proofs, which needs to add to the set of public inputs of the wrapped
//      base proof a further public input that makes the public input interface of the wrapped
//      proofs compatible with the one of merged proofs
// - A generic wrap circuit that has the same public inputs of the wrapped proof
struct WrapCircuitInner<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> {
    proof_targets: Vec<ProofWithPublicInputsTarget<D>>,
    circuit_data: Vec<CircuitData<F, C, D>>,
    inner_data: Vec<VerifierCircuitTarget>,
    // this target is only necessary when the data structure is employed to wrap base proofs, as
    // it represents the additional public input added by the wrap circuit
    circuit_set_target: Option<CircuitSetTarget>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    WrapCircuitInner<F, C, D>
where
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    // build the wrap circuit for a proof enforcing the circuit with verifier data `inner_vd`
    // and `inner_cd`; if `add_public_input` is `true`, then the set of public inputs of the wrapped
    // proof is enriched with a further public input that makes the public input interface of the wrapped
    // proofs compatible with the one of merged proofs
    fn build_wrap_circuit(
        inner_vd: &VerifierOnlyCircuitData<C, D>,
        inner_cd: &CommonCircuitData<F, D>,
        config: &CircuitConfig,
        mut add_public_input: bool,
    ) -> Self {
        let mut vd = inner_vd;
        let mut cd = inner_cd;
        let mut wrap_circuit = Self {
            proof_targets: Vec::new(),
            circuit_data: Vec::new(),
            inner_data: Vec::new(),
            circuit_set_target: None,
        };

        loop {
            let mut builder = CircuitBuilder::<F, D>::new(config.clone());
            let wrap_step = wrap_circuit.circuit_data.len();
            let pt = builder.add_virtual_proof_with_pis::<C>(cd);
            let inner_data = VerifierCircuitTarget {
                constants_sigmas_cap:
                    // we allocate `constants_sigmas_cap` as constants only in the first wrapping step,
                    // as otherwise it is not possible to obtain a wrapping circuit which is as
                    // small as the recursion threshold
                    if wrap_step != 0 {
                        builder.add_virtual_cap(cd.config.fri_config.cap_height)
                    } else {
                        MerkleCapTarget(
                            vd.constants_sigmas_cap.0.iter().map(|hash|
                            builder.constant_hash(*hash)
                            ).collect::<Vec<_>>()
                        )
                    },
                // instead, `circuit_digest` is a constant for all the wrapping circuits
                circuit_digest: builder.constant_hash(vd.circuit_digest),
            };
            builder.verify_proof::<C>(&pt, &inner_data, cd);

            if wrap_step != 0 {
                // in wrapping circuits where the `constants_sigmas_cap` are allocated as private
                // inputs, their correctness is enforced by re-computing the circuit digest and
                // comparing it with the constant one hardcoded in the wrapping circuit at hand
                check_circuit_digest_target::<_, C, D>(&mut builder, &inner_data, cd.degree_bits());
            }

            for pi_t in pt.public_inputs.iter() {
                builder.register_public_input(pi_t.clone())
            }

            if add_public_input {
                // add a `MerkleCapTarget` as a public input representing the set of circuits that
                // can be merged with the `MergeCircuit`
                let pi_target = CircuitSetTarget::build_target(&mut builder);
                builder.register_public_inputs(pi_target.to_targets().as_slice());
                // we need to add the public input only for the first wrap step
                add_public_input = false;
                wrap_circuit.circuit_set_target = Some(pi_target);
            }

            let data = builder.build::<C>();

            wrap_circuit.proof_targets.push(pt);
            wrap_circuit.circuit_data.push(data);
            wrap_circuit.inner_data.push(inner_data);
            let circuit_data = wrap_circuit.circuit_data.last().unwrap();
            (cd, vd) = (&circuit_data.common, &circuit_data.verifier_only);

            log::debug!(
                "wrap step {} done. circuit size is {}",
                wrap_step + 1,
                cd.degree_bits()
            );
            if circuit_data.common.degree_bits() == RECURSION_THRESHOLD {
                break;
            }
        }

        wrap_circuit
    }

    // wrap a proof `inner_proof` enforcing the circuit with data `inner_cd` employing the wrap
    // circuit. The `circuit_set_digest` input is needed only when `self.circuit_set_target` is
    // employed, that is when a wrap circuit for base proofs is employed
    fn wrap_proof(
        &self,
        inner_proof: ProofWithPublicInputs<F, C, D>,
        circuit_set_digest: Option<CircuitSetDigest<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>> {
        let mut proof = inner_proof;
        let mut circuit_data: Option<&VerifierOnlyCircuitData<C, D>> = None;

        let mut circuit_set_public_input = self.circuit_set_target.is_some();

        for ((pt, cd), inner_data) in self
            .proof_targets
            .iter()
            .zip(self.circuit_data.iter())
            .zip(self.inner_data.iter())
        {
            let mut pw = PartialWitness::new();
            pw.set_proof_with_pis_target(pt, &proof);
            if let Some(vd) = circuit_data {
                // no need to set `constants_sigmas_cap` target in the first wrapping step, as they
                // are hardcoded as constant in the first wrapping circuit
                pw.set_cap_target(&inner_data.constants_sigmas_cap, &vd.constants_sigmas_cap);
            }

            if circuit_set_public_input {
                circuit_set_digest
                    .as_ref()
                    .unwrap()
                    .set_circuit_set_target(&mut pw, self.circuit_set_target.as_ref().unwrap());
                circuit_set_public_input = false;
            }
            proof = cd.prove(pw)?;
            circuit_data = Some(&cd.verifier_only);
        }

        Ok(proof)
    }
}

// Wrap circuit employed to wrap base proofs; such wrap circuit needs to add to the set of public
// inputs of the wrapped base proof a further public input that makes the public input interface
// of the wrapped proofs compatible with one of merged proofs
pub(crate) struct WrapCircuitForBaseProofs<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(WrapCircuitInner<F, C, D>);

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    WrapCircuitForBaseProofs<F, C, D>
where
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    pub(crate) fn build_wrap_circuit(
        inner_vd: &VerifierOnlyCircuitData<C, D>,
        inner_cd: &CommonCircuitData<F, D>,
        config: &CircuitConfig,
    ) -> Self {
        WrapCircuitForBaseProofs::<F, C, D>(WrapCircuitInner::build_wrap_circuit(
            inner_vd, inner_cd, config, true,
        ))
    }

    pub(crate) fn wrap_proof(
        &self,
        inner_proof: ProofWithPublicInputs<F, C, D>,
        circuit_set_digest: CircuitSetDigest<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>> {
        self.0.wrap_proof(inner_proof, Some(circuit_set_digest))
    }

    // Helper function that returns a pointer to the circuit data of the circuit for the last
    // wrap step
    pub(crate) fn final_proof_circuit_data(&self) -> &CircuitData<F, C, D> {
        self.0.circuit_data.last().unwrap()
    }
}

// Generic wrap circuit that simply copies the public inputs of the wrapped proof
pub(crate) struct WrapCircuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(WrapCircuitInner<F, C, D>);

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> WrapCircuit<F, C, D>
where
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    pub(crate) fn build_wrap_circuit(
        inner_vd: &VerifierOnlyCircuitData<C, D>,
        inner_cd: &CommonCircuitData<F, D>,
        config: &CircuitConfig,
    ) -> Self {
        WrapCircuit::<F, C, D>(WrapCircuitInner::build_wrap_circuit(
            inner_vd, inner_cd, config, false,
        ))
    }

    pub(crate) fn wrap_proof(
        &self,
        inner_proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>> {
        self.0.wrap_proof(inner_proof, None)
    }

    // Helper function that returns a pointer to the circuit data of the circuit for the last
    // wrap step
    pub(crate) fn final_proof_circuit_data(&self) -> &CircuitData<F, C, D> {
        self.0.circuit_data.last().unwrap()
    }
}

#[cfg(test)]
pub(crate) mod test {
    use plonky2::gates::noop::NoopGate;
    use plonky2::plonk::config::PoseidonGoldilocksConfig;
    use plonky2::field::types::Sample;
    use rstest::rstest;

    use super::*;
    use crate::recursion::test_circuits::{check_panic, logger, MulBaseCircuit};

    pub(crate) fn mutable_final_proof_circuit_data<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        const D: usize,
    >(
        circuit: &mut WrapCircuitForBaseProofs<F, C, D>,
    ) -> &mut CircuitData<F, C, D> {
        circuit.0.circuit_data.last_mut().unwrap()
    }

    #[rstest]
    fn test_wrap_circuit_keys(_logger: ()) {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();
        const DEGREE: usize = 14;
        let mul_circuit = MulBaseCircuit::<F, C, D>::build_base_circuit(&config, DEGREE, false);

        let proof = mul_circuit.generate_base_proof(F::rand()).unwrap();

        let wrap_circuit = WrapCircuitForBaseProofs::<F, C, D>::build_wrap_circuit(
            &mul_circuit.get_circuit_data().verifier_only,
            &mul_circuit.get_circuit_data().common,
            &config,
        );

        let mul_circuit_swap = MulBaseCircuit::<F, C, D>::build_base_circuit(&config, DEGREE, true);

        let proof_swap = mul_circuit_swap.generate_base_proof(F::rand()).unwrap();

        assert_eq!(
            mul_circuit_swap.get_circuit_data().common.degree_bits(),
            DEGREE
        );

        // generate random circuit digest
        let circuit_set_digest = CircuitSetDigest::default();

        let wrap_proof = wrap_circuit
            .wrap_proof(proof, circuit_set_digest.clone())
            .unwrap();

        wrap_circuit
            .final_proof_circuit_data()
            .verify(wrap_proof)
            .unwrap();

        let wrap_circuit_mul_swap = WrapCircuitForBaseProofs::<F, C, D>::build_wrap_circuit(
            &mul_circuit_swap.get_circuit_data().verifier_only,
            &mul_circuit_swap.get_circuit_data().common,
            &config,
        );

        let wrap_proof_swap = wrap_circuit_mul_swap
            .wrap_proof(proof_swap.clone(), circuit_set_digest.clone())
            .unwrap();

        wrap_circuit_mul_swap
            .final_proof_circuit_data()
            .verify(wrap_proof_swap)
            .unwrap();

        assert_ne!(
            mul_circuit.get_circuit_data().verifier_only,
            mul_circuit_swap.get_circuit_data().verifier_only
        );

        assert_ne!(
            wrap_circuit.final_proof_circuit_data().verifier_only,
            wrap_circuit_mul_swap
                .final_proof_circuit_data()
                .verifier_only
        );

        // check that wrapping a proof with the wrong wrapping circuit does not work
        check_panic!(
            || wrap_circuit
                .wrap_proof(proof_swap, circuit_set_digest,)
                .unwrap(),
            "wrapping proof with wrong circuit did not panic"
        );
    }

    #[rstest]
    fn test_wrapping_base_circuit_with_domain_separator(_logger: ()) {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig::standard_recursion_config();

        let mut builder = CircuitBuilder::<F, D>::new(config.clone());
        for _ in 0..=(1 << 12) {
            builder.add_gate(NoopGate, vec![]);
        }
        builder.set_domain_separator(vec![F::rand()]);
        let pi_t = builder.add_virtual_public_input();

        let data = builder.build::<C>();

        assert_eq!(data.common.degree_bits(), 13);

        let wrap_circuit = WrapCircuitForBaseProofs::build_wrap_circuit(
            &data.verifier_only,
            &data.common,
            &config,
        );

        let mut pw = PartialWitness::new();
        let public_input = F::rand();
        pw.set_target(pi_t, public_input);

        let proof = data.prove(pw).unwrap();

        let wrapped_proof = wrap_circuit
            .wrap_proof(proof, CircuitSetDigest::default())
            .unwrap();

        assert_eq!(wrapped_proof.public_inputs[0], public_input);

        wrap_circuit
            .final_proof_circuit_data()
            .verify(wrapped_proof)
            .unwrap()
    }
}
