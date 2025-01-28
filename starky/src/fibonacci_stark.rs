//! An example of generating and verifying STARK proofs for the Fibonacci sequence.
//! The toy STARK system also includes two columns that are a permutation of the other,
//! to highlight the use of the permutation argument with logUp.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
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
    type EvaluationFrame<FE, P, const D2: usize>
        = StarkFrame<P, P::Scalar, FIBONACCI_COLUMNS, FIBONACCI_PUBLIC_INPUTS>
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
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    use anyhow::Result;
    use itertools::Itertools;
    use plonky2::field::extension::Extendable;
    use plonky2::field::types::Field;
    use plonky2::hash::hash_types::RichField;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig};
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

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;
    type S = FibonacciStark<F, D>;

    fn fibonacci<F: Field>(n: usize, x0: F, x1: F) -> F {
        (0..n).fold((x0, x1), |x, _| (x.1, x.0 + x.1)).1
    }

    #[test]
    fn test_fibonacci_stark() -> Result<()> {
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
            None,
            &mut TimingTree::default(),
        )?;

        verify_stark_proof(stark, proof, &config, None)
    }

    #[test]
    fn test_fibonacci_stark_degree() -> Result<()> {
        let num_rows = 1 << 5;
        let stark = S::new(num_rows);
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_fibonacci_stark_circuit() -> Result<()> {
        let num_rows = 1 << 5;
        let stark = S::new(num_rows);
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }

    #[test]
    fn test_recursive_stark_verifier() -> Result<()> {
        init_logger();

        let config = StarkConfig::standard_fast_config();
        let degree_bits = 5;
        let num_rows = 1 << degree_bits;
        let public_inputs = [F::ZERO, F::ONE, fibonacci(num_rows - 1, F::ZERO, F::ONE)];

        // Test first STARK
        let stark = S::new(num_rows);
        let trace = stark.generate_trace(public_inputs[0], public_inputs[1]);
        let proof = prove::<F, C, S, D>(
            stark,
            &config,
            trace,
            &public_inputs,
            None,
            &mut TimingTree::default(),
        )?;
        verify_stark_proof(stark, proof.clone(), &config, None)?;
        assert_eq!(degree_bits, proof.proof.recover_degree_bits(&config));

        recursive_proof::<F, C, S, C, D>(stark, proof, &config, true)
    }

    fn recursive_proof<
        F: RichField + Extendable<D>,
        C: GenericConfig<D, F = F>,
        S: Stark<F, D> + Copy,
        InnerC: GenericConfig<D, F = F>,
        const D: usize,
    >(
        stark: S,
        inner_proof: StarkProofWithPublicInputs<F, InnerC, D>,
        inner_config: &StarkConfig,
        print_gate_counts: bool,
    ) -> Result<()>
    where
        InnerC::Hasher: AlgebraicHasher<F>,
    {
        let circuit_config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(circuit_config);
        let mut pw = PartialWitness::new();
        let degree_bits = inner_proof.proof.recover_degree_bits(inner_config);
        let pt =
            add_virtual_stark_proof_with_pis(&mut builder, &stark, inner_config, degree_bits, 0, 0);
        set_stark_proof_with_pis_target(&mut pw, &pt, &inner_proof, degree_bits, builder.zero())?;

        verify_stark_proof_circuit::<F, InnerC, S, D>(&mut builder, stark, pt, inner_config, None);

        if print_gate_counts {
            builder.print_gate_counts(0);
        }

        let data = builder.build::<C>();
        let proof = data.prove(pw)?;
        data.verify(proof)
    }

    fn init_logger() {
        let _ = env_logger::builder().format_timestamp(None).try_init();
    }

    #[test]
    fn test_recursive_verifier_with_multiple_degree_bits() -> Result<()> {
        init_logger();

        let mut stark_config = StarkConfig::standard_fast_config();
        stark_config.fri_config.num_query_rounds = 1;

        let min_degree_bits_to_support = 4;
        // Currently, we only support verifier_degree_bits to be {30, 26, 22, 18, …}, as they
        // generate the max final polynomial length when using the default configuration
        // ConstantArityBits(4, 5). This ensures that for other degrees, the final proof polynomial
        // will not be longer than the circuit’s final polynomial length.
        let verifier_degree_bits = 30;
        let degree_bits = 4..=15;
        let verifier_fri_params = stark_config.fri_params(verifier_degree_bits);

        // Generate STARK proofs for each degree in `degree_bits`
        let proofs: Vec<_> = degree_bits
            .clone()
            .map(|degree_bits| {
                let num_rows = 1 << degree_bits;
                let public_inputs = [F::ZERO, F::ONE, fibonacci(num_rows - 1, F::ZERO, F::ONE)];
                let stark = S::new(num_rows);
                let trace = stark.generate_trace(public_inputs[0], public_inputs[1]);

                // Generate proof with the specified verifier degree
                prove::<F, C, S, D>(
                    stark,
                    &stark_config,
                    trace,
                    &public_inputs,
                    Some(verifier_fri_params.clone()),
                    &mut TimingTree::default(),
                )
                .unwrap()
            })
            .collect();

        // Configure the circuit for recursive verification
        let num_rows = 1 << verifier_degree_bits;
        let stark = S::new(num_rows);
        for p in proofs.clone() {
            verify_stark_proof(stark, p, &stark_config, Some(verifier_fri_params.clone()))?;
        }

        let recursive_verification_circuit_config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(recursive_verification_circuit_config);
        let zero = builder.zero();

        // Set up proof verification within the circuit
        let pt = add_virtual_stark_proof_with_pis(
            &mut builder,
            &stark,
            &stark_config,
            verifier_degree_bits,
            0,
            0,
        );
        verify_stark_proof_circuit::<F, C, S, D>(
            &mut builder,
            stark,
            pt.clone(),
            &stark_config,
            Some(min_degree_bits_to_support),
        );
        builder.print_gate_counts(0);

        // Build the recursive circuit
        let data = builder.build::<C>();

        // Verify each proof using partial witnesses
        degree_bits
            .zip_eq(proofs)
            .try_for_each(|(degree_bits, proof)| {
                let mut pw = PartialWitness::new();
                set_stark_proof_with_pis_target(&mut pw, &pt, &proof, degree_bits, zero)?;
                let proof = data.prove(pw)?;
                data.verify(proof)
            })?;

        Ok(())
    }
}
