use std::marker::PhantomData;

use plonky2::field::extension_field::{Extendable, FieldExtension};
use plonky2::field::packed_field::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};

/// Toy STARK system used for testing.
/// Computes a Factorial sequence with state `[fact, n]` using the state transition
/// `fact' <- fact * (n + 1), n' <- n + 1`.
#[derive(Copy, Clone)]
struct FactorialStark<F: RichField + Extendable<D>, const D: usize> {
    num_rows: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> FactorialStark<F, D> {
    // The first public input is `x0`.
    const PI_INDEX_X0: usize = 0;
    // The second public input is the first element of the last row, which should be equal to
    // `num_rows` factorial.
    const PI_INDEX_RES: usize = 1;

    fn new(num_rows: usize) -> Self {
        Self {
            num_rows,
            _phantom: PhantomData,
        }
    }

    /// Generate the trace using `x0, 1` as initial state values.
    fn generate_trace(&self, x0: F) -> Vec<PolynomialValues<F>> {
        let mut trace_rows = (0..self.num_rows)
            .scan([x0, F::ONE], |acc, _| {
                let tmp = *acc;
                acc[0] = tmp[0] * (tmp[1] + F::ONE);
                acc[1] = tmp[1] + F::ONE;
                Some(tmp)
            })
            .collect::<Vec<_>>();
        trace_rows_to_poly_values(trace_rows)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for FactorialStark<F, D> {
    const COLUMNS: usize = 2;
    const PUBLIC_INPUTS: usize = 2;

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
            .constraint_last_row(vars.local_values[0] - vars.public_inputs[Self::PI_INDEX_RES]);

        // x0' <- x0 * (x1 + 1)
        yield_constr.constraint_transition(
            vars.next_values[0] - vars.local_values[0] * (vars.local_values[1] + FE::ONE),
        );
        // x1' <- x1 + 1
        yield_constr.constraint_transition(vars.next_values[1] - vars.local_values[1] - FE::ONE);
    }

    fn eval_ext_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }, { Self::PUBLIC_INPUTS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        // Check public inputs.
        let pis_constraints = [
            builder.sub_extension(vars.local_values[0], vars.public_inputs[Self::PI_INDEX_X0]),
            builder.sub_extension(vars.local_values[0], vars.public_inputs[Self::PI_INDEX_RES]),
        ];
        yield_constr.constraint_first_row(builder, pis_constraints[0]);
        yield_constr.constraint_last_row(builder, pis_constraints[1]);

        let one = builder.one_extension();
        // x0' <- x0 * (x1 + 1)
        let first_col_constraint = {
            let tmp1 = builder.add_extension(vars.local_values[1], one);
            let tmp2 = builder.mul_extension(vars.local_values[0], tmp1);
            builder.sub_extension(vars.next_values[0], tmp2)
        };
        yield_constr.constraint_transition(builder, first_col_constraint);
        // x1' <- x1 + 1
        let second_col_constraint = {
            let tmp = builder.add_extension(vars.local_values[1], one);
            builder.sub_extension(vars.next_values[1], tmp)
        };
        yield_constr.constraint_transition(builder, second_col_constraint);
    }

    fn constraint_degree(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::field::extension_field::Extendable;
    use plonky2::field::field_types::Field;
    use plonky2::hash::hash_types::RichField;
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{
        AlgebraicHasher, GenericConfig, Hasher, PoseidonGoldilocksConfig,
    };
    use plonky2::util::timing::TimingTree;

    use crate::config::StarkConfig;
    use crate::factorial_stark::FactorialStark;
    use crate::proof::StarkProofWithPublicInputs;
    use crate::prover::prove;
    use crate::recursive_verifier::{
        add_virtual_stark_proof_with_pis, recursively_verify_stark_proof,
        set_stark_proof_with_pis_target,
    };
    use crate::stark::Stark;
    use crate::stark_testing::test_stark_low_degree;
    use crate::verifier::verify_stark_proof;

    fn factorial<F: Field>(n: usize, x0: F) -> F {
        (0..n)
            .fold((x0, F::ONE), |x, _| (x.0 * (x.1 + F::ONE), x.1 + F::ONE))
            .0
    }

    #[test]
    fn test_factorial_stark() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FactorialStark<F, D>;

        let config = StarkConfig::standard_fast_config();
        let num_rows = 1 << 3;
        let public_inputs = [F::ONE, factorial(num_rows - 1, F::ONE)];
        let stark = S::new(num_rows);
        let trace = stark.generate_trace(public_inputs[0]);
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
    fn test_factorial_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FactorialStark<F, D>;

        let num_rows = 1 << 3;
        let stark = S::new(num_rows);
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_recursive_stark_verifier() -> Result<()> {
        init_logger();
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = FactorialStark<F, D>;

        let config = StarkConfig::standard_fast_config();
        let num_rows = 1 << 5;
        let public_inputs = [F::ONE, factorial(num_rows - 1, F::ONE)];
        let stark = S::new(num_rows);
        let trace = stark.generate_trace(public_inputs[0]);
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
        [(); C::Hasher::HASH_SIZE]:,
    {
        let circuit_config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(circuit_config);
        let mut pw = PartialWitness::new();
        let degree_bits = inner_proof.proof.recover_degree_bits(inner_config);
        let pt = add_virtual_stark_proof_with_pis(&mut builder, stark, inner_config, degree_bits);
        set_stark_proof_with_pis_target(&mut pw, &pt, &inner_proof);

        recursively_verify_stark_proof::<F, InnerC, S, D>(&mut builder, stark, pt, inner_config);

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
