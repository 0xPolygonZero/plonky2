use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;

use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// Toy STARK system used for testing.
/// Computes a Fibonacci sequence with state `[x0, x1, i, j]` using the state transition
/// `x0' <- x1, x1' <- x0 + x1, i' <- i+1, j' <- j+1`.
/// Note: The `i, j` columns are only used to test the permutation argument.
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

    fn new(num_rows: usize) -> Self {
        Self {
            num_rows,
            _phantom: PhantomData,
        }
    }

    /// Generate the trace using `x0, x1, 0, 1` as initial state values.
    fn generate_trace(&self, x0: F, x1: F) -> Vec<PolynomialValues<F>> {
        let mut trace_rows = (0..self.num_rows)
            .scan([x0, x1, F::ZERO, F::ONE], |acc, _| {
                let tmp = *acc;
                acc[0] = tmp[1];
                acc[1] = tmp[0] + tmp[1];
                acc[2] = tmp[2] + F::ONE;
                acc[3] = tmp[3] + F::ONE;
                Some(tmp)
            })
            .collect::<Vec<_>>();
        trace_rows[self.num_rows - 1][3] = F::ZERO; // So that column 2 and 3 are permutation of one another.
        trace_rows_to_poly_values(trace_rows)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for FibonacciStark<F, D> {
    const COLUMNS: usize = 4;
    const PUBLIC_INPUTS: usize = 3;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        // Check public inputs.
        yield_constr
            .constraint_first_row(vars.local_values[0] - vars.public_inputs[Self::PI_INDEX_X0]);
        yield_constr
            .constraint_first_row(vars.local_values[1] - vars.public_inputs[Self::PI_INDEX_X1]);
        yield_constr
            .constraint_last_row(vars.local_values[1] - vars.public_inputs[Self::PI_INDEX_RES]);

        // x0' <- x1
        yield_constr.constraint_transition(vars.next_values[0] - vars.local_values[1]);
        // x1' <- x0 + x1
        yield_constr.constraint_transition(
            vars.next_values[1] - vars.local_values[0] - vars.local_values[1],
        );
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        // Check public inputs.
        let pis_constraints = [
            builder.sub_extension(vars.local_values[0], vars.public_inputs[Self::PI_INDEX_X0]),
            builder.sub_extension(vars.local_values[1], vars.public_inputs[Self::PI_INDEX_X1]),
            builder.sub_extension(vars.local_values[1], vars.public_inputs[Self::PI_INDEX_RES]),
        ];
        yield_constr.constraint_first_row(builder, pis_constraints[0]);
        yield_constr.constraint_first_row(builder, pis_constraints[1]);
        yield_constr.constraint_last_row(builder, pis_constraints[2]);

        // x0' <- x1
        let first_col_constraint = builder.sub_extension(vars.next_values[0], vars.local_values[1]);
        yield_constr.constraint_transition(builder, first_col_constraint);
        // x1' <- x0 + x1
        let second_col_constraint = {
            let tmp = builder.sub_extension(vars.next_values[1], vars.local_values[0]);
            builder.sub_extension(tmp, vars.local_values[1])
        };
        yield_constr.constraint_transition(builder, second_col_constraint);
    }

    fn constraint_degree(&self) -> usize {
        2
    }

    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        vec![PermutationPair::singletons(2, 3)]
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
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
            public_inputs,
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
        init_logger();
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
            public_inputs,
            &mut TimingTree::default(),
        )?;
        verify_stark_proof(stark, proof.clone(), &config)?;

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
        [(); S::COLUMNS]:,
        [(); S::PUBLIC_INPUTS]:,
    {
        let circuit_config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(circuit_config);
        let mut pw = PartialWitness::new();
        let degree_bits = inner_proof.proof.recover_degree_bits(inner_config);
        let pt = add_virtual_stark_proof_with_pis(&mut builder, stark, inner_config, degree_bits);
        set_stark_proof_with_pis_target(&mut pw, &pt, &inner_proof);

        verify_stark_proof_circuit::<F, InnerC, S, D>(&mut builder, stark, pt, inner_config);

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
}
