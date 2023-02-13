//!
//! This module mainly provides the `AggregationScheme` utilities, which implements the interfaces
//! exposed externally to the crate to recursively aggregate an unlimited number of proofs in a
//! single proof.
//!

use std::collections::HashMap;
use std::ops::Deref;

use anyhow::{Error, Result};
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_data::{CircuitConfig, VerifierCircuitData};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, GenericHashOut, Hasher};
use plonky2::plonk::proof::ProofWithPublicInputs;
use plonky2::field::extension::Extendable;

use crate::public_input_aggregation::PublicInputAggregation;
use crate::recursion::merge_circuit::MergeCircuit;
use crate::recursion::wrap_circuit::WrapCircuitForBaseProofs;
use crate::recursion::{
    BaseCircuitInfo, PreparedProof, RecursionCircuit, VerifierOnlyCircuitDataWrapper,
};

#[derive(Clone)]
/// `PreparedProofForAggregation` represents the `PreparedProof`s employed in `AggregationScheme`.
/// It is constructed from a base proof by calling the method `prepare_proof_for_aggregation` of
/// `AggregationScheme`
pub struct PreparedProofForAggregation<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
> {
    proof: ProofWithPublicInputs<F, C, D>,
    circuit_data: VerifierOnlyCircuitDataWrapper<C, D>,
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    PreparedProof<F, C, D> for PreparedProofForAggregation<F, C, D>
{
    fn get_proof(&self) -> &ProofWithPublicInputs<F, C, D> {
        &self.proof
    }
}

/// `AggregationScheme` allows to aggregate several base proofs generated from a circuit
/// belong to a given set of circuits; the public inputs of the proofs are aggregated employing
/// the strategy specified by the `PublicInputAggregation` scheme `PI`
pub struct AggregationScheme<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
    PI: PublicInputAggregation,
> {
    wrap_circuits: HashMap<Vec<F>, WrapCircuitForBaseProofs<F, C, D>>,
    merge_circuit: MergeCircuit<F, C, D, PI>,
    aggregation_factor: usize,
    // buffer employed to store the set of prepared proofs to be aggregated, that are the proofs
    // provided by the user with `add_proofs_for_aggregation` function
    to_be_aggregated_proofs: Vec<PreparedProofForAggregation<F, C, D>>,
}

impl<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + 'static,
        const D: usize,
        PI: PublicInputAggregation,
    > RecursionCircuit<F, C, D, PI> for AggregationScheme<F, C, D, PI>
where
    C::Hasher: AlgebraicHasher<F>,
    [(); C::Hasher::HASH_SIZE]:,
{
    type PreparedProof = PreparedProofForAggregation<F, C, D>;

    fn build_circuit<'a>(
        circuit_set: impl Iterator<Item = Box<dyn BaseCircuitInfo<F, C, D, PIScheme = PI> + 'a>>,
    ) -> Result<Self> {
        Self::build_circuit_with_custom_aggregation_factor(circuit_set, 2)
    }

    fn build_circuit_with_custom_aggregation_factor<'a>(
        circuit_set: impl Iterator<Item = Box<dyn BaseCircuitInfo<F, C, D, PIScheme = PI> + 'a>>,
        aggregation_factor: usize,
    ) -> Result<Self> {
        let config = CircuitConfig::standard_recursion_config();
        let (circuit_digests, wrap_circuits) = circuit_set.map(|ci| {
            let ci = ci.deref();
            let verifier_data = ci.get_verifier_circuit_data();
            let wrap_circuit = WrapCircuitForBaseProofs::build_wrap_circuit(
                &verifier_data.verifier_only,
                &verifier_data.common,
                &config,
            );
            (
                wrap_circuit.final_proof_circuit_data().verifier_only.circuit_digest,
                (verifier_data.verifier_only.circuit_digest.to_vec(), wrap_circuit),
            )
        }).unzip::<_,_,Vec<<C::Hasher as Hasher<F>>::Hash>,HashMap<Vec<F>, WrapCircuitForBaseProofs<F,C,D>>>();
        let merge_circuit =
            MergeCircuit::build_merge_circuit(config.clone(), aggregation_factor, circuit_digests)?;

        Ok(
            AggregationScheme {
                wrap_circuits,
                merge_circuit,
                aggregation_factor,
                to_be_aggregated_proofs: vec![],
            }
        )
    }

    fn prepare_proof_for_aggregation(
        &self,
        proof: ProofWithPublicInputs<F, C, D>,
        circuit_data: &VerifierCircuitData<F, C, D>,
    ) -> Result<Self::PreparedProof> {
        let wrap_circuit = self
            .wrap_circuits
            .get(
                circuit_data
                    .verifier_only
                    .circuit_digest
                    .to_vec()
                    .as_slice(),
            )
            .ok_or(Error::msg("invalid circuit data"))?;
        let wrapped_proof =
            wrap_circuit.wrap_proof(proof, self.merge_circuit.get_circuit_set_digest())?;

        let wrap_circuit_vd = VerifierOnlyCircuitDataWrapper::from(
            &wrap_circuit.final_proof_circuit_data().verifier_only,
        );

        Ok(PreparedProofForAggregation {
            proof: wrapped_proof,
            circuit_data: wrap_circuit_vd,
        })
    }

    fn add_proofs_for_aggregation(
        mut self,
        prepared_proofs: impl IntoIterator<Item = Self::PreparedProof>,
    ) -> Self {
        for proof in prepared_proofs {
            self.to_be_aggregated_proofs.push(proof);
        }
        self
    }

    fn aggregate_proofs_with(
        mut self,
        prepared_proofs: impl IntoIterator<Item = Self::PreparedProof>,
    ) -> Result<(Self, Self::PreparedProof)> {
        let circuit_data = VerifierOnlyCircuitDataWrapper::from(
            &self
                .merge_circuit
                .aggregated_proof_circuit_data()
                .verifier_only,
        );
        let (mut to_be_aggregated_proofs, mut vds): (Vec<_>, Vec<_>) = self
            .to_be_aggregated_proofs
            .into_iter()
            .chain(prepared_proofs)
            .map(|proof| (proof.proof, proof.circuit_data.0))
            .unzip();

        if to_be_aggregated_proofs.len() < 2 {
            return Err(Error::msg("no proofs to be aggregated"));
        }

        while to_be_aggregated_proofs.len() >= self.aggregation_factor {
            let mut proofs_iter = to_be_aggregated_proofs.chunks_exact(self.aggregation_factor);
            let mut vds_iter = vds.chunks_exact(self.aggregation_factor);
            let mut aggregated_proofs = (&mut proofs_iter)
                .zip(&mut vds_iter)
                .map(|(proof_chunk, vd_chunk)| {
                    self.merge_circuit
                        .merge_proofs(proof_chunk, vd_chunk.iter())
                })
                .collect::<Result<Vec<_>>>()?;
            let mut aggregated_vds = (0..aggregated_proofs.len())
                .map(|_| circuit_data.clone().0)
                .collect::<Vec<_>>();
            aggregated_proofs.extend_from_slice(proofs_iter.remainder());
            aggregated_vds.append(
                &mut vds_iter
                    .remainder()
                    .iter()
                    .map(|vd| VerifierOnlyCircuitDataWrapper::from(vd).0)
                    .collect::<Vec<_>>(),
            );
            to_be_aggregated_proofs = aggregated_proofs;
            vds = aggregated_vds;
        }

        let aggregated_proof = if to_be_aggregated_proofs.len() != 1 {
            self.merge_circuit
                .merge_proofs(to_be_aggregated_proofs.as_slice(), vds.iter())?
        } else {
            to_be_aggregated_proofs.pop().unwrap()
        };

        self.to_be_aggregated_proofs = vec![];
        Ok((
            self,
            PreparedProofForAggregation {
                proof: aggregated_proof,
                circuit_data,
            },
        ))
    }

    fn verify_aggregated_proof(&self, prepared_proof: Self::PreparedProof) -> Result<()> {
        let aggregated_proof = prepared_proof.get_proof();
        // check that the public inputs corresponding to the circuit set digest corresponds to the
        // expected set of circuits
        assert_eq!(
            aggregated_proof.public_inputs[PI::num_public_inputs()..]
                .to_vec(),
            self.merge_circuit
                .get_circuit_set_digest()
                .flatten(),
        );
        // verify the proof
        self.merge_circuit
            .aggregated_proof_circuit_data()
            .verify(prepared_proof.get_proof().clone())
    }
}

