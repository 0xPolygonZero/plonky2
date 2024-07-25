//! An example of generating and verifying STARK proofs for the Fibonacci sequence.
//! The toy STARK system also includes two columns that are a permutation of the other,
//! to highlight the use of the permutation argument with logUp.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use core::marker::PhantomData;

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;

/// Toy STARK system used for testing.
/// Computes a Fibonacci sequence with state `[x0, x1]` using the state transition
/// `x0' <- x1, x1' <- x0 + x1.
#[derive(Copy, Clone)]
struct FibonacciStark<F: RichField + Extendable<D>, const D: usize> {
    num_rows: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> FibonacciStark<F, D> {
    // The first public input is `x0`.
    const PI_INDEX_X0: usize = 0;
    // The second public input is `x1`.
    const PI_INDEX_X1: usize = 1;
    // The third public input is the second element of the last row, which should be equal to the
    // `num_rows`-th Fibonacci number.
    const PI_INDEX_RES: usize = 2;

    const fn new(num_rows: usize) -> Self {
        Self {
            num_rows,
            _phantom: PhantomData,
        }
    }

    /// Generate the trace using `x0, x1` as initial state values.
    fn generate_trace(&self, x0: F, x1: F) -> Vec<PolynomialValues<F>> {
        let trace_rows = (0..self.num_rows)
            .scan([x0, x1], |acc, _| {
                let tmp = *acc;
                acc[0] = tmp[1];
                acc[1] = tmp[0] + tmp[1];
                Some(tmp)
            })
            .collect::<Vec<_>>();
        trace_rows_to_poly_values(trace_rows)
    }
}

const FIBONACCI_COLUMNS: usize = 2;
const FIBONACCI_PUBLIC_INPUTS: usize = 3;

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for FibonacciStark<F, D> {
    type EvaluationFrame<FE, P, const D2: usize> = StarkFrame<P, P::Scalar, FIBONACCI_COLUMNS, FIBONACCI_PUBLIC_INPUTS>
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    type EvaluationFrameTarget = StarkFrame<
        ExtensionTarget<D>,
        ExtensionTarget<D>,
        FIBONACCI_COLUMNS,
        FIBONACCI_PUBLIC_INPUTS,
    >;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: &Self::EvaluationFrame<FE, P, D2>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let local_values = vars.get_local_values();
        let next_values = vars.get_next_values();
        let public_inputs = vars.get_public_inputs();

        // Check public inputs.
        yield_constr.constraint_first_row(local_values[0] - public_inputs[Self::PI_INDEX_X0]);
        yield_constr.constraint_first_row(local_values[1] - public_inputs[Self::PI_INDEX_X1]);
        yield_constr.constraint_last_row(local_values[1] - public_inputs[Self::PI_INDEX_RES]);

        // x0' <- x1
        yield_constr.constraint_transition(next_values[0] - local_values[1]);
        // x1' <- x0 + x1
        yield_constr.constraint_transition(next_values[1] - local_values[0] - local_values[1]);
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let local_values = vars.get_local_values();
        let next_values = vars.get_next_values();
        let public_inputs = vars.get_public_inputs();
        // Check public inputs.
        let pis_constraints = [
            builder.sub_extension(local_values[0], public_inputs[Self::PI_INDEX_X0]),
            builder.sub_extension(local_values[1], public_inputs[Self::PI_INDEX_X1]),
            builder.sub_extension(local_values[1], public_inputs[Self::PI_INDEX_RES]),
        ];
        yield_constr.constraint_first_row(builder, pis_constraints[0]);
        yield_constr.constraint_first_row(builder, pis_constraints[1]);
        yield_constr.constraint_last_row(builder, pis_constraints[2]);

        // x0' <- x1
        let first_col_constraint = builder.sub_extension(next_values[0], local_values[1]);
        yield_constr.constraint_transition(builder, first_col_constraint);
        // x1' <- x0 + x1
        let second_col_constraint = {
            let tmp = builder.sub_extension(next_values[1], local_values[0]);
            builder.sub_extension(tmp, local_values[1])
        };
        yield_constr.constraint_transition(builder, second_col_constraint);
    }

    fn constraint_degree(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use anyhow::Result;
    use log::Level;
    use plonky2::field::extension::Extendable;
    use plonky2::field::types::Field;
    use plonky2::gates::noop::NoopGate;
    use plonky2::hash::hash_types::RichField;
    use plonky2::iop::witness::{PartialWitness, WitnessWrite};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::{CircuitConfig, CommonCircuitData, VerifierOnlyCircuitData};
    use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::plonk::proof::{ProofWithPublicInputs};
    use plonky2::plonk::wrapper::plonky2_config::PoseidonBN128GoldilocksConfig;
    use plonky2::util::timing::TimingTree;

    use crate::config::StarkConfig;
    use crate::fibonacci_stark::FibonacciStark;
    use crate::proof::StarkProofWithPublicInputs;
    use crate::prover::prove;
    use crate::recursive_verifier::{
        add_virtual_stark_proof_with_pis, set_stark_proof_with_pis_target,
        verify_stark_proof_circuit,
    };
    use crate::stark::Stark;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};
    use crate::verifier::verify_stark_proof;

    fn fibonacci<F: Field>(n: usize, x0: F, x1: F) -> F {
        (0..n).fold((x0, x1), |x, _| (x.1, x.0 + x.1)).1
    }

    #[test]
    fn test_fibonacci_stark() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FibonacciStark<F, D>;

        let config = StarkConfig::standard_fast_config();
        let num_rows = 1 << 5;
        let public_inputs = [F::ZERO, F::ONE, fibonacci(num_rows - 1, F::ZERO, F::ONE)];

