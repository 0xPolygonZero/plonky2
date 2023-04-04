#![feature(generic_const_exprs)]

use std::marker::PhantomData;

use anyhow::Result;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use plonky2::field::extension::Extendable;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::hashing::HashConfig;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{
    CircuitConfig, CircuitData, CommonCircuitData, VerifierCircuitTarget,
};
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig};
use plonky2::plonk::proof::{ProofWithPublicInputs, ProofWithPublicInputsTarget};
use poseidon2_plonky2::poseidon2_goldilock::Poseidon2GoldilocksConfig;
use tynm::type_name;

use crate::circuits::BaseCircuit;

mod circuits;

macro_rules! pretty_print {
    ($($arg:tt)*) => {
        print!("\x1b[0;36mINFO ===========>\x1b[0m ");
        println!($($arg)*);
    }
}

/// Data structure with all input/output targets and the `CircuitData` for each circuit employed
/// to recursively shrink a proof up to the recursion threshold. The data structure contains a set
/// of targets and a `CircuitData` for each shrink step
struct ShrinkCircuit<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
    const D: usize,
> {
    proof_targets: Vec<ProofWithPublicInputsTarget<D>>,
    circuit_data: Vec<CircuitData<F, C, D>>,
    inner_data: Vec<VerifierCircuitTarget>,
    _inner_c: PhantomData<InnerC>,
}

const RECURSION_THRESHOLD: usize = 12;

impl<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        InnerC: GenericConfig<D, F = F>,
        const D: usize,
    > ShrinkCircuit<F, C, InnerC, D>
where
    InnerC::Hasher: AlgebraicHasher<F, InnerC::HCO>,
    [(); InnerC::HCO::WIDTH]:,
    [(); InnerC::HCI::WIDTH]:,
    C::Hasher: AlgebraicHasher<F, C::HCO>,
    [(); C::HCO::WIDTH]:,
    [(); C::HCI::WIDTH]:,
{
    pub fn build_shrink_circuit(inner_cd: &CommonCircuitData<F, D>, config: CircuitConfig) -> Self {
        let mut circuit_data = inner_cd;
        let mut shrink_circuit = Self {
            proof_targets: Vec::new(),
            circuit_data: Vec::new(),
            inner_data: Vec::new(),
            _inner_c: PhantomData::<InnerC>,
        };
        while circuit_data.degree_bits() > RECURSION_THRESHOLD {
            let mut builder = CircuitBuilder::<F, D>::new(config.clone());
            //let mut pw = PartialWitness::new();
            let pt = builder.add_virtual_proof_with_pis(circuit_data);

            let inner_data =
                builder.add_virtual_verifier_data(circuit_data.config.fri_config.cap_height);
            if shrink_circuit.num_shrink_steps() > 0 {
                builder.verify_proof::<C>(&pt, &inner_data, circuit_data);
            } else {
                builder.verify_proof::<InnerC>(&pt, &inner_data, circuit_data);
            }

            for &pi_t in pt.public_inputs.iter() {
                let t = builder.add_virtual_public_input();
                builder.connect(pi_t, t);
            }

            let data = builder.build::<C>();

            shrink_circuit.proof_targets.push(pt);
            shrink_circuit.circuit_data.push(data);
            shrink_circuit.inner_data.push(inner_data);
            circuit_data = &shrink_circuit.circuit_data.last().unwrap().common;
        }

        shrink_circuit
    }

    fn set_witness<GC: GenericConfig<D, F = F>>(
        pw: &mut PartialWitness<F>,
        proof: &ProofWithPublicInputs<F, GC, D>,
        pt: &ProofWithPublicInputsTarget<D>,
        inner_data: &VerifierCircuitTarget,
        circuit_data: &CircuitData<F, GC, D>,
    ) where
        GC::Hasher: AlgebraicHasher<F, GC::HCO>,
    {
        pw.set_proof_with_pis_target(pt, proof);
        pw.set_cap_target(
            &inner_data.constants_sigmas_cap,
            &circuit_data.verifier_only.constants_sigmas_cap,
        );
        pw.set_hash_target(
            inner_data.circuit_digest,
            circuit_data.verifier_only.circuit_digest,
        );
    }

    pub fn shrink_proof<'a>(
        &'a self,
        inner_proof: ProofWithPublicInputs<F, InnerC, D>,
        inner_cd: &'a CircuitData<F, InnerC, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>> {
        let mut proof = None;
        let mut circuit_data = None;

        for ((pt, cd), inner_data) in self
            .proof_targets
            .iter()
            .zip(self.circuit_data.iter())
            .zip(self.inner_data.iter())
        {
            let mut pw = PartialWitness::new();
            match (proof, circuit_data) {
                (None, None) => Self::set_witness(&mut pw, &inner_proof, pt, inner_data, inner_cd),
                (Some(inner_proof), Some(inner_cd)) => {
                    Self::set_witness(&mut pw, &inner_proof, pt, inner_data, inner_cd);
                }
                _ => unreachable!(),
            }
            proof = Some(cd.prove(pw)?);
            circuit_data = Some(cd);
        }

        Ok(proof.unwrap())
    }

    pub fn num_shrink_steps(&self) -> usize {
        self.circuit_data.len()
    }

    pub fn get_circuit_data(&self) -> &CircuitData<F, C, D> {
        self.circuit_data.last().unwrap()
    }
}