#[cfg(test)]
mod tests {
    use plonky2::hash::hash_types::RichField;
    use plonky2::hash::merkle_tree::MerkleTree;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{
        AlgebraicHasher, GenericConfig, Hasher, PoseidonGoldilocksConfig,
    };
    use plonky2::field::extension::Extendable;
    use plonky2::field::types::Sample;
    use rand::{thread_rng, Rng};
    use rstest::rstest;
    use serial_test::serial;

    use crate::public_input_aggregation::shared_state::{
        MerkleRootState, SharedStatePublicInput, SimpleState,
        State,
    };
    use crate::public_input_aggregation::PublicInputAggregation;
    use crate::recursion::recursive_circuit::{AggregationScheme, PreparedProofForAggregation};
    use crate::recursion::test_circuits::{
        logger, ExpBaseCircuit, MerkleRootStateBaseCircuit, MulBaseCircuit,
    };
    use crate::recursion::{BaseCircuitInfo, PreparedProof, RECURSION_THRESHOLD, RecursionCircuit, prepare_base_circuit_for_circuit_set};

    const D: usize = 2;
    type PC = PoseidonGoldilocksConfig;
    type F = <PC as GenericConfig<D>>::F;

    // multiple checks to ensure the validity of an aggregated proof for a
    // `SharedStatePublicInput` aggregation scheme employing `ST` as state representation
    fn check_aggregated_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + 'static,
        const D: usize,
        ST: State,
    >(
        aggregated_proof: &PreparedProofForAggregation<F, C, D>,
        aggregation_scheme: &AggregationScheme<F, C, D, SharedStatePublicInput<ST>>,
        init: Vec<F>,
        final_state: Vec<F>,
    ) where
        C::Hasher: AlgebraicHasher<F>,
        [(); C::Hasher::HASH_SIZE]:,
    {
        aggregation_scheme
            .verify_aggregated_proof(aggregated_proof.clone())
            .unwrap();
        let aggregated_proof = aggregated_proof.get_proof();

        assert_eq!(aggregated_proof.public_inputs[..ST::num_targets()], init);
        assert_eq!(
            aggregated_proof.public_inputs
                [ST::num_targets()..SharedStatePublicInput::<ST>::num_public_inputs()],
            final_state
        );
    }