        let stark = S::new(num_rows);
        let trace = stark.generate_trace(public_inputs[0], public_inputs[1]);
        let proof = prove::<F, C, S, D>(
            stark,
            &config,
            trace,
            &public_inputs,
            &mut TimingTree::default(),
        )?;

        verify_stark_proof(stark, proof, &config)
    }

    #[test]
    fn test_fibonacci_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FibonacciStark<F, D>;

        let num_rows = 1 << 5;
        let stark = S::new(num_rows);
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_fibonacci_stark_circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FibonacciStark<F, D>;

        let num_rows = 1 << 5;
        let stark = S::new(num_rows);
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }

    #[test]
    fn test_recursive_stark_verifier() -> Result<()> {
        // starky proof
        // plonky2 recursive proof
        // plonky2 wrapper proof (to GoldilockBn128Poseidon)
        init_logger();
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;

        type C2 = PoseidonBN128GoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FibonacciStark<F, D>;

        let config = StarkConfig::standard_fast_config();
        let num_rows = 1 << 5;
        let public_inputs = [F::ZERO, F::ONE, fibonacci(num_rows - 1, F::ZERO, F::ONE)];

        // Test first STARK
        let stark = S::new(num_rows);
        let trace = stark.generate_trace(public_inputs[0], public_inputs[1]);
        let proof = prove::<F, C, S, D>(
            stark,
            &config,
            trace,
            &public_inputs,
            &mut TimingTree::default(),
        )?;
        verify_stark_proof(stark, proof.clone(), &config)?;

        let (r_p, r_v, r_c) = recursive_starky_proof::<F, C, S, C, D>(stark, proof, &config)?;
        // this recursive proof will be wrapped

        let (w_p, w_v, w_c) = recursive_plonky2_proof::<F, C2, C, D>(r_p, r_v, r_c)?;

        let common_data_file = File::create("common_circuit_data.json")?;
        serde_json::to_writer_pretty(&common_data_file, &w_c)?;
        println!("Succesfully wrote common circuit data to common_circuit_data.json");

        let verifier_data_file = File::create("verifier_only_circuit_data.json")?;
        serde_json::to_writer_pretty(&verifier_data_file, &w_v)?;
        println!("Succesfully wrote verifier data to verifier_only_circuit_data.json");

        let proof_file = File::create("proof_with_public_inputs.json")?;
        serde_json::to_writer_pretty(&proof_file, &w_p)?;
        println!("Succesfully wrote proof to proof_with_public_inputs.json");

        Ok(())
    }

    fn recursive_starky_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F> + serde::Serialize,
        S: Stark<F, D> + Copy,
        InnerC: GenericConfig<D, F = F>,
        const D: usize,
    >(
        stark: S,
        inner_proof: StarkProofWithPublicInputs<F, InnerC, D>,
        inner_config: &StarkConfig,
    ) -> Result<Proof<F, C, D>>
    where
        InnerC::Hasher: AlgebraicHasher<F>,
    {
        println!("start recursive stakry in plonky2");
        let circuit_config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(circuit_config);
        let mut pw = PartialWitness::new();
        let degree_bits = inner_proof.proof.recover_degree_bits(inner_config);
        let pt =
            add_virtual_stark_proof_with_pis(&mut builder, &stark, inner_config, degree_bits, 0, 0);
        set_stark_proof_with_pis_target(&mut pw, &pt, &inner_proof, builder.zero());

        verify_stark_proof_circuit::<F, InnerC, S, D>(&mut builder, stark, pt, inner_config);

        let data = builder.build::<C>();
        let proof = data.prove(pw)?;

        let res_proof = proof.clone();
        data.verify(proof)?;
        println!("recursive plonky2 degree: {}", data.common.degree_bits());
        Ok((res_proof, data.verifier_only, data.common))
    }

    type Proof<F, C, const D: usize> = (
        ProofWithPublicInputs<F, C, D>,
        VerifierOnlyCircuitData<C, D>,
        CommonCircuitData<F, D>,
    );

    fn recursive_plonky2_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        InnerC: GenericConfig<D, F = F>,
        const D: usize,
    >(
        inner_proof: ProofWithPublicInputs<F, InnerC, D>,
        inner_vd: VerifierOnlyCircuitData<InnerC, D>,
        inner_cd: CommonCircuitData<F, D>,
    ) -> Result<Proof<F, C, D>>
    where
        InnerC::Hasher: AlgebraicHasher<F>,
    {
        let circuit_config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(circuit_config.clone());
        let mut pw = PartialWitness::new();
        let pt = builder.add_virtual_proof_with_pis(&inner_cd);
        pw.set_proof_with_pis_target(&pt, &inner_proof);

        let inner_data = builder.add_virtual_verifier_data(inner_cd.config.fri_config.cap_height);
        pw.set_cap_target(
            &inner_data.constants_sigmas_cap,
            &inner_vd.constants_sigmas_cap,
        );
        pw.set_hash_target(inner_data.circuit_digest, inner_vd.circuit_digest);

        builder.verify_proof::<InnerC>(&pt, &inner_data, &inner_cd);

        let data = builder.build::<C>();

        let mut timing = TimingTree::new("prove", Level::Debug);
        let proof = plonky2::plonk::prover::prove(&data.prover_only, &data.common, pw, &mut timing)?;

        println!("plonky2 wrapper degrees: {}", data.common.degree_bits());

        data.verify(proof.clone())?;

        Ok((proof, data.verifier_only, data.common))
    }

    fn init_logger() {
        let _ = env_logger::builder().format_timestamp(None).try_init();
    }
}