fn bench_recursive_proof<
    F: RichField + Extendable<D>,
    const D: usize,
    C: GenericConfig<D, F = F>,
    InnerC: GenericConfig<D, F = F>,
>(
    c: &mut Criterion,
) where
    InnerC::Hasher: AlgebraicHasher<F, InnerC::HCO>,
    [(); InnerC::HCO::WIDTH]:,
    [(); InnerC::HCI::WIDTH]:,
    C::Hasher: AlgebraicHasher<F, C::HCO>,
    [(); C::HCO::WIDTH]:,
    [(); C::HCI::WIDTH]:,
{
    let mut group = c.benchmark_group(&format!(
        "recursive-proof<{}, {}>",
        type_name::<C>(),
        type_name::<InnerC>()
    ));

    let config = CircuitConfig::standard_recursion_config();

    for degree in [13, 15] {
        let base_circuit =
            BaseCircuit::<F, InnerC, D, InnerC::HCO, InnerC::Hasher>::build_base_circuit(
                config.clone(),
                degree,
            );

        assert_eq!(base_circuit.get_circuit_data().common.degree_bits(), degree);

        let proof = base_circuit.generate_base_proof(F::rand()).unwrap();

        let inner_cd = &base_circuit.get_circuit_data().common;

        group.bench_function(
            format!("build circuit for degree {}", degree).as_str(),
            |b| {
                b.iter_with_large_drop(|| {
                    ShrinkCircuit::<F, C, InnerC, D>::build_shrink_circuit(
                        inner_cd,
                        config.clone(),
                    );
                })
            },
        );

        let shrink_circuit =
            ShrinkCircuit::<F, C, InnerC, D>::build_shrink_circuit(inner_cd, config.clone());

        pretty_print!("shrink steps: {}", shrink_circuit.num_shrink_steps());

        let inner_cd = base_circuit.get_circuit_data();

        group.bench_function(
            format!("shrinking proof of degree {}", degree).as_str(),
            |b| {
                b.iter_batched(
                    || proof.clone(),
                    |proof| shrink_circuit.shrink_proof(proof, inner_cd).unwrap(),
                    BatchSize::PerIteration,
                )
            },
        );

        let shrunk_proof = shrink_circuit.shrink_proof(proof, inner_cd).unwrap();
        let shrunk_cd = shrink_circuit.get_circuit_data();

        assert_eq!(shrunk_cd.common.degree_bits(), RECURSION_THRESHOLD);

        //let proof_bytes = serde_cbor::to_vec(&shrunk_proof).unwrap();
        //fancy_print!("Proof length: {} bytes for {} gates", proof_bytes.len(), shrunk_cd.common.degree());

        group.bench_function(
            format!("verify proof for degree {}", degree).as_str(),
            |b| {
                b.iter_batched(
                    || shrunk_proof.clone(),
                    |proof| shrunk_cd.verify(proof).unwrap(),
                    BatchSize::PerIteration,
                )
            },
        );
    }

    group.finish();
}

fn benchmark(c: &mut Criterion) {
    const D: usize = 2;
    type F = GoldilocksField;
    bench_recursive_proof::<F, D, PoseidonGoldilocksConfig, PoseidonGoldilocksConfig>(c);
    bench_recursive_proof::<F, D, PoseidonGoldilocksConfig, Poseidon2GoldilocksConfig>(c);
    bench_recursive_proof::<F, D, Poseidon2GoldilocksConfig, PoseidonGoldilocksConfig>(c);
    bench_recursive_proof::<F, D, Poseidon2GoldilocksConfig, Poseidon2GoldilocksConfig>(c);
}

criterion_group!(name = benches;
    config = Criterion::default().sample_size(10);
    targets = benchmark);
criterion_main!(benches);