    #[rstest]
    #[case::ok(false, 10, 4, true)]
    #[case::ok_zk(true, 10, 4, true)]
    #[case::pad(false, 8, 4, true)]
    #[case::less_than_aggregation_factor(false, 2, 4, true)]
    #[case::aggregate_circuits_with_different_size(false, 6, 2, false)]
    #[case::aggregation_factor_not_power_of_2(false, 8, 6, true)]
    #[should_panic(expected = "meaningless to merge less than 2 proofs, provide a higher number of proofs to merge")]
    #[case::aggregation_factor_less_than_2(false, 4, 1, true)]
    #[serial]
    fn test_recursive_circuit(
        #[case] zk: bool,
        #[case] num_proofs: usize,
        #[case] aggregation_factor: usize,
        #[case] aggregate_circuits_with_same_size: bool,
        _logger: (),
    ) {
        let config = if zk {
            CircuitConfig::standard_recursion_config()
        } else {
            CircuitConfig::standard_recursion_zk_config()
        };
        let mut rng = thread_rng();
        let (exp_circuit_degree, mul_circuit_degree) = if aggregate_circuits_with_same_size {
            (RECURSION_THRESHOLD, RECURSION_THRESHOLD)
        } else {
            (
                RECURSION_THRESHOLD+1,
                RECURSION_THRESHOLD+2,
            )
        };
        let exp_base_circuit =
            ExpBaseCircuit::<F, PC, D>::build_base_circuit(&config, exp_circuit_degree);
        let mul_base_circuit =
            MulBaseCircuit::<F, PC, D>::build_base_circuit(&config, mul_circuit_degree, false);
        println!("base circuit built");
        let exp_vd = (&exp_base_circuit).get_verifier_circuit_data();
        let mul_vd = (&mul_base_circuit).get_verifier_circuit_data();

        let init = F::rand();
        let mut state = init;
        // generate base proofs interleaving the 2 base circuits
        let base_proofs = (0..num_proofs)
            .map(|i| {
                let (proof, vd) = if rng.gen() {
                    (
                        mul_base_circuit.generate_base_proof(state).unwrap(),
                        &mul_vd,
                    )
                } else {
                    (
                        exp_base_circuit.generate_base_proof(state).unwrap(),
                        &exp_vd,
                    )
                };
                println!("generated {}-th base proof", i + 1);
                state = proof.public_inputs[1];
                (proof, vd)
            })
            .collect::<Vec<_>>();

        let final_output = state;

        let circuit_set= vec![prepare_base_circuit_for_circuit_set(&exp_base_circuit),
                              prepare_base_circuit_for_circuit_set(&mul_base_circuit)];

        let mut aggregation_scheme =
            AggregationScheme::build_circuit_with_custom_aggregation_factor(
                circuit_set.into_iter(),
                aggregation_factor,
            ).unwrap();

        for (proof, vd) in base_proofs.into_iter() {
            let prepared_proof = aggregation_scheme.prepare_proof_for_aggregation(proof, vd);
            aggregation_scheme = aggregation_scheme.add_proofs_for_aggregation(prepared_proof);
        }

        let (aggregation_scheme, aggregated_proof) = aggregation_scheme.aggregate_proofs().unwrap();

        check_aggregated_proof::<_, _, D, SimpleState>(
            &aggregated_proof,
            &aggregation_scheme,
            vec![init],
            vec![final_output],
        );
    }

    #[rstest]
    #[serial]
    fn test_recursive_circuit_with_merkle_root_base_circuit(_logger: ()) {
        const CAP_HEIGHT: usize = 0;

        let config = CircuitConfig::standard_recursion_config();
        let num_leaves = 1 << 12;
        let base_circuit =
            MerkleRootStateBaseCircuit::<F, PC, D, CAP_HEIGHT>::build_circuit(&config, num_leaves);
        let verifier_data = (&base_circuit).get_verifier_circuit_data();

        let mut rng = thread_rng();

        let leaves = (0..num_leaves).map(|_| vec![F::rand()]).collect::<Vec<_>>();

        let mut mt = MerkleTree::<F, <PC as GenericConfig<D>>::Hasher>::new(leaves, CAP_HEIGHT);
        let initial_state = mt.cap.clone();
        let num_proofs = 4;
        let base_proofs = (0..num_proofs)
            .map(|_| {
                base_circuit.generate_base_proof(
                    &mut mt,
                    rng.gen_range(0..num_leaves),
                    rng.gen_range(0..4),
                )
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .unwrap();

        let circuit_set = vec![prepare_base_circuit_for_circuit_set(&base_circuit)];

        let mut aggregation_scheme =
            AggregationScheme::build_circuit(
                circuit_set.into_iter(),
            ).unwrap();

        for proof in base_proofs {
            let prepared_proof =
                aggregation_scheme.prepare_proof_for_aggregation(proof, &verifier_data);
            aggregation_scheme = aggregation_scheme.add_proofs_for_aggregation(prepared_proof);
        }

        // add a further proof
        let base_proof = base_circuit
            .generate_base_proof(&mut mt, rng.gen_range(0..num_leaves), rng.gen_range(0..4))
            .unwrap();
        let final_state = mt.cap.clone();
        let prepared_proof =
            aggregation_scheme.prepare_proof_for_aggregation(base_proof, &verifier_data);

        let (aggregation_scheme, aggregated_proof) = aggregation_scheme
            .aggregate_proofs_with(prepared_proof)
            .unwrap();

        check_aggregated_proof::<_, _, D, MerkleRootState<CAP_HEIGHT>>(
            &aggregated_proof,
            &aggregation_scheme,
            initial_state.flatten(),
            final_state.flatten(),
        );

        // verify that the proof computed by the aggregation scheme can be further aggregated with other proofs
        let base_proof = base_circuit
            .generate_base_proof(&mut mt, rng.gen_range(0..num_leaves), rng.gen_range(0..4))
            .unwrap();
        let final_state = mt.cap.clone();
        let prepared_proof = aggregation_scheme
            .prepare_proof_for_aggregation(base_proof, &verifier_data)
            .unwrap();

        let (aggregation_scheme, aggregated_proof) = aggregation_scheme
            .aggregate_proofs_with(vec![aggregated_proof, prepared_proof].into_iter())
            .unwrap();
        check_aggregated_proof::<_, _, D, MerkleRootState<CAP_HEIGHT>>(
            &aggregated_proof,
            &aggregation_scheme,
            initial_state.flatten(),
            final_state.flatten(),
        );
    }
}
